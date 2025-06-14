use crate::cookies::AddCookieParametersWrapper;
use crate::error::ErrorStatus;
use crate::wd::{self, WebDriverCompatibleCommand};
use crate::{error, Client};
use base64::Engine;
use futures_util::future::{self, Either};
use futures_util::{FutureExt, TryFutureExt};
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper_util::client::legacy::connect;
use hyper_util::rt::TokioExecutor;
use serde_json::Value as Json;
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::mem;
use std::pin::Pin;
use std::task::Context;
use std::task::{ready, Poll};
use tokio::sync::{mpsc, oneshot};
use webdriver::command::WebDriverCommand;
use webdriver::response::NewSessionResponse;

type Ack = oneshot::Sender<Result<Json, error::CmdError>>;

type Wcmd = WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum Cmd {
    SetUa(String),
    GetSessionId,
    Shutdown,
    Persist,
    GetUa,
    Raw {
        req: hyper::Request<http_body_util::combinators::BoxBody<hyper::body::Bytes, Infallible>>,
        rsp: oneshot::Sender<
            Result<hyper::Response<hyper::body::Incoming>, hyper_util::client::legacy::Error>,
        >,
    },
    WebDriver(Box<dyn WebDriverCompatibleCommand + Send>),
}

impl WebDriverCompatibleCommand for Wcmd {
    /// Helper for determining what URL endpoint to use for various requests.
    ///
    /// This mapping is essentially that of <https://www.w3.org/TR/webdriver/#list-of-endpoints>.
    fn endpoint(
        &self,
        base_url: &url::Url,
        session_id: Option<&str>,
    ) -> Result<url::Url, url::ParseError> {
        if let WebDriverCommand::NewSession(..) = self {
            return base_url.join("session");
        }

        if let WebDriverCommand::Status = self {
            return base_url.join("status");
        }

        let base = { base_url.join(&format!("session/{}/", session_id.as_ref().unwrap()))? };
        match self {
            WebDriverCommand::NewSession(..) => unreachable!(),
            WebDriverCommand::DeleteSession => unreachable!(),
            WebDriverCommand::Get(..) | WebDriverCommand::GetCurrentUrl => base.join("url"),
            WebDriverCommand::GoBack => base.join("back"),
            WebDriverCommand::GoForward => base.join("forward"),
            WebDriverCommand::Refresh => base.join("refresh"),
            WebDriverCommand::GetTitle => base.join("title"),
            WebDriverCommand::GetPageSource => base.join("source"),
            WebDriverCommand::GetWindowHandle => base.join("window"),
            WebDriverCommand::GetWindowHandles => base.join("window/handles"),
            WebDriverCommand::NewWindow(..) => base.join("window/new"),
            WebDriverCommand::CloseWindow => base.join("window"),
            WebDriverCommand::GetWindowRect => base.join("window/rect"),
            WebDriverCommand::SetWindowRect(..) => base.join("window/rect"),
            WebDriverCommand::MinimizeWindow => base.join("window/minimize"),
            WebDriverCommand::MaximizeWindow => base.join("window/maximize"),
            WebDriverCommand::FullscreenWindow => base.join("window/fullscreen"),
            WebDriverCommand::SwitchToWindow(..) => base.join("window"),
            WebDriverCommand::SwitchToFrame(_) => base.join("frame"),
            WebDriverCommand::SwitchToParentFrame => base.join("frame/parent"),
            WebDriverCommand::FindElement(..) => base.join("element"),
            WebDriverCommand::FindElements(..) => base.join("elements"),
            WebDriverCommand::FindElementElement(ref p, _) => {
                base.join(&format!("element/{}/element", p.0))
            }
            WebDriverCommand::FindElementElements(ref p, _) => {
                base.join(&format!("element/{}/elements", p.0))
            }
            WebDriverCommand::GetActiveElement => base.join("element/active"),
            WebDriverCommand::IsDisplayed(ref we) => {
                base.join(&format!("element/{}/displayed", we.0))
            }
            WebDriverCommand::IsSelected(ref we) => {
                base.join(&format!("element/{}/selected", we.0))
            }
            WebDriverCommand::GetElementAttribute(ref we, ref attr) => {
                base.join(&format!("element/{}/attribute/{}", we.0, attr))
            }
            WebDriverCommand::GetElementProperty(ref we, ref prop) => {
                base.join(&format!("element/{}/property/{}", we.0, prop))
            }
            WebDriverCommand::GetCSSValue(ref we, ref attr) => {
                base.join(&format!("element/{}/css/{}", we.0, attr))
            }
            WebDriverCommand::GetElementText(ref we) => {
                base.join(&format!("element/{}/text", we.0))
            }
            WebDriverCommand::GetElementTagName(ref we) => {
                base.join(&format!("element/{}/name", we.0))
            }
            WebDriverCommand::GetElementRect(ref we) => {
                base.join(&format!("element/{}/rect", we.0))
            }
            WebDriverCommand::IsEnabled(ref we) => base.join(&format!("element/{}/enabled", we.0)),
            WebDriverCommand::ExecuteScript(..) => base.join("execute/sync"),
            WebDriverCommand::ExecuteAsyncScript(..) => base.join("execute/async"),
            WebDriverCommand::GetCookies
            | WebDriverCommand::AddCookie(_)
            | WebDriverCommand::DeleteCookies => base.join("cookie"),
            WebDriverCommand::GetNamedCookie(ref name)
            | WebDriverCommand::DeleteCookie(ref name) => base.join(&format!("cookie/{}", name)),
            WebDriverCommand::GetTimeouts | WebDriverCommand::SetTimeouts(..) => {
                base.join("timeouts")
            }
            WebDriverCommand::ElementClick(ref we) => base.join(&format!("element/{}/click", we.0)),
            WebDriverCommand::ElementClear(ref we) => base.join(&format!("element/{}/clear", we.0)),
            WebDriverCommand::ElementSendKeys(ref we, _) => {
                base.join(&format!("element/{}/value", we.0))
            }
            WebDriverCommand::PerformActions(..) | WebDriverCommand::ReleaseActions => {
                base.join("actions")
            }
            WebDriverCommand::DismissAlert => base.join("alert/dismiss"),
            WebDriverCommand::AcceptAlert => base.join("alert/accept"),
            WebDriverCommand::GetAlertText | WebDriverCommand::SendAlertText(..) => {
                base.join("alert/text")
            }
            WebDriverCommand::TakeScreenshot => base.join("screenshot"),
            WebDriverCommand::TakeElementScreenshot(ref we) => {
                base.join(&format!("element/{}/screenshot", we.0))
            }
            WebDriverCommand::Print(..) => base.join("print"),
            WebDriverCommand::Status => unreachable!(),
            _ => unimplemented!(),
        }
    }

    fn method_and_body(&self, request_url: &url::Url) -> (http::Method, Option<String>) {
        use http::Method;
        use webdriver::command;

        // Most actions are just GET requests with no parameters
        let mut method = Method::GET;
        let mut body = None;

        // but some have a request body
        match self {
            WebDriverCommand::NewSession(command::NewSessionParameters {
                capabilities: ref conf,
            }) => {
                let mut capabilities = serde_json::value::Map::new();
                capabilities.insert(
                    String::from("capabilities"),
                    serde_json::to_value(conf)
                        .expect("SpecNewSessionParameters is always valid JSON"),
                );
                if !request_url.username().is_empty() {
                    capabilities.insert(
                        String::from("user"),
                        serde_json::to_value(request_url.username())
                            .expect("all strings are valid JSON"),
                    );
                }
                if let Some(pwd) = request_url.password() {
                    capabilities.insert(
                        String::from("user"),
                        serde_json::to_value(pwd).expect("all strings are valid JSON"),
                    );
                }
                body = Some(
                    serde_json::to_string(&serde_json::Value::Object(capabilities))
                        .expect("a serde_json::Value can always be turned into JSON"),
                );

                method = Method::POST;
            }
            WebDriverCommand::Get(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::FindElement(ref loc)
            | WebDriverCommand::FindElements(ref loc)
            | WebDriverCommand::FindElementElement(_, ref loc)
            | WebDriverCommand::FindElementElements(_, ref loc) => {
                body = Some(serde_json::to_string(loc).unwrap());
                method = Method::POST;
            }

            WebDriverCommand::ExecuteScript(ref script) => {
                body = Some(serde_json::to_string(script).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::ExecuteAsyncScript(ref script) => {
                body = Some(serde_json::to_string(script).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::ElementSendKeys(_, ref keys) => {
                body = Some(serde_json::to_string(keys).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::ElementClick(..)
            | WebDriverCommand::ElementClear(..)
            | WebDriverCommand::GoBack
            | WebDriverCommand::GoForward
            | WebDriverCommand::Refresh
            | WebDriverCommand::MinimizeWindow
            | WebDriverCommand::MaximizeWindow
            | WebDriverCommand::FullscreenWindow
            | WebDriverCommand::DismissAlert
            | WebDriverCommand::AcceptAlert => {
                body = Some("{}".to_string());
                method = Method::POST;
            }
            WebDriverCommand::NewWindow(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::CloseWindow => {
                method = Method::DELETE;
            }
            WebDriverCommand::SetWindowRect(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::SwitchToWindow(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::SwitchToFrame(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::SwitchToParentFrame => {
                body = Some("{}".to_string());
                method = Method::POST;
            }
            WebDriverCommand::AddCookie(ref params) => {
                let wrapper = AddCookieParametersWrapper { cookie: params };
                body = Some(serde_json::to_string(&wrapper).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::DeleteCookie(_) | WebDriverCommand::DeleteCookies => {
                method = Method::DELETE;
            }
            WebDriverCommand::SetTimeouts(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::PerformActions(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::ReleaseActions => {
                method = Method::DELETE;
            }
            WebDriverCommand::SendAlertText(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::Print(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            _ => {}
        }
        (method, body)
    }

    fn is_new_session(&self) -> bool {
        matches!(self, WebDriverCommand::NewSession(..))
    }
}

impl From<Wcmd> for Cmd {
    fn from(o: Wcmd) -> Self {
        Cmd::WebDriver(Box::new(o))
    }
}

#[derive(Debug)]
pub(crate) struct Task {
    request: Cmd,
    ack: Ack,
}

impl Client {
    pub(crate) async fn issue<C>(&self, cmd: C) -> Result<Json, error::CmdError>
    where
        C: Into<Cmd>,
    {
        let (tx, rx) = oneshot::channel();
        let cmd = cmd.into();
        let r = self.tx.send(Task {
            request: cmd,
            ack: tx,
        });

        if r.is_err() {
            return Err(error::CmdError::Lost(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WebDriver session has been closed",
            )));
        }

        let r = rx.await;
        r.unwrap_or_else(|_| {
            Err(error::CmdError::Lost(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "WebDriver session was closed while waiting",
            )))
        })
    }

    /// Issue the specified [`WebDriverCompatibleCommand`] to the WebDriver instance.
    pub async fn issue_cmd(
        &self,
        cmd: impl WebDriverCompatibleCommand + Send + 'static,
    ) -> Result<Json, error::CmdError> {
        self.issue(Cmd::WebDriver(Box::new(cmd))).await
    }
}

enum Ongoing {
    None,
    Break,
    Shutdown {
        ack: Option<Ack>,
        fut: hyper_util::client::legacy::ResponseFuture,
    },
    WebDriver {
        ack: Ack,
        fut: Pin<Box<dyn Future<Output = Result<Json, error::CmdError>> + Send>>,
    },
    Raw {
        ack: Ack,
        ret: oneshot::Sender<
            Result<hyper::Response<hyper::body::Incoming>, hyper_util::client::legacy::Error>,
        >,
        fut: hyper_util::client::legacy::ResponseFuture,
    },
}

enum OngoingResult {
    Continue,
    Break,
    SessionId(String),
}

impl Ongoing {
    fn is_some(&self) -> bool {
        !matches!(self, Ongoing::None)
    }

    // returns true if outer loop should break
    fn poll(&mut self, try_extract_session: bool, cx: &mut Context<'_>) -> Poll<OngoingResult> {
        let rt = match mem::replace(self, Ongoing::None) {
            Ongoing::None => OngoingResult::Continue,
            Ongoing::Break => OngoingResult::Break,
            Ongoing::Shutdown { mut fut, ack } => {
                if Pin::new(&mut fut).poll(cx).is_pending() {
                    *self = Ongoing::Shutdown { fut, ack };
                    return Poll::Pending;
                }

                if let Some(ack) = ack {
                    let _ = ack.send(Ok(Json::Null));
                }
                OngoingResult::Break
            }
            Ongoing::WebDriver { mut fut, ack } => {
                let rsp = if let Poll::Ready(v) = fut.as_mut().poll(cx) {
                    v
                } else {
                    *self = Ongoing::WebDriver { fut, ack };
                    return Poll::Pending;
                };
                let mut rt = OngoingResult::Continue;
                if try_extract_session {
                    // we can safely assume that this supposed to be a response to NewSession
                    // pick out the session id, because we'll need it later
                    if let Ok(Json::Object(ref v)) = rsp {
                        // TODO: not all impls are w3c compatible
                        // See https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L189
                        // and https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L200
                        if let Some(session_id) = v.get("sessionId") {
                            if let Some(session_id) = session_id.as_str() {
                                rt = OngoingResult::SessionId(session_id.to_string());
                            }
                        }
                    }
                }

                let _ = ack.send(rsp);
                rt
            }
            Ongoing::Raw { mut fut, ack, ret } => {
                let rt = if let Poll::Ready(v) = Pin::new(&mut fut).poll(cx) {
                    v
                } else {
                    *self = Ongoing::Raw { fut, ack, ret };
                    return Poll::Pending;
                };
                let _ = ack.send(Ok(Json::Null));
                let _ = ret.send(rt);
                OngoingResult::Continue
            }
        };
        Poll::Ready(rt)
    }
}

pub(crate) struct Session<C>
where
    C: connect::Connect,
{
    ongoing: Ongoing,
    rx: mpsc::UnboundedReceiver<Task>,
    client: hyper_util::client::legacy::Client<
        C,
        http_body_util::combinators::BoxBody<hyper::body::Bytes, Infallible>,
    >,
    wdb: url::Url,
    session: Option<String>,
    ua: Option<String>,
    persist: bool,
}

impl<C> Future for Session<C>
where
    C: connect::Connect + Unpin + 'static + Clone + Sync + Send,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            if self.ongoing.is_some() {
                let has_session = self.session.is_none();
                match ready!(self.ongoing.poll(has_session, cx)) {
                    OngoingResult::Break => break,
                    OngoingResult::SessionId(sid) => {
                        self.session = Some(sid);
                    }
                    OngoingResult::Continue => {}
                }
            }

            // if we get here, there can be no ongoing request.
            // queue a new one.
            if let Some(Task { request, ack }) = ready!(Pin::new(&mut self.rx).poll_recv(cx)) {
                // some calls are just local housekeeping calls
                match request {
                    Cmd::GetSessionId => {
                        let _ = ack.send(Ok(self
                            .session
                            .clone()
                            .map(Json::String)
                            .unwrap_or(Json::Null)));
                    }
                    Cmd::SetUa(ua) => {
                        self.ua = Some(ua);
                        let _ = ack.send(Ok(Json::Null));
                    }
                    Cmd::GetUa => {
                        let _ =
                            ack.send(Ok(self.ua.clone().map(Json::String).unwrap_or(Json::Null)));
                    }
                    Cmd::Raw { req, rsp } => {
                        self.ongoing = Ongoing::Raw {
                            ack,
                            ret: rsp,
                            fut: self.client.request(req),
                        };
                    }
                    Cmd::Persist => {
                        self.persist = true;
                        let _ = ack.send(Ok(Json::Null));
                    }
                    Cmd::Shutdown => {
                        // explicit client shutdown
                        self.shutdown(Some(ack));
                    }
                    Cmd::WebDriver(request) => {
                        self.ongoing = Ongoing::WebDriver {
                            ack,
                            fut: Box::pin(self.issue_wd_cmd(request)),
                        };
                    }
                };
            } else {
                // we're shutting down!
                if self.persist {
                    self.ongoing = Ongoing::Break;
                } else {
                    self.shutdown(None);
                }
            }
        }

        Poll::Ready(())
    }
}

impl<C> Session<C>
where
    C: connect::Connect + Unpin + 'static + Clone + Send + Sync,
{
    fn new(
        rx: mpsc::UnboundedReceiver<Task>,
        client: hyper_util::client::legacy::Client<C, BoxBody<hyper::body::Bytes, Infallible>>,
        wdb_url: url::Url,
        session_id: Option<impl Into<String>>,
    ) -> Self {
        Session {
            ongoing: Ongoing::None,
            rx,
            client,
            wdb: wdb_url,
            session: session_id.map(Into::into),
            ua: None,
            persist: false,
        }
    }

    fn shutdown(&mut self, ack: Option<Ack>) {
        // session was not created
        if self.session.is_none() {
            self.ongoing = Ongoing::Break;
            return;
        }

        let url = {
            self.wdb
                .join(&format!("session/{}", self.session.as_ref().unwrap()))
                .unwrap()
        };

        self.ongoing = Ongoing::Shutdown {
            ack,
            fut: self.client.request(
                hyper::Request::delete(url.as_str())
                    .body(BoxBody::new(http_body_util::Empty::new()))
                    .unwrap(),
            ),
        };
    }

    fn map_handshake_response(
        response: Result<Json, error::CmdError>,
    ) -> Result<NewSessionResponse, error::NewSessionError> {
        match response {
            Ok(Json::Object(v)) => {
                // https://w3c.github.io/webdriver/#dfn-new-sessions
                // TODO: not all impls are w3c compatible
                // See https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L189
                // and https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L200
                if let (Some(Json::String(session_id)), Some(capabilities)) =
                    (v.get("sessionId"), v.get("capabilities"))
                {
                    if capabilities.is_object() {
                        return Ok(NewSessionResponse {
                            session_id: session_id.to_owned(),
                            capabilities: capabilities.to_owned(),
                        });
                    }
                }
                Err(error::NewSessionError::NotW3C(Json::Object(v)))
            }
            Ok(v) | Err(error::CmdError::NotW3C(v)) => Err(error::NewSessionError::NotW3C(v)),
            Err(error::CmdError::Failed(e)) => Err(error::NewSessionError::Failed(e)),
            Err(error::CmdError::FailedC(e)) => Err(error::NewSessionError::FailedC(e)),
            Err(error::CmdError::Lost(e)) => Err(error::NewSessionError::Lost(e)),
            Err(error::CmdError::NotJson(v)) => {
                Err(error::NewSessionError::NotW3C(Json::String(v)))
            }
            Err(error::CmdError::Standard(
                e @ error::WebDriver {
                    error: ErrorStatus::SessionNotCreated,
                    ..
                },
            )) => Err(error::NewSessionError::SessionNotCreated(e)),
            Err(error::CmdError::Standard(
                e @ error::WebDriver {
                    error: ErrorStatus::UnknownError,
                    ..
                },
            )) => Err(error::NewSessionError::NotW3C(
                serde_json::to_value(e)
                    .expect("error::WebDriver should always be serializeable to JSON"),
            )),
            Err(e) => Err(error::NewSessionError::UnexpectedError(e)),
        }
    }

    pub(crate) async fn create_client_and_parse_url(
        webdriver: &str,
        connector: C,
    ) -> Result<
        (
            hyper_util::client::legacy::Client<C, BoxBody<hyper::body::Bytes, Infallible>>,
            url::Url,
        ),
        error::NewSessionError,
    > {
        // Where is the WebDriver server?
        let wdb = webdriver
            .parse::<url::Url>()
            .map_err(error::NewSessionError::BadWebdriverUrl)?;

        // We want a tls-enabled client
        let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new())
            .build::<_, BoxBody<hyper::body::Bytes, Infallible>>(connector);

        Ok((client, wdb))
    }

    pub(crate) async fn setup_session(
        client: hyper_util::client::legacy::Client<C, BoxBody<hyper::body::Bytes, Infallible>>,
        wdb: url::Url,
        session_id: Option<&str>,
    ) -> Result<Client, error::NewSessionError> {
        // We're going to need a channel for sending requests to the WebDriver host
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(Session::new(
            rx,
            client,
            wdb,
            session_id.map(|id| id.to_string()),
        ));

        // now that the session is running, let's do the handshake
        Ok(Client {
            tx,
            new_session_response: None,
        })
    }

    pub(crate) async fn with_capabilities_and_connector(
        webdriver: &str,
        cap: &webdriver::capabilities::Capabilities,
        connector: C,
    ) -> Result<Client, error::NewSessionError> {
        let (client, wdb) = Self::create_client_and_parse_url(webdriver, connector).await?;
        let mut cap = cap.to_owned();

        // Create a new session for this client
        // https://www.w3.org/TR/webdriver/#dfn-new-session
        // https://www.w3.org/TR/webdriver/#capabilities
        //  - we want the browser to wait for the page to load
        if !cap.contains_key("pageLoadStrategy") {
            cap.insert("pageLoadStrategy".to_string(), Json::from("normal"));
        }

        // make chrome comply with w3c
        if cap.get("browserName") != Some(&Json::from("internet explorer")) {
            cap.entry("goog:chromeOptions".to_string())
                .or_insert_with(|| Json::Object(serde_json::Map::new()))
                .as_object_mut()
                .expect("goog:chromeOptions wasn't a JSON object")
                .insert("w3c".to_string(), Json::from(true));
        }

        let mut client = Self::setup_session(client, wdb, None).await?;

        let session_config = webdriver::capabilities::SpecNewSessionParameters {
            alwaysMatch: cap.clone(),
            firstMatch: vec![webdriver::capabilities::Capabilities::new()],
        };
        let spec = webdriver::command::NewSessionParameters {
            capabilities: session_config,
        };

        match client
            .issue(WebDriverCommand::NewSession(spec))
            .map(Self::map_handshake_response)
            .await
        {
            Ok(new_session_response) => {
                client.new_session_response =
                    Some(wd::NewSessionResponse::from_wd(new_session_response));
                Ok(client)
            }
            // the webdriver host _could_ still support the legacy webdriver protocol, but since
            // that's no longer supported by the webdriver crate, we also don't support it.
            Err(e) => Err(e),
        }
    }

    /// Helper for issuing a WebDriver command, and then reading and parsing the response.
    ///
    /// Since most `WebDriverCommand` arguments can already be turned directly into JSON, this is
    /// mostly a matter of picking the right URL and method from [the spec], and stuffing the JSON
    /// encoded arguments (if any) into the body.
    ///
    /// [the spec]: https://www.w3.org/TR/webdriver/#list-of-endpoints
    fn issue_wd_cmd(
        &self,
        cmd: Box<impl WebDriverCompatibleCommand + Send + 'static + ?Sized>,
    ) -> impl Future<Output = Result<Json, error::CmdError>> {
        // TODO: make this an async fn
        // will take some doing as returned future must be independent of self
        let url = match cmd.endpoint(&self.wdb, self.session.as_deref()) {
            Ok(url) => url,
            Err(e) => return Either::Right(future::err(error::CmdError::from(e))),
        };

        let (method, mut body) = cmd.method_and_body(&url);

        // issue the command to the webdriver server
        let mut req = hyper::Request::builder();
        req = req.method(method).uri(url.as_str());
        if let Some(ref s) = self.ua {
            req = req.header(hyper::header::USER_AGENT, s.to_owned());
        }
        // because https://github.com/hyperium/hyper/pull/727
        if !url.username().is_empty() || url.password().is_some() {
            req = req.header(
                hyper::header::AUTHORIZATION,
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode(&format!(
                        "{}:{}",
                        url.username(),
                        url.password().unwrap_or("")
                    ))
                ),
            );
        }

        let json_mime: mime::Mime = "application/json; charset=utf-8"
            .parse::<mime::Mime>()
            .unwrap_or(mime::APPLICATION_JSON);

        let req = if let Some(body) = body.take() {
            req = req.header(hyper::header::CONTENT_TYPE, json_mime.as_ref());
            req = req.header(hyper::header::CONTENT_LENGTH, body.len());
            self.client.request(req.body(BoxBody::new(body)).unwrap())
        } else {
            self.client.request(
                req.body(BoxBody::new(http_body_util::Empty::new()))
                    .unwrap(),
            )
        };

        let f = req
            .map_err(error::CmdError::from)
            .and_then(move |res| {
                // keep track of result status (.body() consumes self -- ugh)
                let status = res.status();

                // check that the server sent us json
                let ctype = res
                    .headers()
                    .get(hyper::header::CONTENT_TYPE)
                    .and_then(|ctype| ctype.to_str().ok()?.parse::<mime::Mime>().ok());

                // What did the server send us?
                res.into_body()
                    .collect()
                    .map_ok(|body| body.to_bytes())
                    .map_ok(move |body| (body, ctype, status))
                    .map_err(|e| -> error::CmdError { e.into() })
            })
            .map(|r| {
                let (body, ctype, status) = r?;

                // Too bad we can't stream into a String :(
                let body =
                    String::from_utf8(body.to_vec()).expect("non utf-8 response from webdriver");

                if let Some(ctype) = ctype {
                    if ctype.type_() == mime::APPLICATION_JSON.type_()
                        && ctype.subtype() == mime::APPLICATION_JSON.subtype()
                    {
                        Ok((body, status))
                    } else {
                        // nope, something else...
                        Err(error::CmdError::NotJson(body))
                    }
                } else {
                    // WebDriver host sent us something weird...
                    Err(error::CmdError::NotJson(body))
                }
            })
            .map(move |r| {
                let (body, status) = r?;
                let is_success = status.is_success();

                // https://www.w3.org/TR/webdriver/#dfn-send-a-response
                // NOTE: the standard specifies that even errors use the "Send a Response" steps
                let body = match serde_json::from_str(&*body)? {
                    Json::Object(mut v) => v
                        .remove("value")
                        .ok_or(error::CmdError::NotW3C(Json::Object(v))),
                    v => Err(error::CmdError::NotW3C(v)),
                }?;

                if is_success {
                    return Ok(body);
                }

                // https://www.w3.org/TR/webdriver/#dfn-send-an-error
                // https://www.w3.org/TR/webdriver/#handling-errors
                let mut body = match body {
                    Json::Object(o) => o,
                    j => return Err(error::CmdError::NotW3C(j)),
                };

                // phantomjs injects a *huge* field with the entire screen contents -- remove that
                body.remove("screen");

                if !body.contains_key("error")
                    || !body.contains_key("message")
                    || !body["error"].is_string()
                    || !body["message"].is_string()
                {
                    return Err(error::CmdError::NotW3C(Json::Object(body)));
                }

                let Some(es) = body["error"].as_str() else {
                    return Err(error::CmdError::NotW3C(Json::Object(body)));
                };
                let es = es.parse()?;

                let message = match body.remove("message") {
                    Some(Json::String(x)) => x,
                    _ => String::new(),
                };

                let mut wd_error = error::WebDriver::new(es, message);

                // Add the stacktrace if there is one.
                if let Some(Json::String(x)) = body.remove("stacktrace") {
                    wd_error = wd_error.with_stacktrace(x);
                }

                // Some commands may annotate errors with extra data.
                if let Some(x) = body.remove("data") {
                    wd_error = wd_error.with_data(x);
                }
                Err(error::CmdError::from_webdriver_error(wd_error))
            });

        Either::Left(f)
    }
}
