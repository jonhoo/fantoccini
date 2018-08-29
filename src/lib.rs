//! A high-level API for programmatically interacting with web pages through WebDriver.
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
//! # Examples
//!
//! These examples all assume that you have a [WebDriver compatible] process running on port 4444.
//! A quick way to get one is to run [`geckodriver`] at the command line. The code also has
//! partial support for the legacy WebDriver protocol used by `chromedriver` and `ghostdriver`.
//!
//! The examples will be using `unwrap` generously --- you should probably not do that in your
//! code, and instead deal with errors when they occur. This is particularly true for methods that
//! you *expect* might fail, such as lookups by CSS selector.
//!
//! Let's start out clicking around on Wikipedia:
//!
//! ```no_run
//! # extern crate tokio_core;
//! # extern crate futures;
//! # extern crate fantoccini;
//! # fn main() {
//! use fantoccini::{Client, Locator};
//! use futures::future::Future;
//! let mut core = tokio_core::reactor::Core::new().unwrap();
//! let c = Client::new("http://localhost:4444", &core.handle());
//! let c = core.run(c).unwrap();
//!
//! {
//!     // we want to have a reference to c so we can use it in the and_thens below
//!     let c = &c;
//!
//!     // now let's set up the sequence of steps we want the browser to take
//!     // first, go to the Wikipedia page for Foobar
//!     let f = c.goto("https://en.wikipedia.org/wiki/Foobar")
//!         .and_then(move |_| c.current_url())
//!         .and_then(move |url| {
//!             assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
//!             // click "Foo (disambiguation)"
//!             c.find(Locator::Css(".mw-disambig"))
//!         })
//!         .and_then(|e| e.click())
//!         .and_then(move |_| {
//!             // click "Foo Lake"
//!             c.find(Locator::LinkText("Foo Lake"))
//!         })
//!         .and_then(|e| e.click())
//!         .and_then(move |_| c.current_url())
//!         .and_then(|url| {
//!             assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
//!             Ok(())
//!         });
//!
//!     // and set the browser off to do those things
//!     core.run(f).unwrap();
//! }
//!
//! // drop the client to delete the browser session
//! if let Some(fin) = c.close() {
//!     // and wait for cleanup to finish
//!     core.run(fin).unwrap();
//! }
//! # }
//! ```
//!
//! How did we get to the Foobar page in the first place? We did a search!
//! Let's make the program do that for us instead:
//!
//! ```no_run
//! # extern crate tokio_core;
//! # extern crate futures;
//! # extern crate fantoccini;
//! # fn main() {
//! # use fantoccini::{Client, Locator};
//! # use futures::future::Future;
//! # let mut core = tokio_core::reactor::Core::new().unwrap();
//! # let c = Client::new("http://localhost:4444", &core.handle());
//! # let c = core.run(c).unwrap();
//! # {
//! #    let c = &c;
//! #    let f =
//! // -- snip wrapper code --
//! // go to the Wikipedia frontpage this time
//! c.goto("https://www.wikipedia.org/")
//!     .and_then(move |_| {
//!         // find the search form
//!         c.form(Locator::Css("#search-form"))
//!     })
//!     .and_then(|f| {
//!         // fill it out
//!         f.set_by_name("search", "foobar")
//!     })
//!     .and_then(|f| {
//!         // and submit it
//!         f.submit()
//!     })
//!     // we should now have ended up in the rigth place
//!     .and_then(move |_| c.current_url())
//!     .and_then(|url| {
//!         assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
//!         Ok(())
//!     })
//! // -- snip wrapper code --
//! #    ;
//! #    core.run(f).unwrap();
//! # }
//! # if let Some(fin) = c.close() {
//! #     core.run(fin).unwrap();
//! # }
//! # }
//! ```
//!
//! What if we want to download a raw file? Fantoccini has you covered:
//!
//! ```no_run
//! # extern crate tokio_core;
//! # extern crate futures;
//! # extern crate fantoccini;
//! # fn main() {
//! # use fantoccini::{Client, Locator};
//! # use futures::future::Future;
//! # let mut core = tokio_core::reactor::Core::new().unwrap();
//! # let c = Client::new("http://localhost:4444", &core.handle());
//! # let c = core.run(c).unwrap();
//! # {
//! #    let c = &c;
//! #    let f =
//! // -- snip wrapper code --
//! // go back to the frontpage
//! c.goto("https://www.wikipedia.org/")
//!     .and_then(move |_| {
//!         // find the source for the Wikipedia globe
//!         c.find(Locator::Css("img.central-featured-logo"))
//!     })
//!     .and_then(|img| {
//!         img.attr("src")
//!             .map(|src| src.expect("image should have a src"))
//!     })
//!     .and_then(move |src| {
//!         // now build a raw HTTP client request (which also has all current cookies)
//!         c.raw_client_for(fantoccini::Method::Get, &src)
//!     })
//!     .and_then(|raw| {
//!         use futures::Stream;
//!         // we then read out the image bytes
//!         raw.body().map_err(fantoccini::error::CmdError::from).fold(
//!             Vec::new(),
//!             |mut pixels, chunk| {
//!                 pixels.extend(&*chunk);
//!                 futures::future::ok::<Vec<u8>, fantoccini::error::CmdError>(pixels)
//!             },
//!         )
//!     })
//!     .and_then(|pixels| {
//!         // and voilla, we now have the bytes for the Wikipedia logo!
//!         assert!(pixels.len() > 0);
//!         println!("Wikipedia logo is {}b", pixels.len());
//!         Ok(())
//!     })
//! // -- snip wrapper code --
//! #    ;
//! #    core.run(f).unwrap();
//! # }
//! # if let Some(fin) = c.close() {
//! #     core.run(fin).unwrap();
//! # }
//! # }
//! ```
//!
//! [WebDriver protocol]: https://www.w3.org/TR/webdriver/
//! [CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
//! [powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
//! [operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
//! [WebDriver compatible]: https://github.com/Fyrd/caniuse/issues/2757#issuecomment-304529217
//! [`geckodriver`]: https://github.com/mozilla/geckodriver
#![deny(missing_docs)]

extern crate base64;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate rustc_serialize;
extern crate tokio;
extern crate url;
extern crate mime;
extern crate webdriver;

use futures::{future, Future, Stream};
use rustc_serialize::json::Json;
use std::sync::{Arc, RwLock};
use hyper::header::HeaderValue;
use webdriver::command::WebDriverCommand;
use webdriver::common::ELEMENT_KEY;
use webdriver::error::ErrorStatus;
use webdriver::error::WebDriverError;

pub use hyper::Method;

/// Error types.
pub mod error;

/// An element locator.
///
/// See <https://www.w3.org/TR/webdriver/#element-retrieval>.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum Locator<'a> {
    /// Find an element matching the given CSS selector.
    Css(&'a str),

    /// Find an element using the given `id`
    Id(&'a str),

    /// Find a link element with the given link text.
    ///
    /// The text matching is exact.
    LinkText(&'a str),

    /// Find an element using the given XPath expression.
    XPath(&'a str),
}

impl<'a> Into<webdriver::command::LocatorParameters> for Locator<'a> {
    fn into(self) -> webdriver::command::LocatorParameters {
        match self {
            Locator::Css(s) => webdriver::command::LocatorParameters {
                using: webdriver::common::LocatorStrategy::CSSSelector,
                value: s.to_string(),
            },
            Locator::Id(s) => webdriver::command::LocatorParameters {
                using: webdriver::common::LocatorStrategy::XPath,
                value: format!("//*[@id=\"{}\"]", s),
            },
            Locator::XPath(s) => webdriver::command::LocatorParameters {
                using: webdriver::common::LocatorStrategy::XPath,
                value: s.to_string(),
            },
            Locator::LinkText(s) => webdriver::command::LocatorParameters {
                using: webdriver::common::LocatorStrategy::LinkText,
                value: s.to_string(),
            },
        }
    }
}

type Cmd = WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>;

/// State held by a `Client`
struct Inner {
    c: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
    handle: tokio::runtime::TaskExecutor,
    wdb: url::Url,
    session: RwLock<Option<String>>,
    legacy: bool,
    ua: RwLock<Option<String>>,
}

impl Inner {
    fn shutdown(&self) -> Option<impl Future<Item = (), Error = hyper::Error>> {
        if self.session.read().unwrap().is_some() {
            let url = {
                let s = self.session.read().unwrap();
                self.wdb
                    .join(&format!("session/{}", s.as_ref().unwrap()))
                    .unwrap()
            };
            *self.session.write().unwrap() = None;

            Some(
                self.c
                    .request(hyper::Request::delete(
                        url.to_string(),
                    ).body(hyper::Body::empty()).unwrap())
                    .map(move |_| ()),
            )
        } else {
            None
        }
    }
}

impl Drop for Inner {
    // NOTE: we must implement Drop for Inner, *not* for Client, since Client is dropped often
    fn drop(&mut self) {
        if let Some(end) = self.shutdown() {
            self.handle.spawn(end.map_err(|_| ()));
        }
    }
}

/// A WebDriver client tied to a single browser session.
pub struct Client(Arc<Inner>);

/// A single element on the current page.
pub struct Element {
    c: Client,
    e: webdriver::common::WebElement,
}

/// An HTML form on the current page.
pub struct Form {
    c: Client,
    f: webdriver::common::WebElement,
}

impl Client {
    fn init(
        mut self,
        params: webdriver::command::NewSessionParameters,
    ) -> impl Future<Item = Self, Error = error::NewSessionError> + 'static {
        if let webdriver::command::NewSessionParameters::Legacy(..) = params {
            Arc::get_mut(&mut self.0)
                .expect("during legacy init there should be only one Client instance")
                .legacy = true;
        }

        // Create a new session for this client
        // https://www.w3.org/TR/webdriver/#dfn-new-session
        self.issue_wd_cmd(WebDriverCommand::NewSession(params))
            .then(move |r| match r {
                Ok((this, Json::Object(mut v))) => {
                    // TODO: not all impls are w3c compatible
                    // See https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L189
                    // and https://github.com/SeleniumHQ/selenium/blob/242d64ca4cd3523489ac1e58703fd7acd4f10c5a/py/selenium/webdriver/remote/webdriver.py#L200
                    if let Some(session_id) = v.remove("sessionId") {
                        if let Some(session_id) = session_id.as_string() {
                            *this.0.session.write().unwrap() = Some(session_id.to_string());
                            return Ok(this);
                        }
                        v.insert("sessionId".to_string(), session_id);
                        Err(error::NewSessionError::NotW3C(Json::Object(v)))
                    } else {
                        Err(error::NewSessionError::NotW3C(Json::Object(v)))
                    }
                }
                Ok((_, v)) | Err(error::CmdError::NotW3C(v)) => {
                    Err(error::NewSessionError::NotW3C(v))
                }
                Err(error::CmdError::Failed(e)) => Err(error::NewSessionError::Failed(e)),
                Err(error::CmdError::Lost(e)) => Err(error::NewSessionError::Lost(e)),
                Err(error::CmdError::NotJson(v)) => {
                    Err(error::NewSessionError::NotW3C(Json::String(v)))
                }
                Err(error::CmdError::Standard(
                    e @ WebDriverError {
                        error: ErrorStatus::SessionNotCreated,
                        ..
                    },
                )) => Err(error::NewSessionError::SessionNotCreated(e)),
                Err(e) => {
                    panic!("unexpected webdriver error; {}", e);
                }
            })
    }

    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    ///
    /// Calls `with_capabilities` with an empty capabilities list.
    #[cfg_attr(feature = "cargo-clippy", allow(new_ret_no_self))]
    pub fn new(
        webdriver: &str,
        handle: tokio::runtime::TaskExecutor,
    ) -> impl Future<Item = Self, Error = error::NewSessionError> + 'static {
        Self::with_capabilities(
            webdriver,
            webdriver::capabilities::Capabilities::new(),
            handle,
        )
    }

    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    ///
    /// The given capabilities will be requested in `alwaysMatch` or `desiredCapabilities`
    /// depending on the protocol version supported by the server.
    ///
    /// Returns a future that resolves to a handle for issuing additional WebDriver tasks.
    ///
    /// Note that most callers should explicitly call `Client::close`, and wait for the returned
    /// future before exiting. Not doing so may result in the WebDriver session not being cleanly
    /// closed, which is particularly important for some drivers, such as geckodriver, where
    /// multiple simulatenous sessions are not supported. If `close` is not explicitly called, a
    /// session close request will be spawned on the given `handle` when the last instance of this
    /// `Client` is dropped.
    pub fn with_capabilities(
        webdriver: &str,
        mut cap: webdriver::capabilities::Capabilities,
        handle: tokio::runtime::TaskExecutor,
    ) -> impl Future<Item = Self, Error = error::NewSessionError> + 'static {
        // Where is the WebDriver server?
        let wdb = match webdriver.parse::<url::Url>() {
            Ok(wdb) => wdb,
            Err(e) => {
                return future::Either::B(future::err(error::NewSessionError::BadWebdriverUrl(e)));
            }
        };

        // We want a tls-enabled client
        let client = hyper::Client::builder()
            .executor(handle.clone())
            .build(hyper_tls::HttpsConnector::new(4).unwrap());

        // Set up our WebDriver client
        let c = Client(Arc::new(Inner {
            c: client.clone(),
            handle: handle,
            wdb: wdb.clone(),
            session: RwLock::new(None),
            legacy: false,
            ua: RwLock::new(None),
        }));

        // Required capabilities
        // https://www.w3.org/TR/webdriver/#capabilities
        //  - we want the browser to wait for the page to load
        cap.insert(
            "pageLoadStrategy".to_string(),
            Json::String("normal".to_string()),
        );

        let session_config = webdriver::capabilities::SpecNewSessionParameters {
            alwaysMatch: cap.clone(),
            firstMatch: vec![],
        };
        let spec = webdriver::command::NewSessionParameters::Spec(session_config);

        let f = c.dup().init(spec).or_else(move |e| {
            match e {
                error::NewSessionError::NotW3C(json) => {
                    let mut legacy = false;
                    match json {
                        Json::String(ref err) if err.starts_with("Missing Command Parameter") => {
                            // ghostdriver
                            legacy = true;
                        }
                        Json::Object(ref err) => {
                            legacy = err.get("message")
                                .and_then(|m| m.as_string())
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

                    if legacy {
                        // we're dealing with an implementation that only supports the legacy
                        // WebDriver protocol:
                        // https://github.com/SeleniumHQ/selenium/wiki/JsonWireProtocol
                        let session_config = webdriver::capabilities::LegacyNewSessionParameters {
                            desired: cap,
                            required: webdriver::capabilities::Capabilities::new(),
                        };
                        let spec = webdriver::command::NewSessionParameters::Legacy(session_config);

                        // try a new client
                        future::Either::A(c.init(spec))
                    } else {
                        future::Either::B(future::err(error::NewSessionError::NotW3C(json)))
                    }
                }
                e => future::Either::B(future::err(e)),
            }
        });

        future::Either::A(f)
    }

    /// Get the session ID assigned by the WebDriver server to this client.
    pub fn session_id(&self) -> String {
        self.0.session.read().unwrap().as_ref().unwrap().to_string()
    }

    fn dup(&self) -> Self {
        Client(Arc::clone(&self.0))
    }

    /// Set the User Agent string to use for all subsequent requests.
    pub fn set_ua<S: Into<String>>(&mut self, ua: S) {
        *self.0.ua.write().unwrap() = Some(ua.into());
    }

    /// Helper for determining what URL endpoint to use for various requests.
    ///
    /// This mapping is essentially that of https://www.w3.org/TR/webdriver/#list-of-endpoints.
    fn endpoint_for(&self, cmd: &Cmd) -> Result<url::Url, url::ParseError> {
        if let WebDriverCommand::NewSession(..) = *cmd {
            return self.0.wdb.join("session");
        }

        let base = {
            let session = self.0.session.read().unwrap();
            self.0
                .wdb
                .join(&format!("session/{}/", session.as_ref().unwrap()))?
        };
        match *cmd {
            WebDriverCommand::NewSession(..) => unreachable!(),
            WebDriverCommand::DeleteSession => unreachable!(),
            WebDriverCommand::Get(..) | WebDriverCommand::GetCurrentUrl => base.join("url"),
            WebDriverCommand::GoBack => base.join("back"),
            WebDriverCommand::Refresh => base.join("refresh"),
            WebDriverCommand::GetPageSource => base.join("source"),
            WebDriverCommand::FindElement(..) => base.join("element"),
            WebDriverCommand::FindElements(..) => base.join("elements"),
            WebDriverCommand::GetCookies => base.join("cookie"),
            WebDriverCommand::ExecuteScript(..) if self.0.legacy => base.join("execute"),
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
            WebDriverCommand::FindElementElements(ref p, _) => {
                base.join(&format!("element/{}/elements", p.id))
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
            WebDriverCommand::SetWindowRect(..) => base.join("window/rect"),
            WebDriverCommand::GetWindowRect => base.join("window/rect"),
            WebDriverCommand::TakeScreenshot => base.join("screenshot"),
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
    fn issue_wd_cmd(
        self,
        cmd: WebDriverCommand<webdriver::command::VoidWebDriverExtensionCommand>,
    ) -> impl Future<Item = (Self, Json), Error = error::CmdError> {
        use rustc_serialize::json::ToJson;
        use webdriver::command;

        // most actions are just get requests with not parameters
        let url = match self.endpoint_for(&cmd) {
            Ok(url) => url,
            Err(e) => return future::Either::B(future::err(error::CmdError::from(e))),
        };
        let mut method = Method::GET;
        let mut body = None;

        // but some are special
        match cmd {
            WebDriverCommand::NewSession(command::NewSessionParameters::Spec(ref conf)) => {
                body = Some(format!("{}", conf.to_json()));
                method = Method::POST;
            }
            WebDriverCommand::NewSession(command::NewSessionParameters::Legacy(ref conf)) => {
                body = Some(format!("{}", conf.to_json()));
                method = Method::POST;
            }
            WebDriverCommand::Get(ref params) => {
                body = Some(format!("{}", params.to_json()));
                method = Method::POST;
            }
            WebDriverCommand::FindElement(ref loc)
            | WebDriverCommand::FindElements(ref loc)
            | WebDriverCommand::FindElementElement(_, ref loc)
            | WebDriverCommand::FindElementElements(_, ref loc) => {
                body = Some(format!("{}", loc.to_json()));
                method = Method::POST;
            }
            WebDriverCommand::ExecuteScript(ref script) => {
                body = Some(format!("{}", script.to_json()));
                method = Method::POST;
            }
            WebDriverCommand::ElementSendKeys(_, ref keys) => {
                body = Some(format!("{}", keys.to_json()));
                method = Method::POST;
            }
            WebDriverCommand::ElementClick(..)
            | WebDriverCommand::GoBack
            | WebDriverCommand::Refresh => {
                body = Some("{}".to_string());
                method = Method::POST;
            }
            WebDriverCommand::SetWindowRect(ref params) => {
                body = Some(format!("{}", params.to_json()));
                method = Method::POST;
            }
            _ => {}
        }

        // issue the command to the webdriver server
        let mut req = hyper::Request::builder()
            .uri(url.to_string())
            .method(method)
            .body(hyper::Body::empty())
            .unwrap();
        if let Some(ref s) = *self.0.ua.read().unwrap() {
            req.headers_mut().insert(
                hyper::header::USER_AGENT,
                HeaderValue::from_str(s).unwrap(),
            );
        }
        // because https://github.com/hyperium/hyper/pull/727
        // TODO
        // if !url.username().is_empty() || url.password().is_some() {
        //     req.headers_mut()
        //         .insert(hyper::header::AUTHORIZATION, 
        //             hyper::header::BASIC {
        //                 username: url.username().to_string(),
        //                 password: url.password().map(|pwd| pwd.to_string()),
        //             }));
        // }
        if let Some(ref body) = body {
            req.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            req.headers_mut().insert(
                hyper::header::CONTENT_LENGTH,
                HeaderValue::from_str(&format!("{}", body.len() as u64)).unwrap(),
            );
            *req.body_mut() = hyper::Body::from(body.clone());
        }

        let req = self.0.c.request(req);
        let f = req.map_err(error::CmdError::from)
            .and_then(move |res| {

                // keep track of result status (.body() consumes self -- ugh)
                let status = res.status();

                // check that the server sent us json
                let ctype = {
                    res.headers().get(hyper::header::CONTENT_TYPE)
                        .expect("webdriver response did not have a content type")
                        .to_str()
                        .unwrap()
                        .to_string()
                };

                // What did the server send us?
                res.into_body()
                    .fold(vec![], |mut acc, chunk| -> Result<Vec<u8>, hyper::Error> {
                        acc.extend_from_slice(&chunk);
                        Ok(acc)
                    })
                    .and_then(|v| Ok(String::from_utf8(v).unwrap()))
                    .map(move |body| (self, body, ctype, status))
                    .map_err(|e| -> error::CmdError { e.into() })
            })
            .and_then(|(this, body, ctype, status)| {
                if let Ok(ctype) = ctype.parse::<mime::Mime>()
                {
                    if ctype.type_() == mime::APPLICATION && ctype.subtype() == mime::JSON
                    {
                        return Ok((this, body, status))
                    }
                }
                // nope, something else...
                Err(error::CmdError::NotJson(body))
            })
            .and_then(move |(this, body, status)| {
                let is_new_session = if let WebDriverCommand::NewSession(..) = cmd {
                    true
                } else {
                    false
                };

                let mut is_success = status.is_success();
                let mut legacy_status = 0;

                // https://www.w3.org/TR/webdriver/#dfn-send-a-response
                // NOTE: the standard specifies that even errors use the "Send a Reponse" steps
                let body = match Json::from_str(&*body)? {
                    Json::Object(mut v) => {
                        if this.0.legacy {
                            legacy_status = v["status"].as_u64().unwrap();
                            is_success = legacy_status == 0;
                        }

                        if this.0.legacy && is_new_session {
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
                    return Ok((this, body));
                }

                // https://www.w3.org/TR/webdriver/#dfn-send-an-error
                // https://www.w3.org/TR/webdriver/#handling-errors
                if !body.is_object() {
                    return Err(error::CmdError::NotW3C(body));
                }
                let mut body = body.into_object().unwrap();

                // phantomjs injects a *huge* field with the entire screen contents -- remove that
                body.remove("screen");

                let es = if this.0.legacy {
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
                    if !body.contains_key("error") || !body.contains_key("message")
                        || !body["error"].is_string()
                        || !body["message"].is_string()
                    {
                        return Err(error::CmdError::NotW3C(Json::Object(body)));
                    }

                    use hyper::StatusCode;
                    let error = body["error"].as_string().unwrap();
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
                            _ => unreachable!(),
                        },
                        StatusCode::NOT_FOUND => match error {
                            "unknown command" => ErrorStatus::UnknownCommand,
                            "no such cookie" => ErrorStatus::NoSuchCookie,
                            "invalid session id" => ErrorStatus::InvalidSessionId,
                            "no such element" => ErrorStatus::NoSuchElement,
                            _ => unreachable!(),
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
                            _ => unreachable!(),
                        },
                        StatusCode::REQUEST_TIMEOUT => match error {
                            "timeout" => ErrorStatus::Timeout,
                            "script timeout" => ErrorStatus::ScriptTimeout,
                            _ => unreachable!(),
                        },
                        StatusCode::METHOD_NOT_ALLOWED => match error {
                            "unknown method" => ErrorStatus::UnknownMethod,
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    }
                };

                let message = body["message"].as_string().unwrap().to_string();
                Err(error::CmdError::from(WebDriverError::new(es, message)))
            });

        future::Either::A(f)
    }

    /// Terminate the connection to the webservice.
    ///
    /// Normally, a shutdown of the WebDriver connection will be initiated when the last clone of a
    /// `Client` is dropped. Specifically, the shutdown request will be issued using the tokio
    /// `Handle` given when creating this `Client`. This in turn means that any errors will be
    /// dropped, and that the teardown may not even occur if the reactor does not continue being
    /// turned.
    ///
    /// This function is safe to call multiple times, but once it has been called on one instance
    /// of a `Client`, all requests to other instances of that `Client` will fail. The returned
    /// `Option` will only be true the first time `close` is called.
    ///
    /// This function may be useful in conjunction with `raw_client_for`, as it allows you to close
    /// the automated browser window while doing e.g., a large download.
    pub fn close(&self) -> Option<impl Future<Item = (), Error = hyper::Error>> {
        self.0.shutdown()
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub fn set_window_rect(
        &self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        use webdriver::common::Nullable;

        if x < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(x).into(),
                format!("Expected to be `>= 0` but was `{}`", x),
            )));
        }

        if y < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(y).into(),
                format!("Expected to be `>= 0` but was `{}`", y),
            )));
        }

        if width < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(width).into(),
                format!("Expected to be `>= 0` but was `{}`", width),
            )));
        }

        if height < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(height).into(),
                format!("Expected to be `>= 0` but was `{}`", height),
            )));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Nullable::Value(x),
            y: Nullable::Value(y),
            width: Nullable::Value(width),
            height: Nullable::Value(height),
        });

        future::Either::B(self.dup().issue_wd_cmd(cmd).map(|(this, _)| this))
    }

    /// Gets the x, y, width, and height properties of the current window.
    pub fn get_window_rect(
        &self,
    ) -> impl Future<Item = (u64, u64, u64, u64), Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::GetWindowRect)
            .and_then(|(_, v)| match v {
                Json::Object(mut obj) => {
                    let x = match obj.remove("x").and_then(|x| x.as_u64()) {
                        Some(x) => x,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    let y = match obj.remove("y").and_then(|y| y.as_u64()) {
                        Some(y) => y,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    let width = match obj.remove("width").and_then(|width| width.as_u64()) {
                        Some(width) => width,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    let height = match obj.remove("height").and_then(|height| height.as_u64()) {
                        Some(height) => height,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    Ok((x, y, width, height))
                }
                _ => Err(error::CmdError::NotW3C(v)),
            })
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub fn set_window_size(
        &self,
        width: i32,
        height: i32,
    ) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        use webdriver::common::Nullable;

        if width < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(width).into(),
                format!("Expected to be `>= 0` but was `{}`", width),
            )));
        }

        if height < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(height).into(),
                format!("Expected to be `>= 0` but was `{}`", height),
            )));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Nullable::Null,
            y: Nullable::Null,
            width: Nullable::Value(width),
            height: Nullable::Value(height),
        });

        future::Either::B(self.dup().issue_wd_cmd(cmd).map(|(this, _)| this))
    }

    /// Gets the width and height of the current window.
    pub fn get_window_size(
        &self,
    ) -> impl Future<Item = (u64, u64), Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::GetWindowRect)
            .and_then(|(_, v)| match v {
                Json::Object(mut obj) => {
                    let width = match obj.remove("width").and_then(|width| width.as_u64()) {
                        Some(width) => width,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    let height = match obj.remove("height").and_then(|height| height.as_u64()) {
                        Some(height) => height,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    Ok((width, height))
                }
                _ => Err(error::CmdError::NotW3C(v)),
            })
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub fn set_window_position(
        &self,
        x: i32,
        y: i32,
    ) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        use webdriver::common::Nullable;

        if x < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(x).into(),
                format!("Expected to be `>= 0` but was `{}`", x),
            )));
        }

        if y < 0 {
            return future::Either::A(future::err(error::CmdError::InvalidArgument(
                stringify!(y).into(),
                format!("Expected to be `>= 0` but was `{}`", y),
            )));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Nullable::Value(x),
            y: Nullable::Value(y),
            width: Nullable::Null,
            height: Nullable::Null,
        });

        future::Either::B(self.dup().issue_wd_cmd(cmd).map(|(this, _)| this))
    }

    /// Gets the x and y top-left coordinate of the current window.
    pub fn get_window_position(
        &self,
    ) -> impl Future<Item = (u64, u64), Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::GetWindowRect)
            .and_then(|(_, v)| match v {
                Json::Object(mut obj) => {
                    let x = match obj.remove("x").and_then(|x| x.as_u64()) {
                        Some(x) => x,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    let y = match obj.remove("y").and_then(|y| y.as_u64()) {
                        Some(y) => y,
                        None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    };

                    Ok((x, y))
                }
                _ => Err(error::CmdError::NotW3C(v)),
            })
    }

    /// Navigate directly to the given URL.
    pub fn goto(&self, url: &str) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        let url = url.to_owned();
        self.current_url_()
            .and_then(move |(this, base)| Ok((this, base.join(&url)?)))
            .and_then(move |(this, url)| {
                this.issue_wd_cmd(WebDriverCommand::Get(webdriver::command::GetParameters {
                    url: url.into_string(),
                }))
            })
            .map(|(this, _)| this)
    }

    fn current_url_(
        &self,
    ) -> impl Future<Item = (Self, url::Url), Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::GetCurrentUrl)
            .and_then(|(this, url)| {
                if let Some(url) = url.as_string() {
                    return Ok((this, url.parse()?));
                }

                Err(error::CmdError::NotW3C(url))
            })
    }

    /// Retrieve the currently active URL for this session.
    pub fn current_url(&self) -> impl Future<Item = url::Url, Error = error::CmdError> + 'static {
        self.current_url_().map(|(_, u)| u)
    }

    /// Get a PNG-encoded screenshot of the current page.
    pub fn screenshot(&self) -> impl Future<Item = Vec<u8>, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::TakeScreenshot)
            .and_then(|(_, src)| {
                if let Some(src) = src.as_string() {
                    return base64::decode(src).map_err(|e| error::CmdError::ImageDecodeError(e));
                }

                Err(error::CmdError::NotW3C(src))
            })
    }

    /// Get the HTML source for the current page.
    pub fn source(&self) -> impl Future<Item = String, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::GetPageSource)
            .and_then(|(_, src)| {
                if let Some(src) = src.as_string() {
                    return Ok(src.to_string());
                }

                Err(error::CmdError::NotW3C(src))
            })
    }

    /// Go back to the previous page.
    pub fn back(&self) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::GoBack)
            .map(|(this, _)| this)
    }

    /// Refresh the current previous page.
    pub fn refresh(&self) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::Refresh)
            .map(|(this, _)| this)
    }

    /// Execute the given JavaScript `script` in the current browser session.
    ///
    /// `args` is available to the script inside the `arguments` array. Since `Element` implements
    /// `ToJson`, you can also provide serialized `Element`s as arguments, and they will correctly
    /// serialize to DOM elements on the other side.
    pub fn execute(
        &self,
        script: &str,
        mut args: Vec<Json>,
    ) -> impl Future<Item = Json, Error = error::CmdError> + 'static {
        self.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: script.to_string(),
            args: webdriver::common::Nullable::Value(args),
        };

        self.dup()
            .issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            .map(|(_, v)| v)
    }

    /// Issue an HTTP request to the given `url` with all the same cookies as the current session.
    ///
    /// Calling this method is equivalent to calling `with_raw_client_for` with an empty closure.
    pub fn raw_client_for(
        &self,
        method: Method,
        url: &str,
    ) -> impl Future<Item = hyper::Response<hyper::Body>, Error = error::CmdError> + 'static {
        self.with_raw_client_for(method, url, |_| {})
    }

    /// Build and issue an HTTP request to the given `url` with all the same cookies as the current
    /// session.
    ///
    /// Before the HTTP request is issued, the given `before` closure will be called with a handle
    /// to the `Request` about to be sent.
    pub fn with_raw_client_for<F>(
        &self,
        method: Method,
        url: &str,
        before: F,
    ) -> impl Future<Item = hyper::Response<hyper::Body>, Error = error::CmdError> + 'static
    where
        F: FnOnce(&mut hyper::Request<hyper::Body>) + 'static,
    {
        let url = url.to_owned();
        // We need to do some trickiness here. GetCookies will only give us the cookies for the
        // *current* domain, whereas we want the cookies for `url`'s domain. So, we navigate to the
        // URL in question, fetch its cookies, and then navigate back. *Except* that we can't do
        // that either (what if `url` is some huge file?). So we *actually* navigate to some weird
        // url that's unlikely to exist on the target doamin, and which won't resolve into the
        // actual content, but will still give the same cookies.
        //
        // The fact that cookies can have /path and security constraints makes this even more of a
        // pain. /path in particular is tricky, because you could have a URL like:
        //
        //    example.com/download/some_identifier/ignored_filename_just_for_show
        //
        // Imagine if a cookie is set with path=/download/some_identifier. How do we get that
        // cookie without triggering a request for the (large) file? I don't know. Hence: TODO.
        self.current_url_()
            .and_then(move |(this, old_url)| {
                old_url
                    .clone()
                    .join(&url)
                    .map(move |url| (this, url))
                    .map_err(|e| e.into())
            })
            .and_then(|(this, url)| {
                url.clone()
                    .join("/please_give_me_your_cookies")
                    .map(move |cookie_url| (this, url, cookie_url))
                    .map_err(|e| e.into())
            })
            .and_then(|(this, url, cookie_url)| {
                this.goto(cookie_url.as_str()).map(|this| (this, url))
            })
            .and_then(|(this, url)| {
                this.issue_wd_cmd(WebDriverCommand::GetCookies)
                    .then(|cookies| {
                        match cookies {
                            Ok((this, cookies)) => if cookies.is_array() {
                                future::ok((this, url, cookies))
                            } else {
                                future::err(error::CmdError::NotW3C(cookies))
                            },
                            Err(e) => {
                                // TODO: go back before we return
                                // can't get a handle to this here though :(
                                //future::Either::B(
                                //    this.goto(&format!("{}", old_url))
                                //        .and_then(move |_| future::err(e)),
                                //)
                                future::err(e)
                            }
                        }
                    })
            })
            .and_then(|(this, url, cookies)| this.back().map(|this| (this, url, cookies)))
            .and_then(|(this, url, cookies)| {
                let cookies = cookies.into_array().unwrap();

                // now add all the cookies
                let mut all_ok = true;
                // let mut jar = hyper::header::Cookie::new();
                // for cookie in &cookies {
                //     if !cookie.is_object() {
                //         all_ok = false;
                //         break;
                //     }

                //     // https://w3c.github.io/webdriver/webdriver-spec.html#cookies
                //     let cookie = cookie.as_object().unwrap();
                //     if !cookie.contains_key("name") || !cookie.contains_key("value") {
                //         all_ok = false;
                //         break;
                //     }

                //     if !cookie["name"].is_string() || !cookie["value"].is_string() {
                //         all_ok = false;
                //         break;
                //     }

                //     // Note that since we're sending these cookies, all that matters is the mapping
                //     // from name to value. The other fields only matter when deciding whether to
                //     // include a cookie or not, and the driver has already decided that for us
                //     // (GetCookies is for a particular URL).
                //     jar.append(
                //         cookie["name"].as_string().unwrap().to_owned(),
                //         cookie["value"].as_string().unwrap().to_owned(),
                //     );
                // }

                if all_ok {
                    let mut req = hyper::Request::builder()
                        .uri(url.to_string())
                        .method(method)
                        .body(hyper::Body::empty())
                        .unwrap();
                    // req.headers_mut().set(jar);
                    if let Some(ref s) = *this.0.ua.read().unwrap() {
                        req.headers_mut().insert(
                            hyper::header::USER_AGENT,
                            HeaderValue::from_str(s).unwrap(),
                        );
                    }
                    before(&mut req);
                    future::Either::A(this.0.c.request(req).map_err(|e| e.into()))
                } else {
                    future::Either::B(future::err(error::CmdError::NotW3C(Json::Array(cookies))))
                }
            })
    }

    /// Find an element on the page.
    pub fn find(
        &self,
        search: Locator,
    ) -> impl Future<Item = Element, Error = error::CmdError> + 'static {
        self.by(search.into())
    }

    /// Find elements on the page.
    pub fn find_all(
        &self,
        search: Locator,
    ) -> impl Future<Item = Vec<Element>, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::FindElements(search.into()))
            .and_then(|(this, res)| {
                let array = this.parse_lookup_all(res)?;
                Ok(array.into_iter().map(|e| Element { c: this.dup(), e: e }).collect())
            })
    }

    /// Wait for the given function to return `true` before proceeding.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub fn wait_for<F>(&mut self, mut is_ready: F) -> &mut Self
    where
        F: FnMut(Client) -> bool,
    {
        while !is_ready(self.dup()) {
            use std::thread;
            thread::yield_now();
        }
        self
    }

    /// Wait for the given element to be present on the page.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub fn wait_for_find<'a>(
        &'a self,
        search: Locator<'a>,
    ) -> impl Future<Item = Element, Error = error::CmdError> + 'a {
        futures::future::loop_fn((), move |_| {
            self.by(search.into())
                .map(futures::future::Loop::Break)
                .or_else(|e| {
                    if let error::CmdError::NoSuchElement(_) = e {
                        Ok(futures::future::Loop::Continue(()))
                    } else {
                        Err(e)
                    }
                })
        })
    }

    /// Wait for the page to navigate to a new URL before proceeding.
    ///
    /// If the `current` URL is not provided, `self.current_url()` will be used. Note however that
    /// this introduces a race condition: the browser could finish navigating *before* we call
    /// `current_url()`, which would lead to an eternal wait.
    pub fn wait_for_navigation(
        self,
        current: Option<url::Url>,
    ) -> impl Future<Item = Self, Error = error::CmdError> + 'static {
        match current {
            Some(current) => future::Either::A(future::ok((self, current))),
            None => future::Either::B(self.current_url_()),
        }.and_then(|(mut this, current)| {
            let mut err = None;

            this.wait_for(|c| match c.current_url().wait() {
                Err(e) => {
                    err = Some(e);
                    true
                }
                Ok(ref url) if url == &current => false,
                Ok(_) => true,
            });

            if let Some(e) = err {
                Err(e)
            } else {
                Ok(this)
            }
        })
    }

    /// Locate a form on the page.
    ///
    /// Through the returned `Form`, HTML forms can be filled out and submitted.
    pub fn form(
        &self,
        search: Locator,
    ) -> impl Future<Item = Form, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::FindElement(search.into()))
            .and_then(|(this, res)| {
                let f = this.parse_lookup(res);
                f.map(move |f| Form { c: this, f: f })
            })
    }

    // helpers

    fn by(
        &self,
        locator: webdriver::command::LocatorParameters,
    ) -> impl Future<Item = Element, Error = error::CmdError> + 'static {
        self.dup()
            .issue_wd_cmd(WebDriverCommand::FindElement(locator))
            .and_then(|(this, res)| {
                let e = this.parse_lookup(res);
                e.map(move |e| Element { c: this, e: e })
            })
    }

    /// Extract the `WebElement` from a `FindElement` or `FindElementElement` command.
    fn parse_lookup(&self, res: Json) -> Result<webdriver::common::WebElement, error::CmdError> {
        if !res.is_object() {
            return Err(error::CmdError::NotW3C(res));
        }

        // legacy protocol uses "ELEMENT" as identifier
        let key = if self.0.legacy {
            "ELEMENT"
        } else {
            ELEMENT_KEY
        };

        let mut res = res.into_object().unwrap();
        if !res.contains_key(key) {
            return Err(error::CmdError::NotW3C(Json::Object(res)));
        }

        match res.remove(key) {
            Some(Json::String(wei)) => {
                return Ok(webdriver::common::WebElement::new(wei));
            }
            Some(v) => {
                res.insert(key.to_string(), v);
            }
            None => {}
        }

        Err(error::CmdError::NotW3C(Json::Object(res)))
    }

    /// Extract `WebElement`s from a `FindElements` or `FindElementElements` command.
    fn parse_lookup_all(&self, res: Json) -> Result<Vec<webdriver::common::WebElement>, error::CmdError> {
        if !res.is_array() {
            return Err(error::CmdError::NotW3C(res));
        }

        let mut array = Vec::new();
        for json in res.into_array().unwrap().into_iter() {
            let e = self.parse_lookup(json)?;
            array.push(e);
        }

        Ok(array)
    }

    fn fixup_elements(&self, args: &mut [Json]) {
        if self.0.legacy {
            for arg in args {
                // the serialization of WebElement uses the W3C index,
                // but legacy implementations need us to use the "ELEMENT" index
                if let Json::Object(ref mut o) = *arg {
                    if let Some(wei) = o.remove(ELEMENT_KEY) {
                        o.insert("ELEMENT".to_string(), wei);
                    }
                }
            }
        }
    }
}

impl Element {
    /// Look up an [attribute] value for this element by name.
    ///
    /// `Ok(None)` is returned if the element does not have the given attribute.
    ///
    /// [attribute]: https://dom.spec.whatwg.org/#concept-attribute
    pub fn attr(
        self,
        attribute: &str,
    ) -> impl Future<Item = Option<String>, Error = error::CmdError> + 'static {
        let cmd = WebDriverCommand::GetElementAttribute(self.e.clone(), attribute.to_string());
        self.c.issue_wd_cmd(cmd).and_then(|(_, v)| match v {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        })
    }

    /// Look up a DOM [property] for this element by name.
    ///
    /// `Ok(None)` is returned if the element does not have the given property.
    ///
    /// [property]: https://www.ecma-international.org/ecma-262/5.1/#sec-8.12.1
    pub fn prop(
        self,
        prop: &str,
    ) -> impl Future<Item = Option<String>, Error = error::CmdError> + 'static {
        let cmd = WebDriverCommand::GetElementProperty(self.e.clone(), prop.to_string());
        self.c.issue_wd_cmd(cmd).and_then(|(_, v)| match v {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        })
    }

    /// Retrieve the text contents of this elment.
    pub fn text(self) -> impl Future<Item = String, Error = error::CmdError> + 'static {
        let cmd = WebDriverCommand::GetElementText(self.e.clone());
        self.c.issue_wd_cmd(cmd).and_then(|(_, v)| match v {
            Json::String(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        })
    }

    /// Retrieve the HTML contents of this element.
    ///
    /// `inner` dictates whether the wrapping node's HTML is excluded or not. For example, take the
    /// HTML:
    ///
    /// ```html
    /// <div id="foo"><hr /></div>
    /// ```
    ///
    /// With `inner = true`, `<hr />` would be returned. With `inner = false`,
    /// `<div id="foo"><hr /></div>` would be returned instead.
    pub fn html(
        self,
        inner: bool,
    ) -> impl Future<Item = String, Error = error::CmdError> + 'static {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        self.prop(prop).map(|v| v.unwrap())
    }

    /// Simulate the user clicking on this element.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub fn click(self) -> impl Future<Item = Client, Error = error::CmdError> + 'static {
        let cmd = WebDriverCommand::ElementClick(self.e);
        self.c.issue_wd_cmd(cmd).and_then(move |(c, r)| {
            if r.is_null() || r.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                // geckodriver returns {} :(
                Ok(c)
            } else {
                Err(error::CmdError::NotW3C(r))
            }
        })
    }

    /// Follow the `href` target of the element matching the given CSS selector *without* causing a
    /// click interaction.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub fn follow(self) -> impl Future<Item = Client, Error = error::CmdError> + 'static {
        let cmd = WebDriverCommand::GetElementAttribute(self.e, "href".to_string());
        self.c
            .issue_wd_cmd(cmd)
            .and_then(|(this, href)| match href {
                Json::String(v) => Ok((this, v)),
                Json::Null => {
                    let e = WebDriverError::new(
                        webdriver::error::ErrorStatus::InvalidArgument,
                        "cannot follow element without href attribute",
                    );
                    Err(error::CmdError::Standard(e))
                }
                v => Err(error::CmdError::NotW3C(v)),
            })
            .and_then(|(this, href)| {
                this.current_url_()
                    .and_then(move |(this, url)| Ok((this, url.join(&href)?)))
            })
            .and_then(|(this, href)| this.goto(href.as_str()).map(|this| this))
    }

    /// Find and click an `option` child element by its `value` attribute.
    pub fn select_by_value(
        self,
        value: &str,
    ) -> impl Future<Item = Client, Error = error::CmdError> {
        let locator = format!("option[value='{}']", value);
        let locator = webdriver::command::LocatorParameters {
            using: webdriver::common::LocatorStrategy::CSSSelector,
            value: locator,
        };

        let cmd = WebDriverCommand::FindElementElement(self.e, locator);
        self.c
            .issue_wd_cmd(cmd)
            .and_then(move |(c, v)| c.parse_lookup(v).map(move |e| Element { c, e }))
            .and_then(move |e| e.click())
    }
}

impl rustc_serialize::json::ToJson for Element {
    fn to_json(&self) -> Json {
        self.e.to_json()
    }
}

impl Form {
    /// Find a form input using the given `locator` and set its value to `value`.
    pub fn set<'s>(
        &self,
        locator: Locator,
        value: &'s str,
    ) -> impl Future<Item = Self, Error = error::CmdError> + 's {
        let locator = WebDriverCommand::FindElementElement(self.f.clone(), locator.into());
        let f = Form {
            c: self.c.dup(),
            f: self.f.clone(),
        };
        self.c
            .dup()
            .issue_wd_cmd(locator)
            .and_then(|(this, res)| {
                let f = this.parse_lookup(res);
                f.map(move |f| (this, f))
            })
            .and_then(move |(this, field)| {
                use rustc_serialize::json::ToJson;
                let mut args = vec![field.to_json(), Json::String(value.to_string())];
                this.fixup_elements(&mut args);
                let cmd = webdriver::command::JavascriptCommandParameters {
                    script: "arguments[0].value = arguments[1]".to_string(),
                    args: webdriver::common::Nullable::Value(args),
                };

                this.issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            })
            .and_then(|(_, res)| {
                if res.is_null() {
                    Ok(f)
                } else {
                    Err(error::CmdError::NotW3C(res))
                }
            })
    }

    /// Find a form input with the given `name` and set its value to `value`.
    pub fn set_by_name<'s>(
        &self,
        field: &str,
        value: &'s str,
    ) -> impl Future<Item = Self, Error = error::CmdError> + 's {
        let locator = format!("input[name='{}']", field);
        let locator = Locator::Css(&locator);
        self.set(locator, value)
    }

    /// Submit this form using the first available submit button.
    ///
    /// `false` is returned if no submit button was not found.
    pub fn submit(self) -> impl Future<Item = Client, Error = error::CmdError> {
        self.submit_with(Locator::Css("input[type=submit],button[type=submit]"))
    }

    /// Submit this form using the button matched by the given selector.
    ///
    /// `false` is returned if a matching button was not found.
    pub fn submit_with(
        self,
        button: Locator,
    ) -> impl Future<Item = Client, Error = error::CmdError> + 'static {
        let locator = WebDriverCommand::FindElementElement(self.f, button.into());
        self.c
            .issue_wd_cmd(locator)
            .and_then(|(this, res)| {
                let s = this.parse_lookup(res);
                s.map(move |s| (this, s))
            })
            .and_then(move |(this, submit)| {
                this.issue_wd_cmd(WebDriverCommand::ElementClick(submit))
            })
            .and_then(move |(this, res)| {
                if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                    // geckodriver returns {} :(
                    Ok(this)
                } else {
                    Err(error::CmdError::NotW3C(res))
                }
            })
    }

    /// Submit this form using the form submit button with the given label (case-insensitive).
    ///
    /// `false` is returned if a matching button was not found.
    pub fn submit_using(
        self,
        button_label: &str,
    ) -> impl Future<Item = Client, Error = error::CmdError> + 'static {
        let escaped = button_label.replace('\\', "\\\\").replace('"', "\\\"");
        let btn = format!(
            "input[type=submit][value=\"{}\" i],\
             button[type=submit][value=\"{}\" i]",
            escaped, escaped
        );
        self.submit_with(Locator::Css(&btn))
    }

    /// Submit this form directly, without clicking any buttons.
    ///
    /// This can be useful to bypass forms that perform various magic when the submit button is
    /// clicked, or that hijack click events altogether (yes, I'm looking at you online
    /// advertisement code).
    ///
    /// Note that since no button is actually clicked, the `name=value` pair for the submit button
    /// will not be submitted. This can be circumvented by using `submit_sneaky` instead.
    pub fn submit_direct(self) -> impl Future<Item = Client, Error = error::CmdError> + 'static {
        use rustc_serialize::json::ToJson;

        let mut args = vec![self.f.clone().to_json()];
        self.c.fixup_elements(&mut args);
        // some sites are silly, and name their submit button "submit". this ends up overwriting
        // the "submit" function of the form with a reference to the submit button itself, so we
        // can't call .submit(). we get around this by creating a *new* form, and using *its*
        // submit() handler but with this pointed to the real form. solution from here:
        // https://stackoverflow.com/q/833032/472927#comment23038712_834197
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "document.createElement('form').submit.call(arguments[0])".to_string(),
            args: webdriver::common::Nullable::Value(args),
        };

        self.c
            .issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            .and_then(move |(this, res)| {
                if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                    // geckodriver returns {} :(
                    Ok(this)
                } else {
                    Err(error::CmdError::NotW3C(res))
                }
            })
    }

    /// Submit this form directly, without clicking any buttons, and with an extra field.
    ///
    /// Like `submit_direct`, this method will submit this form without clicking a submit button.
    /// However, it will *also* inject a hidden input element on the page that carries the given
    /// `field=value` mapping. This allows you to emulate the form data as it would have been *if*
    /// the submit button was indeed clicked.
    pub fn submit_sneaky(
        self,
        field: &str,
        value: &str,
    ) -> impl Future<Item = Client, Error = error::CmdError> + 'static {
        use rustc_serialize::json::ToJson;
        let mut args = vec![
            self.f.clone().to_json(),
            Json::String(field.to_string()),
            Json::String(value.to_string()),
        ];
        self.c.fixup_elements(&mut args);
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

        let f = self.f;
        self.c
            .issue_wd_cmd(WebDriverCommand::ExecuteScript(cmd))
            .and_then(move |(this, res)| {
                if res.is_null() | res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                    // geckodriver returns {} :(
                    future::Either::A(Form { f: f, c: this }.submit_direct())
                } else {
                    future::Either::B(future::err(error::CmdError::NotW3C(res)))
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_core::reactor::Core;

    macro_rules! tester {
        ($f:ident, $endpoint:expr) => {{
            use std::env;
            let mut core = Core::new().unwrap();
            let h = core.handle();
            let c = match env::var("SAUCE_ACCESS_KEY").ok() {
                Some(pwd) => {
                    let username = env::var("SAUCE_USERNAME").unwrap();
                    let mut cap = webdriver::capabilities::Capabilities::new();
                    match $endpoint {
                        "firefox" => {
                            cap.insert(
                                "platform".to_string(),
                                Json::String(env::var("PLATFORM").expect("env PLATFORM not set")),
                            );
                            cap.insert(
                                "browserName".to_string(),
                                Json::String("firefox".to_string()),
                            );
                            cap.insert("version".to_string(), Json::String("latest".to_string()));
                        }
                        "chrome" => {
                            cap.insert(
                                "platform".to_string(),
                                Json::String(env::var("PLATFORM").expect("env PLATFORM not set")),
                            );
                            cap.insert(
                                "browserName".to_string(),
                                Json::String("chrome".to_string()),
                            );
                            cap.insert("version".to_string(), Json::String("latest".to_string()));
                        }
                        _ => {}
                    }
                    cap.insert("username".to_string(), Json::String(username.clone()));
                    cap.insert("accessKey".to_string(), Json::String(pwd.clone()));
                    if let Some(tunnel) = env::var("TRAVIS_JOB_NUMBER").ok() {
                        cap.insert("tunnel-identifier".to_string(), Json::String(tunnel));
                    }

                    Client::with_capabilities(
                        &format!(
                            "http://{}:{}@ondemand.saucelabs.com:80/wd/hub/",
                            username, pwd,
                        ),
                        cap,
                        &h,
                    )
                }
                None if env::var("TRAVIS").is_ok() => {
                    // TODO: maybe use the chrome/firefox addons as a fallback?
                    // https://docs.travis-ci.com/user/gui-and-headless-browsers/#Using-xvfb-to-Run-Tests-That-Require-a-GUI
                    unimplemented!("cannot yet test on travis without Sauce");
                }
                None => {
                    // NOTE: can't be ::new because impl Future won't match
                    Client::with_capabilities(
                        "http://localhost:4444",
                        webdriver::capabilities::Capabilities::new(),
                        &h,
                    )
                }
            };

            let c = core.run(c).expect("failed to construct test client");
            let session_id = c.session_id();
            let x = core.run($f(&c));
            if let Some(fin) = c.close() {
                core.run(fin).expect("failed to close test session");
            }

            if let Ok(pwd) = env::var("SAUCE_ACCESS_KEY") {
                let tell_sauce = hyper::Client::configure()
                    .connector(hyper_tls::HttpsConnector::new(1, &core.handle()).unwrap())
                    .build(&core.handle());

                let url = format!(
                    "https://saucelabs.com/rest/v1/{}/jobs/{}",
                    env::var("SAUCE_USERNAME").unwrap(),
                    session_id
                );
                let mut req = hyper::Request::put(url.parse().unwrap());

                req.headers_mut()
                    .set(hyper::header::Authorization(hyper::header::Basic {
                        username: env::var("SAUCE_USERNAME").unwrap(),
                        password: Some(pwd),
                    }));

                let body = format!(
                    r#"{{"name": "{}", "build": "{}"{}, "passed": {}}}"#,
                    stringify!($f),
                    env::var("TRAVIS_BUILD_NUMBER")
                        .ok()
                        .unwrap_or(format!("null")),
                    env::var("TRAVIS_RUST_VERSION")
                        .map(|v| format!(r#", "tags": ["{}"]"#, v))
                        .ok()
                        .unwrap_or(String::new()),
                    if x.is_ok() { "true" } else { "false" }
                );
                req.headers_mut().set(hyper::header::ContentType::json());
                req.headers_mut()
                    .set(hyper::header::ContentLength(body.len() as u64));
                req.set_body(body.clone());
                match core.run(tell_sauce.request(req)) {
                    Err(e) => {
                        eprintln!("failed to tell sauce: {:?}", e);
                    }
                    Ok(res) => {
                        eprintln!(
                            "told sauce, got: {}",
                            String::from_utf8(core.run(res.body().concat2()).unwrap().to_vec())
                                .unwrap()
                        );
                    }
                }
            }
            x.expect("test produced unexpected error response");
        }};
    }

    fn works_inner<'a>(c: &'a Client) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        // go to the Wikipedia page for Foobar
        c.goto("https://en.wikipedia.org/wiki/Foobar")
            .and_then(move |_| c.find(Locator::Id("History_and_etymology")))
            .and_then(move |e| e.text())
            .and_then(move |text| {
                assert_eq!(text, "History and etymology");
                c.current_url()
            })
            .and_then(move |url| {
                assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
                // click "Foo (disambiguation)"
                c.find(Locator::Css(".mw-disambig"))
            })
            .and_then(|e| e.click())
            .and_then(move |_| {
                // click "Foo Lake"
                c.find(Locator::LinkText("Foo Lake"))
            })
            .and_then(|e| e.click())
            .and_then(move |_| c.current_url())
            .and_then(|url| {
                assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
                Ok(())
            })
    }

    fn clicks_inner_by_locator<'a>(
        c: &'a Client,
    ) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        // go to the Wikipedia frontpage this time
        c.goto("https://www.wikipedia.org/")
            .and_then(move |_| {
                // find, fill out, and submit the search form
                c.form(Locator::Css("#search-form"))
            })
            .and_then(|f| f.set(Locator::Css("input[name='search']"), "foobar"))
            .and_then(|f| f.submit())
            .and_then(move |_| c.current_url())
            .and_then(|url| {
                // we should now have ended up in the rigth place
                assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
                Ok(())
            })
    }

    fn clicks_inner<'a>(c: &'a Client) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        // go to the Wikipedia frontpage this time
        c.goto("https://www.wikipedia.org/")
            .and_then(move |_| {
                // find, fill out, and submit the search form
                c.form(Locator::Css("#search-form"))
            })
            .and_then(|f| f.set_by_name("search", "foobar"))
            .and_then(|f| f.submit())
            .and_then(move |_| c.current_url())
            .and_then(|url| {
                // we should now have ended up in the rigth place
                assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
                Ok(())
            })
    }

    fn raw_inner<'a>(c: &'a Client) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        // go back to the frontpage
        c.goto("https://www.wikipedia.org/")
            .and_then(move |_| {
                // find the source for the Wikipedia globe
                c.find(Locator::Css("img.central-featured-logo"))
            })
            .and_then(|img| {
                img.attr("src")
                    .map(|src| src.expect("image should have a src"))
            })
            .and_then(move |src| {
                // now build a raw HTTP client request (which also has all current cookies)
                c.raw_client_for(Method::Get, &src)
            })
            .and_then(|raw| {
                // we then read out the image bytes
                raw.body()
                    .map_err(error::CmdError::from)
                    .fold(Vec::new(), |mut pixels, chunk| {
                        pixels.extend(&*chunk);
                        future::ok::<Vec<u8>, error::CmdError>(pixels)
                    })
            })
            .and_then(|pixels| {
                // and voilla, we now have the bytes for the Wikipedia logo!
                assert!(pixels.len() > 0);
                println!("Wikipedia logo is {}b", pixels.len());
                Ok(())
            })
    }

    fn window_size_inner<'a>(
        c: &'a Client,
    ) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        c.goto("https://www.wikipedia.org/")
            .and_then(move |_| c.set_window_size(500, 400))
            .and_then(move |_| c.get_window_size())
            .and_then(move |(width, height)| {
                assert_eq!(width, 500);
                assert_eq!(height, 400);
                Ok(())
            })
    }

    fn window_position_inner<'a>(
        c: &'a Client,
    ) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        c.goto("https://www.wikipedia.org/")
            .and_then(move |_| c.set_window_size(200, 100))
            .and_then(move |_| c.set_window_position(0, 0))
            .and_then(move |_| c.set_window_position(1, 2))
            .and_then(move |_| c.get_window_position())
            .and_then(move |(x, y)| {
                assert_eq!(x, 1);
                assert_eq!(y, 2);
                Ok(())
            })
    }

    fn window_rect_inner<'a>(
        c: &'a Client,
    ) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        c.goto("https://www.wikipedia.org/")
            .and_then(move |_| c.set_window_rect(0, 0, 500, 400))
            .and_then(move |_| c.get_window_position())
            .and_then(move |(x, y)| {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                Ok(())
            })
            .and_then(move |_| c.get_window_size())
            .and_then(move |(width, height)| {
                assert_eq!(width, 500);
                assert_eq!(height, 400);
                Ok(())
            })
            .and_then(move |_| c.set_window_rect(1, 2, 600, 300))
            .and_then(move |_| c.get_window_position())
            .and_then(move |(x, y)| {
                assert_eq!(x, 1);
                assert_eq!(y, 2);
                Ok(())
            })
            .and_then(move |_| c.get_window_size())
            .and_then(move |(width, height)| {
                assert_eq!(width, 600);
                assert_eq!(height, 300);
                Ok(())
            })
    }

    fn finds_all_inner<'a>(c: &'a Client) -> impl Future<Item = (), Error = error::CmdError> + 'a {
        // go to the Wikipedia frontpage this time
        c.goto("https://en.wikipedia.org/")
            .and_then(move |_| c.find_all(Locator::Css("#p-interaction li")))
            .and_then(move |es| future::join_all(es.into_iter().take(4).map(|e| e.text())))
            .and_then(move |texts| {
                assert_eq!(texts, ["Help", "About Wikipedia", "Community portal", "Recent changes"]);
                Ok(())
            })
    }

    mod chrome {
        use super::*;

        #[test]
        fn it_works() {
            tester!(works_inner, "chrome")
        }
        #[test]
        fn it_clicks() {
            tester!(clicks_inner, "chrome")
        }
        #[test]
        fn it_can_be_raw() {
            tester!(raw_inner, "chrome")
        }
        #[test]
        fn it_can_get_and_set_window_size() {
            tester!(window_size_inner, "chrome")
        }
        #[test]
        fn it_can_get_and_set_window_position() {
            tester!(window_position_inner, "chrome")
        }
        #[test]
        fn it_can_get_and_set_window_rect() {
            tester!(window_rect_inner, "chrome")
        }
        #[test]
        fn it_finds_all() {
            tester!(finds_all_inner, "chrome")
        }
    }

    mod firefox {
        use super::*;

        #[test]
        fn it_works() {
            tester!(works_inner, "firefox")
        }
        #[test]
        fn it_clicks() {
            tester!(clicks_inner, "firefox")
        }
        #[test]
        fn it_can_be_raw() {
            tester!(raw_inner, "firefox")
        }
        #[test]
        fn it_can_get_and_set_window_size() {
            tester!(window_size_inner, "firefox")
        }
        #[test]
        #[ignore]
        fn it_can_get_and_set_window_position() {
            tester!(window_position_inner, "firefox")
        }
        #[test]
        #[ignore]
        fn it_can_get_and_set_window_rect() {
            tester!(window_rect_inner, "firefox")
        }
        #[test]
        fn it_finds_all() {
            tester!(finds_all_inner, "firefox")
        }
    }
}
