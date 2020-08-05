use crate::error;
use futures_core::ready;
use futures_util::future::{self, Either};
use futures_util::{FutureExt, TryFutureExt};
use serde_json::Value as Json;
use std::future::Future;
use std::io;
use std::mem;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::sync::{mpsc, oneshot};
use webdriver::command::{WebDriverCommand, WebDriverExtensionCommand, VoidWebDriverExtensionCommand};
use webdriver::error::ErrorStatus;
use webdriver::error::WebDriverError;

type Ack = oneshot::Sender<Result<Json, error::CmdError>>;

/// A WebDriver client tied to a single browser session.
#[derive(Clone, Debug)]
pub struct Client<T: ExtensionCommand> {
    tx: mpsc::UnboundedSender<Task<T>>,
    is_legacy: bool,
}

/// Browser Specific extension commands
pub trait ExtensionCommand: WebDriverExtensionCommand {
    /// Browser specific endpoint URL
    ///
    /// Example:-
    /// Use `moz/addon/install` to install an addon on geckodriver
    fn endpoint(&self) -> &str;

    /// Http method that using for calling to the endpoint
    fn method(&self) -> http::Method;
}

/// Extension command that does nothing. Use this type
/// when you don't want to use extension commands.
pub type VoidExtensionCommand = VoidWebDriverExtensionCommand;

impl ExtensionCommand for VoidExtensionCommand {
   fn endpoint(&self) -> &str {
        panic!("No extensions implemented");
   }

   fn method(&self) -> http::Method {
        panic!("No extensions implemented");
   }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum Cmd<T: ExtensionCommand> {
    SetUA(String),
    GetSessionId,
    Shutdown,
    Persist,
    GetUA,
    Raw {
        req: hyper::Request<hyper::Body>,
        rsp: oneshot::Sender<Result<hyper::Response<hyper::Body>, hyper::Error>>,
    },
    WebDriver(WebDriverCommand<T>),
}

impl<T: ExtensionCommand> From<WebDriverCommand<T>> for Cmd<T> {
    fn from(o: WebDriverCommand<T>) -> Self {
        Cmd::WebDriver(o)
    }
}

#[derive(Debug)]
pub(crate) struct Task<T: ExtensionCommand> {
    request: Cmd<T>,
    ack: Ack,
}

impl<T: ExtensionCommand> Client<T> {
    pub(crate) fn issue<C>(&mut self, cmd: C) -> impl Future<Output = Result<Json, error::CmdError>>
    where
        C: Into<Cmd<T>>,
    {
        let (tx, rx) = oneshot::channel();
        let cmd = cmd.into();
        let r = self.tx.send(Task {
            request: cmd,
            ack: tx,
        });

        async move {
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
    }

    pub(crate) fn is_legacy(&self) -> bool {
        self.is_legacy
    }
}

enum Ongoing {
    None,
    Break,
    Shutdown {
        ack: Option<Ack>,
        fut: hyper::client::ResponseFuture,
    },
    WebDriver {
        ack: Ack,
        fut: Pin<Box<dyn Future<Output = Result<Json, error::CmdError>> + Send>>,
    },
    Raw {
        ack: Ack,
        ret: oneshot::Sender<Result<hyper::Response<hyper::Body>, hyper::Error>>,
        fut: hyper::client::ResponseFuture,
    },
}

enum OngoingResult {
    Continue,
    Break,
    SessionId(String),
}

impl Ongoing {
    fn is_some(&self) -> bool {
        if let Ongoing::None = *self {
            false
        } else {
            true
        }
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

pub(crate) struct Session<T: ExtensionCommand> {
    ongoing: Ongoing,
    rx: mpsc::UnboundedReceiver<Task<T>>,
    client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
    wdb: url::Url,
    session: Option<String>,
    is_legacy: bool,
    ua: Option<String>,
    persist: bool,
}

impl<T: ExtensionCommand + 'static> Future for Session<T> {
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
                    Cmd::SetUA(ua) => {
                        self.ua = Some(ua);
                        let _ = ack.send(Ok(Json::Null));
                    }
                    Cmd::GetUA => {
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
                        // looks like the client setup is falling back to legacy params
                        // keep track of that for later
                        if let WebDriverCommand::NewSession(
                            webdriver::command::NewSessionParameters::Legacy(..),
                        ) = request
                        {
                            self.is_legacy = true;
                        }
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

impl<T: ExtensionCommand + 'static> Session<T> {
    fn shutdown(&mut self, ack: Option<Ack>) {
        let url = {
            self.wdb
                .join(&format!("session/{}", self.session.as_ref().unwrap()))
                .unwrap()
        };

        self.ongoing = Ongoing::Shutdown {
            ack,
            fut: self.client.request(
                hyper::Request::delete(url.as_str())
                    .body(hyper::Body::empty())
                    .unwrap(),
            ),
        };
    }

    fn map_handshake_response(
        response: Result<Json, error::CmdError>,
    ) -> Result<(), error::NewSessionError> {
        match response {
            Ok(Json::Object(mut v)) => {
                // TODO: not all impls are w3c compatible
                // See https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L189
                // and https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L200
                // NOTE: remove so we can re-insert and return if something's wrong
                if let Some(session_id) = v.remove("sessionId") {
                    if session_id.is_string() {
                        return Ok(());
                    }
                    v.insert("sessionId".to_string(), session_id);
                    Err(error::NewSessionError::NotW3C(Json::Object(v)))
                } else {
                    Err(error::NewSessionError::NotW3C(Json::Object(v)))
                }
            }
            Ok(v) | Err(error::CmdError::NotW3C(v)) => Err(error::NewSessionError::NotW3C(v)),
            Err(error::CmdError::Failed(e)) => Err(error::NewSessionError::Failed(e)),
            Err(error::CmdError::Lost(e)) => Err(error::NewSessionError::Lost(e)),
            Err(error::CmdError::NotJson(v)) => {
                Err(error::NewSessionError::NotW3C(Json::String(v)))
            }
            Err(error::CmdError::Standard(
                e
                @
                WebDriverError {
                    error: ErrorStatus::SessionNotCreated,
                    ..
                },
            )) => Err(error::NewSessionError::SessionNotCreated(e)),
            Err(e) => {
                panic!("unexpected webdriver error; {}", e);
            }
        }
    }

    pub(crate) async fn with_capabilities(
        webdriver: &str,
        mut cap: webdriver::capabilities::Capabilities,
    ) -> Result<Client<T>, error::NewSessionError> {
        // Where is the WebDriver server?
        let wdb = webdriver.parse::<url::Url>();

        let wdb = wdb.map_err(error::NewSessionError::BadWebdriverUrl)?;

        // We want a tls-enabled client
        let client =
            hyper::Client::builder().build::<_, hyper::Body>(hyper_tls::HttpsConnector::new());

        // We're going to need a channel for sending requests to the WebDriver host
        let (tx, rx) = mpsc::unbounded_channel();

        // Set up our WebDriver session.
        tokio::spawn(Session {
            rx,
            ongoing: Ongoing::None,
            client,
            wdb,
            session: None,
            is_legacy: false,
            ua: None,
            persist: false,
        });

        // now that the session is running, let's do the handshake
        let mut client = Client {
            tx: tx.clone(),
            is_legacy: false,
        };

        // Create a new session for this client
        // https://www.w3.org/TR/webdriver/#dfn-new-session
        // https://www.w3.org/TR/webdriver/#capabilities
        //  - we want the browser to wait for the page to load
        cap.insert("pageLoadStrategy".to_string(), Json::from("normal"));

        // make chrome comply with w3c
        cap.entry("goog:chromeOptions".to_string())
            .or_insert_with(|| Json::Object(serde_json::Map::new()))
            .as_object_mut()
            .expect("goog:chromeOptions wasn't a JSON object")
            .insert("w3c".to_string(), Json::from(true));

        let session_config = webdriver::capabilities::SpecNewSessionParameters {
            alwaysMatch: cap.clone(),
            firstMatch: vec![webdriver::capabilities::Capabilities::new()],
        };
        let spec = webdriver::command::NewSessionParameters::Spec(session_config);

        match client
            .issue(WebDriverCommand::NewSession(spec))
            .map(Self::map_handshake_response)
            .await
        {
            Ok(_) => Ok(Client {
                tx,
                is_legacy: false,
            }),
            Err(error::NewSessionError::NotW3C(json)) => {
                // maybe try legacy mode?
                let mut legacy = false;
                match json {
                    Json::String(ref err) if err.starts_with("Missing Command Parameter") => {
                        // ghostdriver
                        legacy = true;
                    }
                    Json::Object(ref err) => {
                        legacy = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .map(|s| {
                                // chromedriver < 2.29 || chromedriver == 2.29 || saucelabs
                                s.contains("cannot find dict 'desiredCapabilities'")
                                    || s.contains("Missing or invalid capabilities")
                                    || s.contains("Unexpected server error.")
                            })
                            .unwrap_or(false);
                    }
                    _ => {}
                }

                if !legacy {
                    return Err(error::NewSessionError::NotW3C(json));
                }

                // we're dealing with an implementation that only supports the legacy
                // WebDriver protocol:
                // https://github.com/SeleniumHQ/selenium/wiki/JsonWireProtocol
                let session_config = webdriver::capabilities::LegacyNewSessionParameters {
                    desired: cap,
                    required: webdriver::capabilities::Capabilities::new(),
                };
                let spec = webdriver::command::NewSessionParameters::Legacy(session_config);

                // try again with a legacy client
                client
                    .issue(WebDriverCommand::NewSession(spec))
                    .map(Self::map_handshake_response)
                    .await?;

                Ok(Client {
                    tx,
                    is_legacy: true,
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Helper for determining what URL endpoint to use for various requests.
    ///
    /// This mapping is essentially that of https://www.w3.org/TR/webdriver/#list-of-endpoints.
    fn endpoint_for(&self, cmd: &WebDriverCommand<T>) -> Result<url::Url, url::ParseError> {
        if let WebDriverCommand::NewSession(..) = *cmd {
            return self.wdb.join("session");
        }

        let base = {
            self.wdb
                .join(&format!("session/{}/", self.session.as_ref().unwrap()))?
        };
        match &*cmd {
            WebDriverCommand::NewSession(..) => unreachable!(),
            WebDriverCommand::DeleteSession => unreachable!(),
            WebDriverCommand::Get(..) | WebDriverCommand::GetCurrentUrl => base.join("url"),
            WebDriverCommand::GoBack => base.join("back"),
            WebDriverCommand::Refresh => base.join("refresh"),
            WebDriverCommand::GetPageSource => base.join("source"),
            WebDriverCommand::FindElement(..) => base.join("element"),
            WebDriverCommand::FindElements(..) => base.join("elements"),
            WebDriverCommand::GetCookies => base.join("cookie"),
            WebDriverCommand::ExecuteScript(..) if self.is_legacy => base.join("execute"),
            WebDriverCommand::ExecuteScript(..) => base.join("execute/sync"),
            WebDriverCommand::GetElementProperty(ref we, ref prop) => {
                base.join(&format!("element/{}/property/{}", we.0, prop))
            }
            WebDriverCommand::GetElementAttribute(ref we, ref attr) => {
                base.join(&format!("element/{}/attribute/{}", we.0, attr))
            }
            WebDriverCommand::FindElementElement(ref p, _) => {
                base.join(&format!("element/{}/element", p.0))
            }
            WebDriverCommand::FindElementElements(ref p, _) => {
                base.join(&format!("element/{}/elements", p.0))
            }
            WebDriverCommand::ElementClick(ref we) => base.join(&format!("element/{}/click", we.0)),
            WebDriverCommand::ElementClear(ref we) => base.join(&format!("element/{}/clear", we.0)),
            WebDriverCommand::GetElementText(ref we) => {
                base.join(&format!("element/{}/text", we.0))
            }
            WebDriverCommand::ElementSendKeys(ref we, _) => {
                base.join(&format!("element/{}/value", we.0))
            }
            WebDriverCommand::SetWindowRect(..) => base.join("window/rect"),
            WebDriverCommand::GetWindowRect => base.join("window/rect"),
            WebDriverCommand::TakeScreenshot => base.join("screenshot"),
            WebDriverCommand::SwitchToFrame(_) => base.join("frame"),
            WebDriverCommand::SwitchToParentFrame => base.join("frame/parent"),
            WebDriverCommand::GetWindowHandle => base.join("window"),
            WebDriverCommand::GetWindowHandles => base.join("window/handles"),
            WebDriverCommand::NewWindow(..) => base.join("window/new"),
            WebDriverCommand::SwitchToWindow(..) => base.join("window"),
            WebDriverCommand::CloseWindow => base.join("window"),
            WebDriverCommand::Extension(extension_command) => {
                let endpoint = extension_command.endpoint();
                let endpoint = endpoint.trim_start_matches("/");
                base.join(endpoint)
            }
            _ => unimplemented!(),
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
        &mut self,
        cmd: WebDriverCommand<T>,
    ) -> impl Future<Output = Result<Json, error::CmdError>> {
        // TODO: make this an async fn
        // will take some doing as returned future must be independent of self

        use webdriver::command;

        // most actions are just get requests with not parameters
        let url = match self.endpoint_for(&cmd) {
            Ok(url) => url,
            Err(e) => return Either::Right(future::err(error::CmdError::from(e))),
        };
        use hyper::Method;
        let mut method = Method::GET;
        let mut body = None;

        // but some are special
        match &cmd {
            WebDriverCommand::NewSession(command::NewSessionParameters::Spec(ref conf)) => {
                // TODO: awful hacks
                let mut also = String::new();
                if !url.username().is_empty() {
                    also.push_str(&format!(
                        r#", "user": {}"#,
                        serde_json::to_string(url.username()).unwrap()
                    ));
                }
                if let Some(pwd) = url.password() {
                    also.push_str(&format!(
                        r#", "password": {}"#,
                        serde_json::to_string(pwd).unwrap()
                    ));
                }
                body = Some(format!(
                    r#"{{"capabilities": {}{}}}"#,
                    serde_json::to_string(conf).unwrap(),
                    also
                ));

                method = Method::POST;
            }
            WebDriverCommand::NewSession(command::NewSessionParameters::Legacy(ref conf)) => {
                body = Some(serde_json::to_string(conf).unwrap());
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
            WebDriverCommand::ElementSendKeys(_, ref keys) => {
                body = Some(serde_json::to_string(keys).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::ElementClick(..)
            | WebDriverCommand::ElementClear(..)
            | WebDriverCommand::GoBack
            | WebDriverCommand::Refresh => {
                body = Some("{}".to_string());
                method = Method::POST;
            }
            WebDriverCommand::SetWindowRect(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::SwitchToFrame(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST
            }
            WebDriverCommand::SwitchToParentFrame => {
                body = Some("{}".to_string());
                method = Method::POST
            }
            WebDriverCommand::SwitchToWindow(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::NewWindow(ref params) => {
                body = Some(serde_json::to_string(params).unwrap());
                method = Method::POST;
            }
            WebDriverCommand::CloseWindow => {
                method = Method::DELETE;
            }
            WebDriverCommand::Extension(ext_command) => {
                method = ext_command.method();
                if let Some(param) = ext_command.parameters_json() {
                    body = Some(param.to_string())
                }
            }
            _ => {}
        }

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
                    base64::encode(&format!(
                        "{}:{}",
                        url.username(),
                        url.password().unwrap_or("")
                    ))
                ),
            );
        }

        let req = if let Some(body) = body.take() {
            req = req.header(hyper::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());
            req = req.header(hyper::header::CONTENT_LENGTH, body.len());
            self.client.request(req.body(body.into()).unwrap())
        } else {
            self.client.request(req.body(hyper::Body::empty()).unwrap())
        };

        let legacy = self.is_legacy;
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
                hyper::body::to_bytes(res.into_body())
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
                let is_new_session = if let WebDriverCommand::NewSession(..) = cmd {
                    true
                } else {
                    false
                };

                let mut is_success = status.is_success();
                let mut legacy_status = 0;

                // https://www.w3.org/TR/webdriver/#dfn-send-a-response
                // NOTE: the standard specifies that even errors use the "Send a Reponse" steps
                let body = match serde_json::from_str(&*body)? {
                    Json::Object(mut v) => {
                        if legacy {
                            legacy_status = v["status"].as_u64().unwrap();
                            is_success = legacy_status == 0;
                        }

                        if legacy && is_new_session {
                            // legacy implementations do not wrap sessionId inside "value"
                            Ok(Json::Object(v))
                        } else {
                            v.remove("value")
                                .ok_or_else(|| error::CmdError::NotW3C(Json::Object(v)))
                        }
                    }
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

                let es = if legacy {
                    // old clients use status codes instead of "error", and we now have to map them
                    // https://github.com/SeleniumHQ/selenium/wiki/JsonWireProtocol#response-status-codes
                    if !body.contains_key("message") || !body["message"].is_string() {
                        return Err(error::CmdError::NotW3C(Json::Object(body)));
                    }
                    match legacy_status {
                        6 | 33 => ErrorStatus::SessionNotCreated,
                        7 => ErrorStatus::NoSuchElement,
                        8 => ErrorStatus::NoSuchFrame,
                        9 => ErrorStatus::UnknownCommand,
                        10 => ErrorStatus::StaleElementReference,
                        11 => ErrorStatus::ElementNotInteractable,
                        12 => ErrorStatus::InvalidElementState,
                        13 => ErrorStatus::UnknownError,
                        15 => ErrorStatus::ElementNotSelectable,
                        17 => ErrorStatus::JavascriptError,
                        19 | 32 => ErrorStatus::InvalidSelector,
                        21 => ErrorStatus::Timeout,
                        23 => ErrorStatus::NoSuchWindow,
                        24 => ErrorStatus::InvalidCookieDomain,
                        25 => ErrorStatus::UnableToSetCookie,
                        26 => ErrorStatus::UnexpectedAlertOpen,
                        27 => ErrorStatus::NoSuchAlert,
                        28 => ErrorStatus::ScriptTimeout,
                        29 => ErrorStatus::InvalidCoordinates,
                        34 => ErrorStatus::MoveTargetOutOfBounds,
                        _ => return Err(error::CmdError::NotW3C(Json::Object(body))),
                    }
                } else {
                    if !body.contains_key("error")
                        || !body.contains_key("message")
                        || !body["error"].is_string()
                        || !body["message"].is_string()
                    {
                        return Err(error::CmdError::NotW3C(Json::Object(body)));
                    }

                    use hyper::StatusCode;
                    let error = body["error"].as_str().unwrap();
                    match status {
                        StatusCode::BAD_REQUEST => match error {
                            "element click intercepted" => ErrorStatus::ElementClickIntercepted,
                            "element not selectable" => ErrorStatus::ElementNotSelectable,
                            "element not interactable" => ErrorStatus::ElementNotInteractable,
                            "insecure certificate" => ErrorStatus::InsecureCertificate,
                            "invalid argument" => ErrorStatus::InvalidArgument,
                            "invalid cookie domain" => ErrorStatus::InvalidCookieDomain,
                            "invalid coordinates" => ErrorStatus::InvalidCoordinates,
                            "invalid element state" => ErrorStatus::InvalidElementState,
                            "invalid selector" => ErrorStatus::InvalidSelector,
                            "no such alert" => ErrorStatus::NoSuchAlert,
                            "no such frame" => ErrorStatus::NoSuchFrame,
                            "no such window" => ErrorStatus::NoSuchWindow,
                            "stale element reference" => ErrorStatus::StaleElementReference,
                            _ => unreachable!(
                                "received unknown error ({}) for BAD_REQUEST status code",
                                error
                            ),
                        },
                        StatusCode::NOT_FOUND => match error {
                            "unknown command" => ErrorStatus::UnknownCommand,
                            "no such cookie" => ErrorStatus::NoSuchCookie,
                            "invalid session id" => ErrorStatus::InvalidSessionId,
                            "no such element" => ErrorStatus::NoSuchElement,
                            "no such window" => ErrorStatus::NoSuchWindow,
                            "stale element reference" => ErrorStatus::NoSuchElement,
                            _ => unreachable!(
                                "received unknown error ({}) for NOT_FOUND status code",
                                error
                            ),
                        },
                        StatusCode::INTERNAL_SERVER_ERROR => match error {
                            "javascript error" => ErrorStatus::JavascriptError,
                            "move target out of bounds" => ErrorStatus::MoveTargetOutOfBounds,
                            "session not created" => ErrorStatus::SessionNotCreated,
                            "unable to set cookie" => ErrorStatus::UnableToSetCookie,
                            "unable to capture screen" => ErrorStatus::UnableToCaptureScreen,
                            "unexpected alert open" => ErrorStatus::UnexpectedAlertOpen,
                            "unknown error" => ErrorStatus::UnknownError,
                            "unsupported operation" => ErrorStatus::UnsupportedOperation,
                            _ => unreachable!(
                                "received unknown error ({}) for INTERNAL_SERVER_ERROR status code",
                                error
                            ),
                        },
                        StatusCode::REQUEST_TIMEOUT => match error {
                            "timeout" => ErrorStatus::Timeout,
                            "script timeout" => ErrorStatus::ScriptTimeout,
                            _ => unreachable!(
                                "received unknown error ({}) for REQUEST_TIMEOUT status code",
                                error
                            ),
                        },
                        StatusCode::METHOD_NOT_ALLOWED => match error {
                            "unknown method" => ErrorStatus::UnknownMethod,
                            _ => unreachable!(
                                "received unknown error ({}) for METHOD_NOT_ALLOWED status code",
                                error
                            ),
                        },
                        _ => unreachable!("received unknown status code: {}", status),
                    }
                };

                let message = body["message"].as_str().unwrap().to_string();
                Err(error::CmdError::from(WebDriverError::new(es, message)))
            });

        Either::Left(f)
    }
}
