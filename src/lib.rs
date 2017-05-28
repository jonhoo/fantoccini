//! A high-level API for programmatically interacting with web pages.
//!
//! This crate uses the [WebDriver protocol] to drive a conforming (potentially headless) browser
//! through relatively high-level operations such as "click this element", "submit this form", etc.
//!
//! Most interactions are driven by using [CSS selectors]. With most WebDriver-compatible browser
//! being fairly recent, the more expressive levels of the CSS standard are also supported, giving
//! fairly [powerful] [operators].
//!
//! Forms are managed by first calling `Client::form`, and then using the methods on `Form` to
//! manipulate the form's fields and eventually submitting it.
//!
//! For low-level access to the page, `Client::source` can be used to fetch the full page HTML
//! source code, and `Client::raw_client_for` to build a raw HTTP request for a particular URL.
//!
//! [WebDriver protocol]: https://www.w3.org/TR/webdriver/
//! [CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
//! [powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
//! [operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
#![deny(missing_docs)]

extern crate hyper_native_tls;
extern crate rustc_serialize;
extern crate webdriver;
extern crate cookie;
extern crate hyper;

use webdriver::command::WebDriverCommand;
use webdriver::error::WebDriverError;
use webdriver::error::ErrorStatus;
use webdriver::common::ELEMENT_KEY;
use rustc_serialize::json::Json;
use std::io::prelude::*;

pub use hyper::method::Method;

/// Error types.
pub mod error;

type Cmd = WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>;

/// A WebDriver client tied to a single browser session.
pub struct Client {
    c: hyper::Client,
    wdb: hyper::Url,
    session: Option<String>,
}

/// An HTML form on the current page.
pub struct Form<'a> {
    c: &'a mut Client,
    f: webdriver::common::WebElement,
}

impl Client {
    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    pub fn new<U: hyper::client::IntoUrl>(webdriver: U) -> Result<Self, error::NewSessionError> {
        // Where is the WebDriver server?
        let wdb = webdriver
            .into_url()
            .map_err(|e| error::NewSessionError::BadWebdriverUrl(e))?;

        // We want tls
        let ssl = hyper_native_tls::NativeTlsClient::new().unwrap();
        let connector = hyper::net::HttpsConnector::new(ssl);
        let client = hyper::Client::with_connector(connector);

        // Set up our WebDriver client
        let mut c = Client {
            c: client,
            wdb,
            session: None,
        };

        // Required capabilities
        // https://www.w3.org/TR/webdriver/#capabilities
        let mut cap = webdriver::capabilities::Capabilities::new();
        //  - we want the browser to wait for the page to load
        cap.insert("pageLoadStrategy".to_string(),
                   Json::String("normal".to_string()));

        let session_config = webdriver::capabilities::SpecNewSessionParameters {
            alwaysMatch: cap,
            firstMatch: vec![],
        };
        let spec = webdriver::command::NewSessionParameters::Spec(session_config);

        // Create a new session for this client
        // https://www.w3.org/TR/webdriver/#dfn-new-session
        match c.issue_wd_cmd(WebDriverCommand::NewSession(spec)) {
            Ok(Json::Object(mut v)) => {
                // TODO: not all impls are w3c compatible
                // See https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L189
                // and https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L200
                if let Some(session_id) = v.remove("sessionId") {
                    if let Some(session_id) = session_id.as_string() {
                        c.session = Some(session_id.to_string());
                        return Ok(c);
                    }
                    v.insert("sessionId".to_string(), session_id);
                    Err(error::NewSessionError::NotW3C(Json::Object(v)))
                } else {
                    Err(error::NewSessionError::NotW3C(Json::Object(v)))
                }
            }
            Ok(v) => Err(error::NewSessionError::NotW3C(v)),
            Err(error::CmdError::Standard(e @ WebDriverError {
                                              error: ErrorStatus::SessionNotCreated, ..
                                          })) => Err(error::NewSessionError::SessionNotCreated(e)),
            Err(e) => {
                panic!("unexpected webdriver error; {}", e);
            }
        }
    }

    /// Helper for determining what URL endpoint to use for various requests.
    ///
    /// This mapping is essentially that of https://www.w3.org/TR/webdriver/#list-of-endpoints.
    fn endpoint_for(&self, cmd: &Cmd) -> Result<hyper::Url, hyper::error::ParseError> {
        if let WebDriverCommand::NewSession(..) = *cmd {
            return self.wdb.join("/session");
        }

        let session = self.session.as_ref().unwrap();
        if let WebDriverCommand::DeleteSession = *cmd {
            return self.wdb.join(&format!("/session/{}", session));
        }

        let base = self.wdb.join(&format!("/session/{}/", session))?;
        match *cmd {
            WebDriverCommand::NewSession(..) => unreachable!(),
            WebDriverCommand::DeleteSession => unreachable!(),
            WebDriverCommand::Get(..) |
            WebDriverCommand::GetCurrentUrl => base.join("url"),
            WebDriverCommand::GetPageSource => base.join("source"),
            WebDriverCommand::FindElement(..) => base.join("element"),
            WebDriverCommand::GetCookies => base.join("cookie"),
            WebDriverCommand::ExecuteScript(..) => base.join("execute/sync"),
            WebDriverCommand::GetElementProperty(ref we, ref prop) => {
                base.join(&format!("element/{}/property/{}", we.id, prop))
            }
            WebDriverCommand::GetElementAttribute(ref we, ref attr) => {
                base.join(&format!("element/{}/attribute/{}", we.id, attr))
            }
            WebDriverCommand::FindElementElement(ref p, _) => {
                base.join(&format!("element/{}/element", p.id))
            }
            WebDriverCommand::ElementClick(ref we) => {
                base.join(&format!("element/{}/click", we.id))
            }
            WebDriverCommand::GetElementText(ref we) => {
                base.join(&format!("element/{}/text", we.id))
            }
            WebDriverCommand::ElementSendKeys(ref we, _) => {
                base.join(&format!("element/{}/value", we.id))
            }
            _ => unimplemented!(),
        }
    }

    /// Helper for issuing a WebDriver command, and then reading and parsing the response.
    ///
    /// Since most `WebDriverCommand` arguments already implement `ToJson`, this is mostly a matter
    /// of picking the right URL and method from [the spec], and stuffing the JSON encoded
    /// arguments (if any) into the body.
    ///
    /// [the spec]: https://www.w3.org/TR/webdriver/#list-of-endpoints
    fn issue_wd_cmd(&self,
                    cmd: WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>)
                    -> Result<Json, error::CmdError> {
        use rustc_serialize::json::ToJson;
        use hyper::method::Method;
        use webdriver::command;

        // most actions are just get requests with not parameters
        let url = self.endpoint_for(&cmd)?;
        let mut method = Method::Get;
        let mut body = None;

        // but some are special
        match cmd {
            WebDriverCommand::NewSession(command::NewSessionParameters::Spec(ref conf)) => {
                body = Some(format!("{}", conf.to_json()));
                method = Method::Post;
            }
            WebDriverCommand::Get(ref params) => {
                body = Some(format!("{}", params.to_json()));
                method = Method::Post;
            }
            WebDriverCommand::FindElement(ref loc) |
            WebDriverCommand::FindElementElement(_, ref loc) => {
                body = Some(format!("{}", loc.to_json()));
                method = Method::Post;
            }
            WebDriverCommand::ExecuteScript(ref script) => {
                body = Some(format!("{}", script.to_json()));
                method = Method::Post;
            }
            WebDriverCommand::ElementSendKeys(_, ref keys) => {
                body = Some(format!("{}", keys.to_json()));
                method = Method::Post;
            }
            WebDriverCommand::ElementClick(..) => {
                body = Some("{}".to_string());
                method = Method::Post;
            }
            WebDriverCommand::DeleteSession => {
                method = Method::Delete;
            }
            _ => {}
        }

        // issue the command to the webdriver server
        let mut res = {
            let req = self.c.request(method, url);
            if let Some(ref body) = body {
                let json = body.as_bytes();
                req.body(hyper::client::Body::BufBody(json, json.len()))
                    .send()
            } else {
                req.send()
            }
        }?;

        if let WebDriverCommand::ElementClick(..) = cmd {
            // unfortunately implementations seem to sometimes return very eagerly
            use std::thread;
            use std::time::Duration;
            thread::sleep(Duration::from_millis(500));
        }

        // check that the server sent us json
        use hyper::mime::{Mime, TopLevel, SubLevel};
        let ctype = {
            let ctype = res.headers
                .get::<hyper::header::ContentType>()
                .expect("webdriver response did not have a content type");
            (**ctype).clone()
        };
        match ctype {
            Mime(TopLevel::Application, SubLevel::Json, _) => {}
            _ => {
                // nope, something else...
                let mut body = String::new();
                res.read_to_string(&mut body)?;
                return Err(error::CmdError::NotJson(body));
            }
        }


        // https://www.w3.org/TR/webdriver/#dfn-send-a-response
        // NOTE: the standard specifies that even errors use the "Send a Reponse" steps
        let body = match Json::from_reader(&mut res)? {
            Json::Object(mut v) => {
                v.remove("value")
                    .ok_or_else(|| error::CmdError::NotW3C(Json::Object(v)))
            }
            v => Err(error::CmdError::NotW3C(v)),
        }?;

        if res.status.is_success() {
            return Ok(body);
        }

        // https://www.w3.org/TR/webdriver/#dfn-send-an-error
        // https://www.w3.org/TR/webdriver/#handling-errors
        if !body.is_object() {
            return Err(error::CmdError::NotW3C(body));
        }
        let body = body.into_object().unwrap();
        if !body.contains_key("error") || !body.contains_key("message") ||
           !body["error"].is_string() || !body["message"].is_string() {
            return Err(error::CmdError::NotW3C(Json::Object(body)));
        }

        use hyper::status::StatusCode;
        let error = body["error"].as_string().unwrap();
        let error = match res.status {
            StatusCode::BadRequest => {
                match error {
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
                    _ => unreachable!(),
                }
            }
            StatusCode::NotFound => {
                match error {
                    "unknown command" => ErrorStatus::UnknownCommand,
                    "no such cookie" => ErrorStatus::NoSuchCookie,
                    "invalid session id" => ErrorStatus::InvalidSessionId,
                    "no such element" => ErrorStatus::NoSuchElement,
                    _ => unreachable!(),
                }
            }
            StatusCode::InternalServerError => {
                match error {
                    "javascript error" => ErrorStatus::JavascriptError,
                    "move target out of bounds" => ErrorStatus::MoveTargetOutOfBounds,
                    "session not created" => ErrorStatus::SessionNotCreated,
                    "unable to set cookie" => ErrorStatus::UnableToSetCookie,
                    "unable to capture screen" => ErrorStatus::UnableToCaptureScreen,
                    "unexpected alert open" => ErrorStatus::UnexpectedAlertOpen,
                    "unknown error" => ErrorStatus::UnknownError,
                    "unsupported operation" => ErrorStatus::UnsupportedOperation,
                    _ => unreachable!(),
                }
            }
            StatusCode::RequestTimeout => {
                match error {
                    "timeout" => ErrorStatus::Timeout,
                    "script timeout" => ErrorStatus::ScriptTimeout,
                    _ => unreachable!(),
                }
            }
            StatusCode::MethodNotAllowed => {
                match error {
                    "unknown method" => ErrorStatus::UnknownMethod,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };

        let message = body["message"].as_string().unwrap().to_string();
        Err(WebDriverError::new(error, message).into())
    }

    /// Navigate directly to the given URL.
    pub fn goto<'a>(&'a mut self, url: &str) -> Result<&'a mut Self, error::CmdError> {
        let url = self.current_url()?.join(url)?;
        self.issue_wd_cmd(WebDriverCommand::Get(webdriver::command::GetParameters {
                                                    url: url.into_string(),
                                                }))?;
        Ok(self)
    }

    /// Retrieve the currently active URL for this session.
    pub fn current_url(&self) -> Result<hyper::Url, error::CmdError> {
        let url = self.issue_wd_cmd(WebDriverCommand::GetCurrentUrl)?;
        if let Some(url) = url.as_string() {
            return Ok(hyper::Url::parse(url)?);
        }

        Err(error::CmdError::NotW3C(url))
    }

    /// Get the HTML source for the current page.
    pub fn source(&self) -> Result<String, error::CmdError> {
        let src = self.issue_wd_cmd(WebDriverCommand::GetPageSource)?;
        if let Some(src) = src.as_string() {
            return Ok(src.to_string());
        }

        Err(error::CmdError::NotW3C(src))
    }

    /// Get a `hyper::RequestBuilder` instance with all the same cookies as the current session has
    /// for the given `url`.
    ///
    /// The `RequestBuilder` can then be used to fetch a resource with more granular control (such
    /// as downloading a file).
    ///
    /// Note that the client is tied to the lifetime of the client to prevent the `Client` from
    /// navigating to another page. This is because it would likely be confusing that the builder
    /// did not *also* navigate. Furthermore, the builder's cookies are tied to the URL at the time
    /// of its creation, so after navigation, the user (that's you) may be confused that the right
    /// cookies aren't being included (I know I would).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fantoccini::Client;
    /// let mut c = Client::new("http://localhost:4444").unwrap();
    /// c.goto("https://www.wikipedia.org/").unwrap();
    /// let img = c.lookup_attr("img.central-featured-logo", "src").unwrap().unwrap();
    /// let raw = c.raw_client_for(fantoccini::Method::Get, &img).unwrap();
    /// let mut res = raw.send().unwrap();
    ///
    /// use std::io::prelude::*;
    /// let mut pixels = Vec::new();
    /// res.read_to_end(&mut pixels).unwrap();
    /// println!("Wikipedia logo is {}b", pixels.len());
    /// ```
    pub fn raw_client_for<'a>(&'a mut self,
                              method: Method,
                              url: &str)
                              -> Result<hyper::client::RequestBuilder<'a>, error::CmdError> {
        // We need to do some trickiness here. GetCookies will only give us the cookies for the
        // *current* domain, whereas we want the cookies for `url`'s domain. The fact that cookies
        // can have /path and security constraints makes this even more of a pain. So, to get
        // around all this, we navigate to the URL in question, fetch its cookies, and then
        // navigate back. *Except* that we can't do that either (what if `url` is some huge file?).
        // So we *actually* navigate to some weird url that's deeper than `url`, and hope that we
        // don't end up with a redirect to somewhere entirely different.
        let old_url = self.current_url()?;
        let url = old_url.clone().join(url)?;
        let cookie_url = url.clone().join("please_give_me_your_cookies")?;
        self.goto(&format!("{}", cookie_url))?;
        let cookies = match self.issue_wd_cmd(WebDriverCommand::GetCookies) {
            Ok(cookies) => cookies,
            Err(e) => {
                // go back before we return
                self.goto(&format!("{}", old_url))?;
                return Err(e);
            }
        };
        self.goto(&format!("{}", old_url))?;

        if !cookies.is_array() {
            return Err(error::CmdError::NotW3C(cookies));
        }
        let cookies = cookies.into_array().unwrap();

        // now add all the cookies
        let mut all_ok = true;
        let mut jar = Vec::new();
        for cookie in &cookies {
            if !cookie.is_object() {
                all_ok = false;
                break;
            }

            // https://w3c.github.io/webdriver/webdriver-spec.html#cookies
            let cookie = cookie.as_object().unwrap();
            if !cookie.contains_key("name") || !cookie.contains_key("value") {
                all_ok = false;
                break;
            }

            if !cookie["name"].is_string() || !cookie["value"].is_string() {
                all_ok = false;
                break;
            }

            let val_of = |key| match cookie.get(key) {
                None => webdriver::common::Nullable::Null,
                Some(v) => {
                    if v.is_null() {
                        webdriver::common::Nullable::Null
                    } else {
                        webdriver::common::Nullable::Value(v.clone())
                    }
                }
            };

            let path = val_of("path").map(|v| if let Some(s) = v.as_string() {
                                              s.to_string()
                                          } else {
                                              unimplemented!();
                                          });
            let domain = val_of("domain").map(|v| if let Some(s) = v.as_string() {
                                                  s.to_string()
                                              } else {
                                                  unimplemented!();
                                              });
            let expiry = val_of("expiry").map(|v| if let Some(secs) = v.as_u64() {
                                                  webdriver::common::Date::new(secs)
                                              } else {
                                                  unimplemented!();
                                              });

            // Object({"domain": String("www.wikipedia.org"), "expiry": Null, "httpOnly": Boolean(false), "name": String("CP"), "path": String("/"), "secure": Boolean(false), "value": String("H2")}
            // NOTE: too bad webdriver::response::Cookie doesn't implement FromJson
            let cookie = webdriver::response::Cookie {
                name: cookie["name"].as_string().unwrap().to_string(),
                value: cookie["value"].as_string().unwrap().to_string(),
                path: path,
                domain: domain,
                expiry: expiry,
                secure: cookie
                    .get("secure")
                    .and_then(|v| v.as_boolean())
                    .unwrap_or(false),
                httpOnly: cookie
                    .get("httpOnly")
                    .and_then(|v| v.as_boolean())
                    .unwrap_or(false),
            };

            // so many cookies
            let cookie: cookie::Cookie = cookie.into();
            jar.push(format!("{}", cookie));
        }

        if all_ok {
            println!("making {:?} request for {} with cookies: {:#?}",
                     method,
                     url,
                     jar);
            let mut headers = hyper::header::Headers::new();
            headers.set(hyper::header::Cookie(jar));
            Ok(self.c.request(method, url).headers(headers))
        } else {
            Err(error::CmdError::NotW3C(Json::Array(cookies)))
        }
    }

    /// Look up an [attribute] value by name for the element matching `selector`.
    ///
    /// `selector` should be a CSS selector. `Ok(None)` is returned if the element does not have
    /// the given attribute. `Err(NoSuchElement)` is returned if the element could not be found.
    ///
    /// [attribute]: https://dom.spec.whatwg.org/#concept-attribute
    pub fn lookup_attr(&self,
                       selector: &str,
                       attribute: &str)
                       -> Result<Option<String>, error::CmdError> {
        let e = self.lookup(selector)?;
        let cmd = WebDriverCommand::GetElementAttribute(e, attribute.to_string());
        match self.issue_wd_cmd(cmd)? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up a DOM [property] for the element matching `selector`.
    ///
    /// `selector` should be a CSS selector. `Ok(None)` is returned if the element is not found, or
    /// it does not have the given property.
    ///
    /// [property]: https://www.ecma-international.org/ecma-262/5.1/#sec-8.12.1
    pub fn lookup_prop(&self,
                       selector: &str,
                       prop: &str)
                       -> Result<Option<String>, error::CmdError> {
        let e = self.lookup(selector)?;
        let cmd = WebDriverCommand::GetElementProperty(e, prop.to_string());
        match self.issue_wd_cmd(cmd)? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up the text contents of a node matching the given CSS selector.
    ///
    /// `Ok(None)` is returned if the element was not found.
    pub fn lookup_text(&self, selector: &str) -> Result<Option<String>, error::CmdError> {
        let e = self.lookup(selector)?;
        match self.issue_wd_cmd(WebDriverCommand::GetElementText(e))? {
            Json::String(v) => Ok(Some(v)),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up the HTML contents of a node matching the given CSS selector.
    ///
    /// `Ok(None)` is returned if the element was not found. `inner` dictates whether the wrapping
    /// node's HTML is excluded or not. For example, take the HTML:
    ///
    /// ```html
    /// <div id="foo"><hr /></div>
    /// ```
    ///
    /// With `inner = true`, `<hr />` would be returned. With `inner = false`,
    /// `<div id="foo"><hr /></div>` would be returned instead.
    pub fn lookup_html(&self,
                       selector: &str,
                       inner: bool)
                       -> Result<Option<String>, error::CmdError> {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        self.lookup_prop(selector, prop)
    }

    /// Simulate the user clicking on the element matching the given CSS selector.
    ///
    /// For convenience, `Ok(None)` is returned if the element was not found.
    ///
    /// Note that this *may* result in navigation.
    pub fn click<'a>(&'a mut self,
                     selector: &str)
                     -> Result<Option<&'a mut Self>, error::CmdError> {
        match self.lookup(selector) {
            Err(error::CmdError::NoSuchElement(_)) => Ok(None),
            Err(e) => Err(e),
            Ok(e) => {
                let r = self.issue_wd_cmd(WebDriverCommand::ElementClick(e))?;
                if r.is_null() {
                    Ok(Some(self))
                } else if r.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                    // geckodriver returns {} :(
                    Ok(Some(self))
                } else {
                    Err(error::CmdError::NotW3C(r))
                }
            }
        }
    }

    /// Follow the `href` target of the element matching the given selector *without* causing a
    /// click interaction.
    ///
    /// For convenience, `Ok(None)` is returned if the element was not found, or if it does not
    /// have an `href` attribute.
    pub fn follow_link_nojs<'a>(&'a mut self,
                                selector: &str)
                                -> Result<Option<&'a mut Self>, error::CmdError> {
        if let Some(url) = self.find_link(selector)? {
            self.goto(&format!("{}", url))?;
            Ok(Some(self))
        } else {
            Ok(None)
        }
    }

    /// Wait for the given function to return `true` before proceeding.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub fn wait_for<'a, F>(&'a mut self, mut is_ready: F) -> &'a mut Self
        where F: FnMut(&Client) -> bool
    {
        while !is_ready(self) {
            use std::thread;
            thread::yield_now();
        }
        self
    }

    /// Wait for the page to navigate to a new URL before proceeding.
    ///
    /// If the `current` URL is not provided, `self.current_url()` will be used. Note however that
    /// this introduces a race condition: the browser could finish navigating *before* we call
    /// `current_url()`, which would lead to an eternal wait.
    pub fn wait_for_navigation<'a>(&'a mut self,
                                   current: Option<hyper::Url>)
                                   -> Result<&'a mut Self, error::CmdError> {
        let current = if current.is_none() {
            self.current_url()?
        } else {
            current.unwrap()
        };
        let mut err = None;

        self.wait_for(|c| match c.current_url() {
                          Err(e) => {
                              err = Some(e);
                              true
                          }
                          Ok(ref url) if url == &current => false,
                          Ok(_) => true,
                      });

        if let Some(e) = err { Err(e) } else { Ok(self) }
    }

    /// Locate a form on the page.
    ///
    /// Through the returned `Form`, HTML forms can be filled out and submitted.
    pub fn form<'a>(&'a mut self, selector: &str) -> Result<Form<'a>, error::CmdError> {
        let form = self.lookup(selector)?;
        Ok(Form { c: self, f: form })
    }

    // helpers

    /// Find the URL pointed to by a link matching the given CSS selector.
    fn find_link(&self, selector: &str) -> Result<Option<hyper::Url>, error::CmdError> {
        match self.lookup_attr(selector, "href") {
            Err(error::CmdError::NoSuchElement(_)) => Ok(None),
            Ok(Some(href)) => {
                let url = self.current_url()?;
                Ok(Some(url.join(&href)?))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Look up an element on the page given a CSS selector.
    fn lookup(&self, selector: &str) -> Result<webdriver::common::WebElement, error::CmdError> {
        let locator = Self::mklocator(selector);
        Self::parse_lookup(self.issue_wd_cmd(WebDriverCommand::FindElement(locator)))
    }

    /// Make a WebDriver locator for the given CSS selector.
    ///
    /// See https://www.w3.org/TR/webdriver/#element-retrieval.
    fn mklocator(selector: &str) -> webdriver::command::LocatorParameters {
        webdriver::command::LocatorParameters {
            using: webdriver::common::LocatorStrategy::CSSSelector,
            value: selector.to_string(),
        }
    }

    /// Extract the `WebElement` from a `FindElement` or `FindElementElement` command.
    fn parse_lookup(res: Result<Json, error::CmdError>)
                    -> Result<webdriver::common::WebElement, error::CmdError> {
        let res = res?;
        if !res.is_object() {
            return Err(error::CmdError::NotW3C(res));
        }

        let mut res = res.into_object().unwrap();
        if !res.contains_key(ELEMENT_KEY) {
            return Err(error::CmdError::NotW3C(Json::Object(res)));
        }

        match res.remove(ELEMENT_KEY) {
            Some(Json::String(wei)) => {
                return Ok(webdriver::common::WebElement::new(wei));
            }
            Some(v) => {
                res.insert(ELEMENT_KEY.to_string(), v);
            }
            None => {}
        }

        Err(error::CmdError::NotW3C(Json::Object(res)))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if self.session.is_some() {
            self.issue_wd_cmd(WebDriverCommand::DeleteSession).unwrap();
        }
    }
}

impl<'a> Form<'a> {
    /// Set the `value` of the given `field` in this form.
    pub fn set_by_name<'s>(&'s mut self,
                           field: &str,
                           value: &str)
                           -> Result<&'s mut Self, error::CmdError> {
        let locator = Client::mklocator(&format!("input[name='{}']", field));
        let locator = WebDriverCommand::FindElementElement(self.f.clone(), locator);
        let res = self.c.issue_wd_cmd(locator);
        let field = Client::parse_lookup(res)?;

        use rustc_serialize::json::ToJson;
        let args = vec![field.to_json(), Json::String(value.to_string())];
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "arguments[0].value = arguments[1]".to_string(),
            args: webdriver::common::Nullable::Value(args),
        };

        let res = self.c.issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))?;

        if res.is_null() {
            Ok(self)
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form using the first available submit button.
    ///
    /// `false` is returned if no submit button was not found.
    pub fn submit(self) -> Result<(), error::CmdError> {
        self.submit_using("input[type=submit],button[type=submit]")
    }

    /// Submit this form using the button matched by the given CSS selector.
    ///
    /// `false` is returned if a matching button was not found.
    pub fn submit_with(self, button: &str) -> Result<(), error::CmdError> {
        let locator = Client::mklocator(button);
        let locator = WebDriverCommand::FindElementElement(self.f, locator);
        let res = self.c.issue_wd_cmd(locator);

        let submit = Client::parse_lookup(res)?;
        let res = self.c.issue_wd_cmd(WebDriverCommand::ElementClick(submit))?;

        if res.is_null() {
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form using the form submit button with the given label (case-insensitive).
    ///
    /// `false` is returned if a matching button was not found.
    pub fn submit_using(self, button_label: &str) -> Result<(), error::CmdError> {
        let escaped = button_label.replace('\\', "\\\\").replace('"', "\\\"");
        self.submit_with(&format!("input[type=submit][value=\"{}\" i],\
                                  button[type=submit][value=\"{}\" i]",
                                  escaped,
                                  escaped))
    }

    /// Submit this form directly, without clicking any buttons.
    ///
    /// This can be useful to bypass forms that perform various magic when the submit button is
    /// clicked, or that hijack click events altogether (yes, I'm looking at you online
    /// advertisement code).
    ///
    /// Note that since no button is actually clicked, the `name=value` pair for the submit button
    /// will not be submitted. This can be circumvented by using `submit_sneaky` instead.
    pub fn submit_direct(self) -> Result<(), error::CmdError> {
        use rustc_serialize::json::ToJson;
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "arguments[0].submit()".to_string(),
            args: webdriver::common::Nullable::Value(vec![self.f.clone().to_json()]),
        };

        let res = self.c.issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))?;

        // unfortunately implementations seem to sometimes return very eagerly
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_millis(500));

        if res.is_null() {
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form directly, without clicking any buttons, and with an extra field.
    ///
    /// Like `submit_direct`, this method will submit this form without clicking a submit button.
    /// However, it will *also* inject a hidden input element on the page that carries the given
    /// `field=value` mapping. This allows you to emulate the form data as it would have been *if*
    /// the submit button was indeed clicked.
    pub fn submit_sneaky(self, field: &str, value: &str) -> Result<(), error::CmdError> {
        use rustc_serialize::json::ToJson;
        let args = vec![self.f.clone().to_json(),
                        Json::String(field.to_string()),
                        Json::String(value.to_string())];
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "\
                var h = document.createElement('input');\
                h.setAttribute('type', 'hidden');\
                h.setAttribute('name', arguments[1]);\
                h.value = arguments[2];\
                arguments[0].appendChild(h)"
                    .to_string(),
            args: webdriver::common::Nullable::Value(args),
        };

        let res = self.c.issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))?;
        if !res.is_null() {
            return Err(error::CmdError::NotW3C(res));
        }

        self.submit_direct()
    }
}
