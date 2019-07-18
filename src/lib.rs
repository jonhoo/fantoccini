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
//! The examples will be using `panic!` or `unwrap` generously when errors occur (see `map_err`)
//! --- you should probably not do that in your code, and instead deal with errors when they occur.
//! This is particularly true for methods that you *expect* might fail, such as lookups by CSS
//! selector.
//!
//! Let's start out clicking around on Wikipedia:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # extern crate tokio;
//! # extern crate fantoccini;
//! use fantoccini::{Client, Locator};
//!
//! // let's set up the sequence of steps we want the browser to take
//! #[tokio::main]
//! async fn main() -> Result<(), fantoccini::error::CmdError> {
//!     let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//!
//!     // first, go to the Wikipedia page for Foobar
//!     let mut c = c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
//!     let url = c.current_url().await?;
//!     assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
//!
//!     // click "Foo (disambiguation)"
//!     c.find(Locator::Css(".mw-disambig")).await?.click().await?;
//!
//!     // click "Foo Lake"
//!     c.find(Locator::LinkText("Foo Lake")).await?.click().await?;
//!
//!     let url = c.current_url().await?;
//!     assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");
//!
//!     c.close().await
//! }
//! ```
//!
//! How did we get to the Foobar page in the first place? We did a search!
//! Let's make the program do that for us instead:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # extern crate tokio;
//! # extern crate fantoccini;
//! # use fantoccini::{Client, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! // -- snip wrapper code --
//! // go to the Wikipedia frontpage this time
//! let mut c = c.goto("https://www.wikipedia.org/").await?;
//! // find the search form, fill it out, and submit it
//! let mut c = c.form(Locator::Css("#search-form")).await?
//!              .set_by_name("search", "foobar").await?
//!              .submit().await?;
//!
//! // we should now have ended up in the rigth place
//! let url = c.current_url().await?;
//! assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");
//!
//! // -- snip wrapper code --
//! # c.close().await
//! # }
//! ```
//!
//! What if we want to download a raw file? Fantoccini has you covered:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # extern crate tokio;
//! # extern crate futures_util;
//! # extern crate fantoccini;
//! # use fantoccini::{Client, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! // -- snip wrapper code --
//! // go back to the frontpage
//! let mut c = c.goto("https://www.wikipedia.org/").await?;
//! // find the source for the Wikipedia globe
//! let mut img = c.find(Locator::Css("img.central-featured-logo")).await?;
//! let src = img.attr("src").await?.expect("image should have a src");
//! // now build a raw HTTP client request (which also has all current cookies)
//! let raw = img.client().raw_client_for(fantoccini::Method::GET, &src).await?;
//!
//! // we then read out the image bytes
//! use futures_util::TryStreamExt;
//! let pixels = raw.into_body().try_concat().await.map_err(fantoccini::error::CmdError::from)?;
//! // and voilla, we now have the bytes for the Wikipedia logo!
//! assert!(pixels.len() > 0);
//! println!("Wikipedia logo is {}b", pixels.len());
//!
//! // -- snip wrapper code --
//! # c.close().await
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
#![feature(async_await)]

use futures_util::future::{self, Either};
use futures_util::{FutureExt, TryFutureExt};
use http::HttpTryFrom;
use serde_json::Value as Json;
use tokio::prelude::*;
use tokio::sync::oneshot;
use webdriver::command::{SendKeysParameters, WebDriverCommand};
use webdriver::common::ELEMENT_KEY;
use webdriver::error::WebDriverError;

macro_rules! via_json {
    ($x:expr) => {{
        serde_json::from_str(&serde_json::to_string($x).unwrap()).unwrap()
    }};
}

pub use hyper::Method;

/// Error types.
pub mod error;

/// The long-running session future we spawn for multiplexing onto a running WebDriver instance.
mod session;
use crate::session::{Cmd, Session};

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

pub use crate::session::Client;

/// A single element on the current page.
#[derive(Clone)]
pub struct Element {
    c: Client,
    e: webdriver::common::WebElement,
}

/// An HTML form on the current page.
#[derive(Clone)]
pub struct Form {
    c: Client,
    f: webdriver::common::WebElement,
}

impl Client {
    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    ///
    /// Calls `with_capabilities` with an empty capabilities list.
    #[cfg_attr(feature = "cargo-clippy", allow(new_ret_no_self))]
    pub fn new(webdriver: &str) -> impl Future<Output = Result<Self, error::NewSessionError>> {
        Self::with_capabilities(webdriver, webdriver::capabilities::Capabilities::new())
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
        cap: webdriver::capabilities::Capabilities,
    ) -> impl Future<Output = Result<Self, error::NewSessionError>> {
        Session::with_capabilities(webdriver, cap)
    }

    /// Get the session ID assigned by the WebDriver server to this client.
    pub fn session_id(&mut self) -> impl Future<Output = Result<Option<String>, error::CmdError>> {
        self.issue(Cmd::GetSessionId).map_ok(|v| match v {
            Json::String(s) => Some(s),
            Json::Null => None,
            v => unreachable!("response to GetSessionId was not a string: {:?}", v),
        })
    }

    /// Set the User Agent string to use for all subsequent requests.
    pub fn set_ua<S: Into<String>>(
        &mut self,
        ua: S,
    ) -> impl Future<Output = Result<(), error::CmdError>> {
        self.issue(Cmd::SetUA(ua.into())).map_ok(|_| ())
    }

    /// Get the current User Agent string.
    pub fn get_ua(&mut self) -> impl Future<Output = Result<Option<String>, error::CmdError>> {
        self.issue(Cmd::GetUA).map_ok(|v| match v {
            Json::String(s) => Some(s),
            Json::Null => None,
            v => unreachable!("response to GetSessionId was not a string: {:?}", v),
        })
    }

    /// Terminate the WebDriver session.
    ///
    /// Normally, a shutdown of the WebDriver connection will be initiated when the last clone of a
    /// `Client` is dropped. Specifically, the shutdown request will be issued using the tokio
    /// `Handle` given when creating this `Client`. This in turn means that any errors will be
    /// dropped.
    ///
    /// This function is safe to call multiple times, but once it has been called on one instance
    /// of a `Client`, all requests to other instances of that `Client` will fail.
    ///
    /// This function may be useful in conjunction with `raw_client_for`, as it allows you to close
    /// the automated browser window while doing e.g., a large download.
    pub fn close(&mut self) -> impl Future<Output = Result<(), error::CmdError>> {
        self.issue(Cmd::Shutdown).map_ok(|_| ())
    }

    /// Mark this client's session as persistent.
    ///
    /// After all instances of a `Client` have been dropped, we normally shut down the WebDriver
    /// session, which also closes the associated browser window or tab. By calling this method,
    /// the shutdown command will _not_ be sent to this client's session, meaning its window or tab
    /// will remain open.
    ///
    /// Note that an explicit call to [`Client::close`] will still terminate the session.
    ///
    /// This function is safe to call multiple times.
    pub fn persist(&mut self) -> impl Future<Output = Result<(), error::CmdError>> {
        self.issue(Cmd::Persist).map_ok(|_| ())
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub fn set_window_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> impl Future<Output = Result<(), error::CmdError>> {
        if x < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(x).into(),
                format!("Expected to be `>= 0` but was `{}`", x),
            )));
        }

        if y < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(y).into(),
                format!("Expected to be `>= 0` but was `{}`", y),
            )));
        }

        if width < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(width).into(),
                format!("Expected to be `>= 0` but was `{}`", width),
            )));
        }

        if height < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(height).into(),
                format!("Expected to be `>= 0` but was `{}`", height),
            )));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
        });

        Either::Right(self.issue(cmd).map_ok(|_| ()))
    }

    /// Gets the x, y, width, and height properties of the current window.
    pub fn get_window_rect(
        &mut self,
    ) -> impl Future<Output = Result<(u64, u64, u64, u64), error::CmdError>> {
        self.issue(WebDriverCommand::GetWindowRect)
            .map(|v| match v? {
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
                v => Err(error::CmdError::NotW3C(v)),
            })
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub fn set_window_size(
        &mut self,
        width: i32,
        height: i32,
    ) -> impl Future<Output = Result<(), error::CmdError>> {
        if width < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(width).into(),
                format!("Expected to be `>= 0` but was `{}`", width),
            )));
        }

        if height < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(height).into(),
                format!("Expected to be `>= 0` but was `{}`", height),
            )));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: None,
            y: None,
            width: Some(width),
            height: Some(height),
        });

        Either::Right(self.issue(cmd).map_ok(|_| ()))
    }

    /// Gets the width and height of the current window.
    pub fn get_window_size(&mut self) -> impl Future<Output = Result<(u64, u64), error::CmdError>> {
        self.issue(WebDriverCommand::GetWindowRect)
            .map(|v| match v? {
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
                v => Err(error::CmdError::NotW3C(v)),
            })
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub fn set_window_position(
        &mut self,
        x: i32,
        y: i32,
    ) -> impl Future<Output = Result<(), error::CmdError>> {
        if x < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(x).into(),
                format!("Expected to be `>= 0` but was `{}`", x),
            )));
        }

        if y < 0 {
            return Either::Left(future::err(error::CmdError::InvalidArgument(
                stringify!(y).into(),
                format!("Expected to be `>= 0` but was `{}`", y),
            )));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Some(x),
            y: Some(y),
            width: None,
            height: None,
        });

        Either::Right(self.issue(cmd).map_ok(|_| ()))
    }

    /// Gets the x and y top-left coordinate of the current window.
    pub fn get_window_position(
        &mut self,
    ) -> impl Future<Output = Result<(u64, u64), error::CmdError>> {
        self.issue(WebDriverCommand::GetWindowRect)
            .map(|v| match v? {
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
                v => Err(error::CmdError::NotW3C(v)),
            })
    }

    /// Navigate directly to the given URL.
    pub fn goto(mut self, url: &str) -> impl Future<Output = Result<Self, error::CmdError>> {
        let url = url.to_owned();
        self.current_url_()
            .map(move |base| Ok(base?.join(&url)?))
            .and_then(move |url| {
                async move {
                    let _ = self
                        .issue(WebDriverCommand::Get(webdriver::command::GetParameters {
                            url: url.into_string(),
                        }))
                        .await?;
                    Ok(self)
                }
            })
    }

    fn current_url_(&mut self) -> impl Future<Output = Result<url::Url, error::CmdError>> {
        self.issue(WebDriverCommand::GetCurrentUrl).map(|url| {
            let url = url?;
            if let Some(url) = url.as_str() {
                return Ok(url.parse()?);
            }

            Err(error::CmdError::NotW3C(url))
        })
    }

    /// Retrieve the currently active URL for this session.
    pub fn current_url(&mut self) -> impl Future<Output = Result<url::Url, error::CmdError>> {
        self.current_url_()
    }

    /// Get a PNG-encoded screenshot of the current page.
    pub fn screenshot(&mut self) -> impl Future<Output = Result<Vec<u8>, error::CmdError>> {
        self.issue(WebDriverCommand::TakeScreenshot).map(|src| {
            let src = src?;
            if let Some(src) = src.as_str() {
                return base64::decode(src).map_err(|e| error::CmdError::ImageDecodeError(e));
            }

            Err(error::CmdError::NotW3C(src))
        })
    }

    /// Get the HTML source for the current page.
    pub fn source(&mut self) -> impl Future<Output = Result<String, error::CmdError>> {
        self.issue(WebDriverCommand::GetPageSource).map(|src| {
            let src = src?;
            if let Some(src) = src.as_str() {
                return Ok(src.to_string());
            }

            Err(error::CmdError::NotW3C(src))
        })
    }

    /// Go back to the previous page.
    pub fn back(&mut self) -> impl Future<Output = Result<(), error::CmdError>> {
        self.issue(WebDriverCommand::GoBack).map_ok(|_| ())
    }

    /// Refresh the current previous page.
    pub fn refresh(&mut self) -> impl Future<Output = Result<(), error::CmdError>> {
        self.issue(WebDriverCommand::Refresh).map_ok(|_| ())
    }

    /// Execute the given JavaScript `script` in the current browser session.
    ///
    /// `args` is available to the script inside the `arguments` array. Since `Element` implements
    /// `ToJson`, you can also provide serialized `Element`s as arguments, and they will correctly
    /// serialize to DOM elements on the other side.
    ///
    /// To retrieve the value of a variable, `return` has to be used in the JavaScript code.
    pub fn execute(
        &mut self,
        script: &str,
        mut args: Vec<Json>,
    ) -> impl Future<Output = Result<Json, error::CmdError>> {
        self.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: script.to_string(),
            args: Some(args),
        };

        self.issue(WebDriverCommand::ExecuteScript(cmd))
    }

    /// Issue an HTTP request to the given `url` with all the same cookies as the current session.
    ///
    /// Calling this method is equivalent to calling `with_raw_client_for` with an empty closure.
    pub fn raw_client_for<'a>(
        self,
        method: Method,
        url: &str,
    ) -> impl Future<Output = Result<hyper::Response<hyper::Body>, error::CmdError>> {
        self.with_raw_client_for(method, url, |mut req| {
            req.body(hyper::Body::empty()).unwrap()
        })
    }

    /// Build and issue an HTTP request to the given `url` with all the same cookies as the current
    /// session.
    ///
    /// Before the HTTP request is issued, the given `before` closure will be called with a handle
    /// to the `Request` about to be sent.
    pub fn with_raw_client_for<F>(
        self,
        method: Method,
        url: &str,
        before: F,
    ) -> impl Future<Output = Result<hyper::Response<hyper::Body>, error::CmdError>>
    where
        F: FnOnce(http::request::Builder) -> hyper::Request<hyper::Body>,
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
        async move {
            let mut this = self;
            let old_url = this.current_url_().await?;
            let url = old_url.clone().join(&url)?;
            let cookie_url = url.clone().join("/please_give_me_your_cookies")?;
            this = this.goto(cookie_url.as_str()).await?;

            // TODO: go back before we return if this call errors:
            let cookies = this.issue(WebDriverCommand::GetCookies).await?;
            if !cookies.is_array() {
                // NOTE: this clone should _really_ not be necessary
                Err(error::CmdError::NotW3C(cookies.clone()))?;
            }
            this.back().await?;
            let ua = this.get_ua().await?;

            // now add all the cookies
            let mut all_ok = true;
            let mut jar = Vec::new();
            for cookie in cookies.as_array().unwrap() {
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

                // Note that since we're sending these cookies, all that matters is the mapping
                // from name to value. The other fields only matter when deciding whether to
                // include a cookie or not, and the driver has already decided that for us
                // (GetCookies is for a particular URL).
                jar.push(
                    cookie::Cookie::new(
                        cookie["name"].as_str().unwrap().to_owned(),
                        cookie["value"].as_str().unwrap().to_owned(),
                    )
                    .encoded()
                    .to_string(),
                );
            }

            if !all_ok {
                // NOTE: this clone should _really_ not be necessary
                Err(error::CmdError::NotW3C(cookies))?;
            }

            let mut req = hyper::Request::builder();
            req.method(method)
                .uri(http::Uri::try_from(url.as_str()).unwrap());
            req.header(hyper::header::COOKIE, jar.join("; "));
            if let Some(s) = ua {
                req.header(hyper::header::USER_AGENT, s);
            }
            let req = before(req);
            let (tx, rx) = oneshot::channel();
            this.issue(Cmd::Raw { req, rsp: tx }).await?;
            match rx.await {
                Ok(Ok(r)) => Ok(r),
                Ok(Err(e)) => Err(e.into()),
                Err(e) => unreachable!("Session ended prematurely: {:?}", e),
            }
        }
    }

    /// Find an element on the page.
    pub fn find(
        &mut self,
        search: Locator,
    ) -> impl Future<Output = Result<Element, error::CmdError>> {
        self.by(search.into())
    }

    /// Find elements on the page.
    pub fn find_all(
        &mut self,
        search: Locator,
    ) -> impl Future<Output = Result<Vec<Element>, error::CmdError>> {
        let this = self.clone();
        self.issue(WebDriverCommand::FindElements(search.into()))
            .map(move |res| {
                let res = res?;
                let array = this.parse_lookup_all(res)?;
                Ok(array
                    .into_iter()
                    .map(move |e| Element {
                        c: this.clone(),
                        e: e,
                    })
                    .collect())
            })
    }

    /// Wait for the given function to return `true` before proceeding.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub fn wait_for<F, FF>(
        mut self,
        mut is_ready: F,
    ) -> impl Future<Output = Result<Self, error::CmdError>>
    where
        F: FnMut(&mut Client) -> FF,
        FF: Future<Output = Result<bool, error::CmdError>>,
    {
        async move {
            while !is_ready(&mut self).await? {}
            Ok(self)
        }
    }

    /// Wait for the given element to be present on the page.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub fn wait_for_find(
        mut self,
        search: Locator,
    ) -> impl Future<Output = Result<Element, error::CmdError>> {
        let s: webdriver::command::LocatorParameters = search.into();
        async move {
            loop {
                match self
                    .by(webdriver::command::LocatorParameters {
                        using: s.using.clone(),
                        value: s.value.clone(),
                    })
                    .await
                {
                    Ok(v) => break Ok(v),
                    Err(error::CmdError::NoSuchElement(_)) => {}
                    Err(e) => break Err(e),
                }
            }
        }
    }

    /// Wait for the page to navigate to a new URL before proceeding.
    ///
    /// If the `current` URL is not provided, `self.current_url()` will be used. Note however that
    /// this introduces a race condition: the browser could finish navigating *before* we call
    /// `current_url()`, which would lead to an eternal wait.
    pub fn wait_for_navigation(
        mut self,
        current: Option<url::Url>,
    ) -> impl Future<Output = Result<Self, error::CmdError>> {
        match current {
            Some(current) => Either::Left(future::ok(current)),
            None => Either::Right(self.current_url_()),
        }
        .and_then(move |current| {
            self.wait_for(move |c| {
                // TODO: get rid of this clone
                let current = current.clone();
                c.current_url().map_ok(move |url| url != current)
            })
        })
    }

    /// Locate a form on the page.
    ///
    /// Through the returned `Form`, HTML forms can be filled out and submitted.
    pub fn form(&mut self, search: Locator) -> impl Future<Output = Result<Form, error::CmdError>> {
        let mut c = self.clone();
        let l = search.into();
        async move {
            let res = c.issue(WebDriverCommand::FindElement(l)).await?;
            let f = c.parse_lookup(res)?;
            Ok(Form { c, f: f })
        }
    }

    // helpers

    fn by(
        &mut self,
        locator: webdriver::command::LocatorParameters,
    ) -> impl Future<Output = Result<Element, error::CmdError>> {
        let mut c = self.clone();
        c.issue(WebDriverCommand::FindElement(locator))
            .map(move |res| {
                let res = res?;
                let e = c.parse_lookup(res)?;
                Ok(Element { c: c.clone(), e: e })
            })
    }

    /// Extract the `WebElement` from a `FindElement` or `FindElementElement` command.
    fn parse_lookup(&self, res: Json) -> Result<webdriver::common::WebElement, error::CmdError> {
        let mut res = match res {
            Json::Object(o) => o,
            res => return Err(error::CmdError::NotW3C(res)),
        };

        // legacy protocol uses "ELEMENT" as identifier
        let key = if self.is_legacy() {
            "ELEMENT"
        } else {
            ELEMENT_KEY
        };

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
    fn parse_lookup_all(
        &self,
        res: Json,
    ) -> Result<Vec<webdriver::common::WebElement>, error::CmdError> {
        let res = match res {
            Json::Array(a) => a,
            res => return Err(error::CmdError::NotW3C(res)),
        };

        let mut array = Vec::new();
        for json in res {
            let e = self.parse_lookup(json)?;
            array.push(e);
        }

        Ok(array)
    }

    fn fixup_elements(&self, args: &mut [Json]) {
        if self.is_legacy() {
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
        &mut self,
        attribute: &str,
    ) -> impl Future<Output = Result<Option<String>, error::CmdError>> {
        let cmd = WebDriverCommand::GetElementAttribute(self.e.clone(), attribute.to_string());
        self.c.issue(cmd).map(|v| match v? {
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
        &mut self,
        prop: &str,
    ) -> impl Future<Output = Result<Option<String>, error::CmdError>> {
        let cmd = WebDriverCommand::GetElementProperty(self.e.clone(), prop.to_string());
        self.c.issue(cmd).map(|v| match v? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        })
    }

    /// Retrieve the text contents of this elment.
    pub fn text(&mut self) -> impl Future<Output = Result<String, error::CmdError>> {
        let cmd = WebDriverCommand::GetElementText(self.e.clone());
        self.c.issue(cmd).map(|v| match v? {
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
    pub fn html(&mut self, inner: bool) -> impl Future<Output = Result<String, error::CmdError>> {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        self.prop(prop).map_ok(|v| v.unwrap())
    }

    /// Simulate the user clicking on this element.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub fn click(self) -> impl Future<Output = Result<Client, error::CmdError>> {
        let e = self.e;
        let mut c = self.c;
        let cmd = WebDriverCommand::ElementClick(e);
        c.issue(cmd).map(move |r| {
            let r = r?;
            if r.is_null() || r.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                // geckodriver returns {} :(
                Ok(c)
            } else {
                Err(error::CmdError::NotW3C(r))
            }
        })
    }

    /// Simulate the user sending keys to an element.
    pub fn send_keys(&mut self, text: &str) -> impl Future<Output = Result<(), error::CmdError>> {
        let cmd = WebDriverCommand::ElementSendKeys(
            self.e.clone(),
            SendKeysParameters {
                text: text.to_owned(),
            },
        );
        self.c.issue(cmd).map(move |r| {
            let r = r?;
            if r.is_null() {
                Ok(())
            } else {
                Err(error::CmdError::NotW3C(r))
            }
        })
    }

    /// Get back the [`Client`] hosting this `Element`.
    pub fn client(self) -> Client {
        self.c
    }

    /// Follow the `href` target of the element matching the given CSS selector *without* causing a
    /// click interaction.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub fn follow(self) -> impl Future<Output = Result<Client, error::CmdError>> {
        let e = self.e;
        let mut c = self.c;
        let cmd = WebDriverCommand::GetElementAttribute(e, "href".to_string());
        async move {
            let href = c.issue(cmd).await?;
            let href = match href {
                Json::String(v) => v,
                Json::Null => {
                    let e = WebDriverError::new(
                        webdriver::error::ErrorStatus::InvalidArgument,
                        "cannot follow element without href attribute",
                    );
                    Err(error::CmdError::Standard(e))?
                }
                v => Err(error::CmdError::NotW3C(v))?,
            };

            let url = c.current_url_().await?;
            let href = url.join(&href)?;
            c.goto(href.as_str()).await
        }
    }

    /// Find and click an `option` child element by its `value` attribute.
    pub fn select_by_value(
        self,
        value: &str,
    ) -> impl Future<Output = Result<Client, error::CmdError>> {
        let locator = format!("option[value='{}']", value);
        let locator = webdriver::command::LocatorParameters {
            using: webdriver::common::LocatorStrategy::CSSSelector,
            value: locator,
        };

        let e = self.e;
        let mut c = self.c;
        let cmd = WebDriverCommand::FindElementElement(e, locator);
        async move {
            let v = c.issue(cmd).await?;
            Element {
                e: c.parse_lookup(v)?,
                c,
            }
            .click()
            .await
        }
    }
}

impl Form {
    /// Find a form input using the given `locator` and set its value to `value`.
    pub fn set(
        &mut self,
        locator: Locator,
        value: &str,
    ) -> impl Future<Output = Result<Self, error::CmdError>> {
        let locator = WebDriverCommand::FindElementElement(self.f.clone(), locator.into());
        let f = self.f.clone();
        let mut this = self.c.clone();
        let value = Json::from(value);
        let res = self.c.issue(locator);

        async move {
            let field = this.parse_lookup(res.await?)?;
            let mut args = vec![via_json!(&field), value];
            this.fixup_elements(&mut args);
            let cmd = webdriver::command::JavascriptCommandParameters {
                script: "arguments[0].value = arguments[1]".to_string(),
                args: Some(args),
            };

            let res = this.issue(WebDriverCommand::ExecuteScript(cmd)).await?;
            if res.is_null() {
                Ok(Form { c: this, f: f })
            } else {
                Err(error::CmdError::NotW3C(res))
            }
        }
    }

    /// Find a form input with the given `name` and set its value to `value`.
    pub fn set_by_name<'s>(
        &mut self,
        field: &str,
        value: &str,
    ) -> impl Future<Output = Result<Self, error::CmdError>> {
        let locator = format!("input[name='{}']", field);
        let locator = Locator::Css(&locator);
        self.set(locator, value)
    }

    /// Submit this form using the first available submit button.
    ///
    /// `false` is returned if no submit button was not found.
    pub fn submit(self) -> impl Future<Output = Result<Client, error::CmdError>> {
        self.submit_with(Locator::Css("input[type=submit],button[type=submit]"))
    }

    /// Submit this form using the button matched by the given selector.
    ///
    /// `false` is returned if a matching button was not found.
    pub fn submit_with(
        self,
        button: Locator,
    ) -> impl Future<Output = Result<Client, error::CmdError>> {
        let f = self.f;
        let mut c = self.c;
        let locator = WebDriverCommand::FindElementElement(f, button.into());
        async move {
            let res = c.issue(locator).await?;
            let submit = c.parse_lookup(res)?;
            let res = c.issue(WebDriverCommand::ElementClick(submit)).await?;
            if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                // geckodriver returns {} :(
                Ok(c)
            } else {
                Err(error::CmdError::NotW3C(res))
            }
        }
    }

    /// Submit this form using the form submit button with the given label (case-insensitive).
    ///
    /// `false` is returned if a matching button was not found.
    pub fn submit_using(
        self,
        button_label: &str,
    ) -> impl Future<Output = Result<Client, error::CmdError>> {
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
    pub fn submit_direct(mut self) -> impl Future<Output = Result<Client, error::CmdError>> {
        let mut args = vec![via_json!(&self.f)];
        self.c.fixup_elements(&mut args);
        // some sites are silly, and name their submit button "submit". this ends up overwriting
        // the "submit" function of the form with a reference to the submit button itself, so we
        // can't call .submit(). we get around this by creating a *new* form, and using *its*
        // submit() handler but with this pointed to the real form. solution from here:
        // https://stackoverflow.com/q/833032/472927#comment23038712_834197
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "document.createElement('form').submit.call(arguments[0])".to_string(),
            args: Some(args),
        };

        self.c
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .map(move |res| {
                let res = res?;
                if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                    // geckodriver returns {} :(
                    Ok(self.c)
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
    ) -> impl Future<Output = Result<Client, error::CmdError>> {
        let mut args = vec![via_json!(&self.f), Json::from(field), Json::from(value)];
        self.c.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "\
                     var h = document.createElement('input');\
                     h.setAttribute('type', 'hidden');\
                     h.setAttribute('name', arguments[1]);\
                     h.value = arguments[2];\
                     arguments[0].appendChild(h)"
                .to_string(),
            args: Some(args),
        };

        let f = self.f;
        let mut c = self.c;
        c.issue(WebDriverCommand::ExecuteScript(cmd))
            .and_then(move |res| {
                if res.is_null() | res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
                    // geckodriver returns {} :(
                    Either::Left(Form { f, c }.submit_direct())
                } else {
                    Either::Right(future::err(error::CmdError::NotW3C(res)))
                }
            })
    }

    /// Get back the [`Client`] hosting this `Form`.
    pub fn client(self) -> Client {
        self.c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::try_future;
    use futures_util::TryStreamExt;

    macro_rules! tester {
        ($f:ident, $endpoint:expr) => {{
            use std::sync::{Arc, Mutex};
            use std::thread;
            let c = match $endpoint {
                "firefox" => {
                    let mut caps = serde_json::map::Map::new();
                    let opts = serde_json::json!({ "args": ["--headless"] });
                    caps.insert("moz:firefoxOptions".to_string(), opts.clone());
                    Client::with_capabilities("http://localhost:4444", caps)
                },
                "chrome" => {
                    let mut caps = serde_json::map::Map::new();
                    let opts = serde_json::json!({
                        "args": ["--headless", "--disable-gpu", "--no-sandbox", "--disable-dev-shm-usage"],
                        "binary":
                            if std::path::Path::new("/usr/bin/chromium-browser").exists() {
                                // on Ubuntu, it's called chromium-browser
                                "/usr/bin/chromium-browser"
                            } else {
                                // elsewhere, it's just called chromium
                                "/usr/bin/chromium"
                            }
                    });
                    caps.insert("goog:chromeOptions".to_string(), opts.clone());

                    Client::with_capabilities("http://localhost:9515", caps)
                },
                browser => unimplemented!("unsupported browser backend {}", browser),
            };

            // we'll need the session_id from the thread
            // NOTE: even if it panics, so can't just return it
            let session_id = Arc::new(Mutex::new(None));

            // run test in its own thread to catch panics
            let sid = session_id.clone();
            let success = match thread::spawn(move || {
                let mut rt = tokio::runtime::current_thread::Runtime::new().unwrap();
                let mut c = rt.block_on(c).expect("failed to construct test client");
                *sid.lock().unwrap() = rt.block_on(c.session_id()).unwrap();
                let x = rt.block_on($f(c));
                rt.run().unwrap();
                x
            })
            .join()
            {
                Ok(Ok(_)) => true,
                Ok(Err(e)) => {
                    eprintln!("test future failed to resolve: {:?}", e);
                    false
                }
                Err(e) => {
                    if let Some(e) = e.downcast_ref::<error::CmdError>() {
                        eprintln!("test future panicked: {:?}", e);
                    } else if let Some(e) = e.downcast_ref::<error::NewSessionError>() {
                        eprintln!("test future panicked: {:?}", e);
                    } else {
                        eprintln!("test future panicked; an assertion probably failed");
                    }
                    false
                }
            };

            assert!(success);
        }};
    }

    async fn works_inner(c: Client) -> Result<(), error::CmdError> {
        // go to the Wikipedia page for Foobar
        let mut c = c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
        let mut e = c.find(Locator::Id("History_and_etymology")).await?;
        let text = e.text().await?;
        assert_eq!(text, "History and etymology");
        let mut c = e.client();
        let url = c.current_url().await?;
        assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

        // click "Foo (disambiguation)"
        let mut c = c.find(Locator::Css(".mw-disambig")).await?.click().await?;

        // click "Foo Lake"
        let mut c = c.find(Locator::LinkText("Foo Lake")).await?.click().await?;

        let url = c.current_url().await?;
        assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foo_Lake");

        c.close().await
    }

    async fn clicks_inner_by_locator(c: Client) -> Result<(), error::CmdError> {
        // go to the Wikipedia frontpage this time
        let mut c = c.goto("https://www.wikipedia.org/").await?;

        // find, fill out, and submit the search form
        let mut f = c.form(Locator::Css("#search-form")).await?;
        let f = f
            .set(Locator::Css("input[name='search']"), "foobar")
            .await?;
        let mut c = f.submit().await?;
        let url = c.current_url().await?;

        // we should now have ended up in the rigth place
        assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

        c.close().await
    }

    async fn clicks_inner(c: Client) -> Result<(), error::CmdError> {
        // go to the Wikipedia frontpage this time
        let mut c = c.goto("https://www.wikipedia.org/").await?;

        // find, fill out, and submit the search form
        let mut f = c.form(Locator::Css("#search-form")).await?;
        let f = f.set_by_name("search", "foobar").await?;
        let mut c = f.submit().await?;
        let url = c.current_url().await?;

        // we should now have ended up in the rigth place
        assert_eq!(url.as_ref(), "https://en.wikipedia.org/wiki/Foobar");

        c.close().await
    }

    async fn raw_inner(c: Client) -> Result<(), error::CmdError> {
        // go back to the frontpage
        let mut c = c.goto("https://www.wikipedia.org/").await?;

        // find the source for the Wikipedia globe
        let mut img = c.find(Locator::Css("img.central-featured-logo")).await?;
        let src = img.attr("src").await?.expect("image should have a src");

        // now build a raw HTTP client request (which also has all current cookies)
        let raw = img.client().raw_client_for(Method::GET, &src).await?;

        // we then read out the image bytes
        let pixels = raw
            .into_body()
            .try_concat()
            .map_err(error::CmdError::from)
            .await?;

        // and voilla, we now have the bytes for the Wikipedia logo!
        assert!(pixels.len() > 0);
        println!("Wikipedia logo is {}b", pixels.len());

        c.close().await
    }

    async fn window_size_inner(c: Client) -> Result<(), error::CmdError> {
        let mut c = c.goto("https://www.wikipedia.org/").await?;
        c.set_window_size(500, 400).await?;
        let (width, height) = c.get_window_size().await?;
        assert_eq!(width, 500);
        assert_eq!(height, 400);

        c.close().await
    }

    async fn window_position_inner(c: Client) -> Result<(), error::CmdError> {
        let mut c = c.goto("https://www.wikipedia.org/").await?;
        c.set_window_size(200, 100).await?;
        c.set_window_position(0, 0).await?;
        c.set_window_position(1, 2).await?;
        let (x, y) = c.get_window_position().await?;
        assert_eq!(x, 1);
        assert_eq!(y, 2);

        c.close().await
    }

    async fn window_rect_inner(c: Client) -> Result<(), error::CmdError> {
        let mut c = c.goto("https://www.wikipedia.org/").await?;
        c.set_window_rect(0, 0, 500, 400).await?;
        let (x, y) = c.get_window_position().await?;
        assert_eq!(x, 0);
        assert_eq!(y, 0);
        let (width, height) = c.get_window_size().await?;
        assert_eq!(width, 500);
        assert_eq!(height, 400);
        c.set_window_rect(1, 2, 600, 300).await?;
        let (x, y) = c.get_window_position().await?;
        assert_eq!(x, 1);
        assert_eq!(y, 2);
        let (width, height) = c.get_window_size().await?;
        assert_eq!(width, 600);
        assert_eq!(height, 300);

        c.close().await
    }

    async fn finds_all_inner(c: Client) -> Result<(), error::CmdError> {
        // go to the Wikipedia frontpage this time
        let mut c = c.goto("https://en.wikipedia.org/").await?;
        let es = c.find_all(Locator::Css("#p-interaction li")).await?;
        let texts = try_future::try_join_all(es.into_iter().take(4).map(|mut e| e.text())).await?;
        assert_eq!(
            texts,
            [
                "Help",
                "About Wikipedia",
                "Community portal",
                "Recent changes"
            ]
        );

        c.close().await
    }

    fn persist_inner(c: Client) -> impl Future<Output = Result<(), error::CmdError>> {
        c.goto("https://en.wikipedia.org/")
            .and_then(|mut c| c.persist())
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
        fn it_clicks_by_locator() {
            tester!(clicks_inner_by_locator, "chrome")
        }
        #[test]
        fn it_can_be_raw() {
            tester!(raw_inner, "chrome")
        }
        #[test]
        #[ignore]
        fn it_can_get_and_set_window_size() {
            tester!(window_size_inner, "chrome")
        }
        #[test]
        #[ignore]
        fn it_can_get_and_set_window_position() {
            tester!(window_position_inner, "chrome")
        }
        #[test]
        #[ignore]
        fn it_can_get_and_set_window_rect() {
            tester!(window_rect_inner, "chrome")
        }
        #[test]
        fn it_finds_all() {
            tester!(finds_all_inner, "chrome")
        }
        #[test]
        #[ignore]
        fn it_persists() {
            tester!(persist_inner, "chrome")
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
        fn it_clicks_by_locator() {
            tester!(clicks_inner_by_locator, "firefox")
        }
        #[test]
        fn it_can_be_raw() {
            tester!(raw_inner, "firefox")
        }
        #[test]
        #[ignore]
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
        #[test]
        #[ignore]
        fn it_persists() {
            tester!(persist_inner, "firefox")
        }
    }
}
