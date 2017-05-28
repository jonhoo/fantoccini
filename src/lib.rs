extern crate rustc_serialize;
extern crate webdriver;
extern crate hyper;

use webdriver::command::WebDriverCommand;
use webdriver::error::WebDriverError;
use rustc_serialize::json::Json;
use std::io::prelude::*;
use std::io;

const WEB_ELEMENT_IDENTIFIER: &str = "element-6066-11e4-a52e-4f735466cecf";

pub struct Client {
    c: hyper::Client,
    wdb: hyper::Url,
    session: Option<String>,
}

pub struct Form<'a> {
    c: &'a mut Client,
    f: webdriver::common::WebElement,
}

impl Client {
    pub fn new<U: hyper::client::IntoUrl>(webdriver: U) -> Result<Self, ()> {
        let c = hyper::Client::new();
        let wdb = webdriver.into_url().map_err(|_| ())?;
        let mut c = Client {
            c,
            wdb,
            session: None,
        };

        let mut cap = webdriver::capabilities::Capabilities::new();
        cap.insert("pageLoadStrategy".to_string(),
                   Json::String("normal".to_string()));

        // https://www.w3.org/TR/webdriver/#capabilities
        let session_config = webdriver::capabilities::SpecNewSessionParameters {
            alwaysMatch: cap,
            firstMatch: vec![],
        };

        let spec = webdriver::command::NewSessionParameters::Spec(session_config);
        let mut res = c.issue_wd_cmd(WebDriverCommand::NewSession(spec)).unwrap();

        // TODO: not all impls are w3c compatible
        // See https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L189
        // and https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L200
        c.session = Some(res.into_object()
                             .unwrap()
                             .remove("sessionId")
                             .unwrap()
                             .as_string()
                             .unwrap()
                             .to_string());

        Ok(c)
    }

    fn endpoint_for(&self,
                    cmd: &WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>,
                    eid: Option<&str>)
                    -> Result<hyper::Url, hyper::error::ParseError> {
        if let WebDriverCommand::NewSession(..) = *cmd {
            return self.wdb.join("/session");
        }
        if let WebDriverCommand::DeleteSession = *cmd {
            return self.wdb
                .join(&format!("/session/{}", self.session.as_ref().unwrap()));
        }

        let base = self.wdb
            .join(&format!("/session/{}/", self.session.as_ref().unwrap()))?;
        match *cmd {
            WebDriverCommand::NewSession(..) => unreachable!(),
            WebDriverCommand::DeleteSession => unreachable!(),
            WebDriverCommand::Get(..) |
            WebDriverCommand::GetCurrentUrl => base.join("url"),
            WebDriverCommand::GetPageSource => base.join("source"),
            WebDriverCommand::FindElement(..) => base.join("element"),
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

    fn issue_wd_cmd(&self,
                    cmd: WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>)
                    -> Result<Json, WebDriverError> {
        use webdriver::command;
        use rustc_serialize::json::ToJson;

        // https://www.w3.org/TR/webdriver/#list-of-endpoints
        let mut res: hyper::client::response::Response =
            match cmd {
                    // endpoints with parameters
                    WebDriverCommand::NewSession(command::NewSessionParameters::Spec(ref conf)) => {
                        // https://www.w3.org/TR/webdriver/#dfn-new-session
                        let json = format!("{}", conf.to_json());
                        let json = json.as_bytes();

                        let url = self.endpoint_for(&cmd, None);
                        let req = self.c.post(url.unwrap());
                        let req = req.body(hyper::client::Body::BufBody(json, json.len()));
                        req.send()
                    }
                    WebDriverCommand::Get(ref params) => {
                        // https://www.w3.org/TR/webdriver/#dfn-go
                        let json = format!("{}", params.to_json());
                        let json = json.as_bytes();

                        let url = self.endpoint_for(&cmd, None);
                        let req = self.c.post(url.unwrap());
                        let req = req.body(hyper::client::Body::BufBody(json, json.len()));
                        req.send()
                    }
                    WebDriverCommand::FindElement(ref loc) |
                    WebDriverCommand::FindElementElement(_, ref loc) => {
                        let json = format!("{}", loc.to_json());
                        let json = json.as_bytes();

                        let url = self.endpoint_for(&cmd, None);
                        let req = self.c.post(url.unwrap());
                        let req = req.body(hyper::client::Body::BufBody(json, json.len()));
                        req.send()
                    }
                    WebDriverCommand::ExecuteScript(ref script) => {
                        let json = format!("{}", script.to_json());
                        let json = json.as_bytes();

                        let url = self.endpoint_for(&cmd, None);
                        let req = self.c.post(url.unwrap());
                        let req = req.body(hyper::client::Body::BufBody(json, json.len()));
                        req.send()
                    }
                    WebDriverCommand::ElementSendKeys(_, ref keys) => {
                        let json = format!("{}", keys.to_json());
                        let json = json.as_bytes();

                        let url = self.endpoint_for(&cmd, None);
                        let req = self.c.post(url.unwrap());
                        let req = req.body(hyper::client::Body::BufBody(json, json.len()));
                        req.send()
                    }

                    // various parameter-less get endpoints:
                    WebDriverCommand::GetCurrentUrl |
                    WebDriverCommand::GetPageSource |
                    WebDriverCommand::GetElementAttribute(..) |
                    WebDriverCommand::GetElementProperty(..) |
                    WebDriverCommand::GetElementText(..) => {
                        let url = self.endpoint_for(&cmd, None);
                        self.c.get(url.unwrap()).send()
                    }

                    // parameter-less, but with special verbs:
                    WebDriverCommand::ElementClick(..) => {
                        let url = self.endpoint_for(&cmd, None);
                        let req = self.c.post(url.unwrap());
                        let json = "{}".as_bytes();
                        let req = req.body(hyper::client::Body::BufBody(json, json.len()));
                        let res = req.send();

                        // unfortunately implementations seem to sometimes return very eagerly
                        use std::thread;
                        use std::time::Duration;
                        thread::sleep(Duration::from_millis(500));

                        res
                    }
                    WebDriverCommand::DeleteSession => {
                        // https://www.w3.org/TR/webdriver/#dfn-delete-session
                        let url = self.endpoint_for(&cmd, None);
                        self.c.delete(url.unwrap()).send()
                    }

                    _ => unimplemented!(),
                }
                .map_err(|e| WebDriverError::from(Box::new(e) as Box<_>))?;

        use hyper::status::StatusCode;
        use webdriver::error::ErrorStatus;
        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();

        let mut body = match Json::from_str(&body) {
            Ok(res) => res.into_object().unwrap().remove("value").unwrap(),
            Err(_) if res.status == StatusCode::NotFound => {
                panic!("{}", body);
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        };

        if res.status.is_success() {
            return Ok(body);
        }

        // https://www.w3.org/TR/webdriver/#handling-errors
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
        Err(WebDriverError::new(error, message))
    }

    pub fn goto(&mut self, url: &str) {
        let url = self.current_url().join(url).unwrap();
        self.issue_wd_cmd(WebDriverCommand::Get(webdriver::command::GetParameters {
                                                    url: url.into_string(),
                                                }))
            .unwrap();
    }

    pub fn current_url(&self) -> hyper::Url {
        hyper::Url::parse(self.issue_wd_cmd(WebDriverCommand::GetCurrentUrl)
                              .unwrap()
                              .as_string()
                              .unwrap())
                .unwrap()
    }

    pub fn source(&self) -> String {
        self.issue_wd_cmd(WebDriverCommand::GetPageSource)
            .unwrap()
            .as_string()
            .unwrap()
            .to_string()
    }

    pub fn cookies(&self) -> String {
        // TODO: WebDriverCommand::GetCookies
        unimplemented!()
    }

    fn mklocator(selector: &str) -> webdriver::command::LocatorParameters {
        webdriver::command::LocatorParameters {
            using: webdriver::common::LocatorStrategy::CSSSelector,
            value: selector.to_string(),
        }
    }

    fn parseLookupResponse(res: Result<Json, WebDriverError>)
                           -> Option<webdriver::common::WebElement> {
        match res {
            Err(WebDriverError { error: webdriver::error::ErrorStatus::NoSuchElement, .. }) => None,
            Ok(Json::Object(mut o)) => {
                Some(webdriver::common::WebElement::new(o.remove(WEB_ELEMENT_IDENTIFIER)
                                                            .unwrap()
                                                            .as_string()
                                                            .unwrap()
                                                            .to_string()))
            }
            Ok(Json::Null) => None,
            Ok(e) => {
                println!("{:?}", e);
                unreachable!();
            }
            e => {
                e.unwrap();
                unreachable!();
            }
        }
    }

    fn lookup(&self, selector: &str) -> Option<webdriver::common::WebElement> {
        let locator = Self::mklocator(selector);
        Self::parseLookupResponse(self.issue_wd_cmd(WebDriverCommand::FindElement(locator)))
    }

    pub fn lookup_attr(&self, selector: &str, attribute: &str) -> Option<String> {
        self.lookup(selector).and_then(|e| {
            match self.issue_wd_cmd(WebDriverCommand::GetElementAttribute(e,
                                                                          attribute.to_string()))
                      .unwrap() {
                Json::String(v) => Some(v),
                Json::Null => None,
                _ => unreachable!(),
            }
        })
    }

    pub fn lookup_prop(&self, selector: &str, prop: &str) -> Option<String> {
        self.lookup(selector).and_then(|e| {
            match self.issue_wd_cmd(WebDriverCommand::GetElementProperty(e, prop.to_string()))
                      .unwrap() {
                Json::String(v) => Some(v),
                Json::Null => None,
                _ => unreachable!(),
            }
        })
    }

    pub fn lookup_text(&self, selector: &str) -> Option<String> {
        self.lookup(selector).map(|e| {
                                      self.issue_wd_cmd(WebDriverCommand::GetElementText(e))
                                          .unwrap()
                                          .as_string()
                                          .unwrap()
                                          .to_string()
                                  })
    }

    pub fn lookup_html(&self, selector: &str, inner: bool) -> Option<String> {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        self.lookup_prop(selector, prop)
    }

    pub fn click(&mut self, selector: &str) {
        let e = self.lookup(selector).unwrap();
        self.issue_wd_cmd(WebDriverCommand::ElementClick(e))
            .unwrap()
            .is_null();
    }

    fn find_link(&mut self, selector: &str) -> Option<hyper::Url> {
        self.lookup_attr(selector, "href").and_then(|href| {
                                                        let url = self.current_url();
                                                        url.join(&href).ok()
                                                    })
    }

    pub fn follow_link_nojs(&mut self, selector: &str) {
        if let Some(url) = self.find_link(selector) {
            self.goto(&format!("{}", url));
        }
    }

    pub fn download<W: Write>(&mut self, selector: &str, mut writer: W) -> io::Result<usize> {
        // TODO: self.find_link
        // TODO: WebDriverCommand::GetCookies
        // TODO: hyper::Client::get
        // let mut headers = Headers::new();
        // // if you received cookies in the server response then send the same ones back
        // if let Some(&SetCookie(ref content)) = server_response.headers.get() {
        //    headers.set(Cookie(content.clone()));
        // }
        //
        // hyper_client.request(Method::Get, url)
        //     .headers(headers)
        //     .send();
        unimplemented!()
    }

    pub fn form<'a>(&'a mut self, selector: &str) -> Option<Form<'a>> {
        self.lookup(selector)
            .map(move |form| Form { c: self, f: form })
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
    pub fn set_by_name(&mut self, field: &str, val: &str) {
        let locator = Client::mklocator(&format!("input[name='{}']", field));
        let res = self.c
            .issue_wd_cmd(WebDriverCommand::FindElementElement(self.f.clone(), locator));
        let field = Client::parseLookupResponse(res).unwrap();

        use rustc_serialize::json::ToJson;
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "arguments[0].value = arguments[1]".to_string(),
            args: webdriver::common::Nullable::Value(vec![field.to_json(),
                                                          Json::String(val.to_string())]),
        };
        self.c
            .issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            .unwrap()
            .is_null();
    }

    pub fn submit(mut self) {
        self.submit_using("input[type=submit],button[type=submit]")
    }

    pub fn submit_direct(mut self) {
        use rustc_serialize::json::ToJson;
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "arguments[0].submit()".to_string(),
            args: webdriver::common::Nullable::Value(vec![self.f.clone().to_json()]),
        };
        self.c
            .issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            .unwrap()
            .is_null();

        // unfortunately implementations seem to sometimes return very eagerly
        use std::thread;
        use std::time::Duration;
        thread::sleep(Duration::from_millis(500));
    }

    pub fn submit_sneaky(mut self, field: &str, val: &str) {
        use rustc_serialize::json::ToJson;
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "\
                var h = document.createElement('input');\
                h.setAttribute('type', 'hidden');\
                h.setAttribute('name', arguments[1]);\
                h.value = arguments[2];\
                arguments[0].appendChild(h)"
                    .to_string(),
            args: webdriver::common::Nullable::Value(vec![self.f.clone().to_json(),
                                                          Json::String(field.to_string()),
                                                          Json::String(val.to_string())]),
        };
        self.c
            .issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            .unwrap()
            .is_null();
        self.submit_direct();
    }

    pub fn submit_using(mut self, button: &str) {
        let locator = Client::mklocator(button);
        let res = self.c
            .issue_wd_cmd(WebDriverCommand::FindElementElement(self.f, locator));
        let submit = Client::parseLookupResponse(res).unwrap();
        self.c
            .issue_wd_cmd(WebDriverCommand::ElementClick(submit))
            .unwrap()
            .is_null();
    }
}
