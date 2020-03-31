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
//!     c.goto("https://en.wikipedia.org/wiki/Foobar").await?;
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
//! # extern crate tokio;
//! # extern crate fantoccini;
//! # use fantoccini::{Client, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! // -- snip wrapper code --
//! // go to the Wikipedia frontpage this time
//! c.goto("https://www.wikipedia.org/").await?;
//! // find the search form, fill it out, and submit it
//! let mut f = c.form(Locator::Css("#search-form")).await?;
//! f.set_by_name("search", "foobar").await?
//!  .submit().await?;
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
//! # extern crate tokio;
//! # extern crate futures_util;
//! # extern crate fantoccini;
//! # use fantoccini::{Client, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! // -- snip wrapper code --
//! // go back to the frontpage
//! c.goto("https://www.wikipedia.org/").await?;
//! // find the source for the Wikipedia globe
//! let mut img = c.find(Locator::Css("img.central-featured-logo")).await?;
//! let src = img.attr("src").await?.expect("image should have a src");
//! // now build a raw HTTP client request (which also has all current cookies)
//! let raw = img.client().raw_client_for(fantoccini::Method::GET, &src).await?;
//!
//! // we then read out the image bytes
//! use futures_util::TryStreamExt;
//! let pixels = hyper::body::to_bytes(raw.into_body()).await.map_err(fantoccini::error::CmdError::from)?;
//! // and voilla, we now have the bytes for the Wikipedia logo!
//! assert!(pixels.len() > 0);
//! println!("Wikipedia logo is {}b", pixels.len());
//!
//! // -- snip wrapper code --
//! # c.close().await
//! # }
//! ```
//!
//! For more examples, take a look at the `examples/` directory.
//!
//! [WebDriver protocol]: https://www.w3.org/TR/webdriver/
//! [CSS selectors]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
//! [powerful]: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes
//! [operators]: https://developer.mozilla.org/en-US/docs/Web/CSS/Attribute_selectors
//! [WebDriver compatible]: https://github.com/Fyrd/caniuse/issues/2757#issuecomment-304529217
//! [`geckodriver`]: https://github.com/mozilla/geckodriver
#![deny(missing_docs)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

use serde_json::Value as Json;
use std::convert::TryFrom;
use std::future::Future;
use tokio::sync::oneshot;
use webdriver::command::{
    NewWindowParameters, SendKeysParameters, SwitchToFrameParameters, SwitchToWindowParameters,
    WebDriverCommand,
};
use webdriver::common::{FrameId, ELEMENT_KEY};
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
#[derive(Clone, Debug)]
pub struct Element {
    client: Client,
    element: webdriver::common::WebElement,
}

/// An HTML form on the current page.
#[derive(Clone, Debug)]
pub struct Form {
    client: Client,
    form: webdriver::common::WebElement,
}

impl Client {
    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    ///
    /// Calls `with_capabilities` with an empty capabilities list.
    #[allow(clippy::new_ret_no_self)]
    pub async fn new(webdriver: &str) -> Result<Self, error::NewSessionError> {
        Self::with_capabilities(webdriver, webdriver::capabilities::Capabilities::new()).await
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
    pub async fn with_capabilities(
        webdriver: &str,
        cap: webdriver::capabilities::Capabilities,
    ) -> Result<Self, error::NewSessionError> {
        Session::with_capabilities(webdriver, cap).await
    }

    /// Get the session ID assigned by the WebDriver server to this client.
    pub async fn session_id(&mut self) -> Result<Option<String>, error::CmdError> {
        match self.issue(Cmd::GetSessionId).await? {
            Json::String(s) => Ok(Some(s)),
            Json::Null => Ok(None),
            v => unreachable!("response to GetSessionId was not a string: {:?}", v),
        }
    }

    /// Set the User Agent string to use for all subsequent requests.
    pub async fn set_ua<S: Into<String>>(&mut self, ua: S) -> Result<(), error::CmdError> {
        self.issue(Cmd::SetUA(ua.into())).await?;
        Ok(())
    }

    /// Get the current User Agent string.
    pub async fn get_ua(&mut self) -> Result<Option<String>, error::CmdError> {
        match self.issue(Cmd::GetUA).await? {
            Json::String(s) => Ok(Some(s)),
            Json::Null => Ok(None),
            v => unreachable!("response to GetSessionId was not a string: {:?}", v),
        }
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
    pub async fn close(&mut self) -> Result<(), error::CmdError> {
        self.issue(Cmd::Shutdown).await?;
        Ok(())
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
    pub async fn persist(&mut self) -> Result<(), error::CmdError> {
        self.issue(Cmd::Persist).await?;
        Ok(())
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub async fn set_window_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), error::CmdError> {
        if x < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(x).into(),
                format!("Expected to be `>= 0` but was `{}`", x),
            ));
        }

        if y < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(y).into(),
                format!("Expected to be `>= 0` but was `{}`", y),
            ));
        }

        if width < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(width).into(),
                format!("Expected to be `>= 0` but was `{}`", width),
            ));
        }

        if height < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(height).into(),
                format!("Expected to be `>= 0` but was `{}`", height),
            ));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
        });

        self.issue(cmd).await?;
        Ok(())
    }

    /// Gets the x, y, width, and height properties of the current window.
    pub async fn get_window_rect(&mut self) -> Result<(u64, u64, u64, u64), error::CmdError> {
        match self.issue(WebDriverCommand::GetWindowRect).await? {
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
        }
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub async fn set_window_size(
        &mut self,
        width: i32,
        height: i32,
    ) -> Result<(), error::CmdError> {
        if width < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(width).into(),
                format!("Expected to be `>= 0` but was `{}`", width),
            ));
        }

        if height < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(height).into(),
                format!("Expected to be `>= 0` but was `{}`", height),
            ));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: None,
            y: None,
            width: Some(width),
            height: Some(height),
        });

        self.issue(cmd).await?;
        Ok(())
    }

    /// Gets the width and height of the current window.
    pub async fn get_window_size(&mut self) -> Result<(u64, u64), error::CmdError> {
        match self.issue(WebDriverCommand::GetWindowRect).await? {
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
        }
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// All values must be `>= 0` or you will get a `CmdError::InvalidArgument`.
    pub async fn set_window_position(&mut self, x: i32, y: i32) -> Result<(), error::CmdError> {
        if x < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(x).into(),
                format!("Expected to be `>= 0` but was `{}`", x),
            ));
        }

        if y < 0 {
            return Err(error::CmdError::InvalidArgument(
                stringify!(y).into(),
                format!("Expected to be `>= 0` but was `{}`", y),
            ));
        }

        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Some(x),
            y: Some(y),
            width: None,
            height: None,
        });

        self.issue(cmd).await?;
        Ok(())
    }

    /// Gets the x and y top-left coordinate of the current window.
    pub async fn get_window_position(&mut self) -> Result<(u64, u64), error::CmdError> {
        match self.issue(WebDriverCommand::GetWindowRect).await? {
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
        }
    }

    /// Navigate directly to the given URL.
    pub async fn goto(&mut self, url: &str) -> Result<(), error::CmdError> {
        let url = url.to_owned();
        let base = self.current_url_().await?;
        let url = base.join(&url)?;
        self.issue(WebDriverCommand::Get(webdriver::command::GetParameters {
            url: url.into_string(),
        }))
        .await?;
        Ok(())
    }

    async fn current_url_(&mut self) -> Result<url::Url, error::CmdError> {
        let url = self.issue(WebDriverCommand::GetCurrentUrl).await?;
        if let Some(url) = url.as_str() {
            let url = if url.is_empty() { "about:blank" } else { url };
            Ok(url.parse()?)
        } else {
            Err(error::CmdError::NotW3C(url))
        }
    }

    /// Retrieve the currently active URL for this session.
    pub async fn current_url(&mut self) -> Result<url::Url, error::CmdError> {
        self.current_url_().await
    }

    /// Get a PNG-encoded screenshot of the current page.
    pub async fn screenshot(&mut self) -> Result<Vec<u8>, error::CmdError> {
        let src = self.issue(WebDriverCommand::TakeScreenshot).await?;
        if let Some(src) = src.as_str() {
            base64::decode(src).map_err(error::CmdError::ImageDecodeError)
        } else {
            Err(error::CmdError::NotW3C(src))
        }
    }

    /// Get the HTML source for the current page.
    pub async fn source(&mut self) -> Result<String, error::CmdError> {
        let src = self.issue(WebDriverCommand::GetPageSource).await?;
        if let Some(src) = src.as_str() {
            Ok(src.to_string())
        } else {
            Err(error::CmdError::NotW3C(src))
        }
    }

    /// Go back to the previous page.
    pub async fn back(&mut self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::GoBack).await?;
        Ok(())
    }

    /// Refresh the current previous page.
    pub async fn refresh(&mut self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::Refresh).await?;
        Ok(())
    }

    /// Execute the given JavaScript `script` in the current browser session.
    ///
    /// `args` is available to the script inside the `arguments` array. Since `Element` implements
    /// `ToJson`, you can also provide serialized `Element`s as arguments, and they will correctly
    /// serialize to DOM elements on the other side.
    ///
    /// To retrieve the value of a variable, `return` has to be used in the JavaScript code.
    pub async fn execute(
        &mut self,
        script: &str,
        mut args: Vec<Json>,
    ) -> Result<Json, error::CmdError> {
        self.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: script.to_string(),
            args: Some(args),
        };

        self.issue(WebDriverCommand::ExecuteScript(cmd)).await
    }

    /// Issue an HTTP request to the given `url` with all the same cookies as the current session.
    ///
    /// Calling this method is equivalent to calling `with_raw_client_for` with an empty closure.
    pub async fn raw_client_for(
        &mut self,
        method: Method,
        url: &str,
    ) -> Result<hyper::Response<hyper::Body>, error::CmdError> {
        self.with_raw_client_for(method, url, |req| req.body(hyper::Body::empty()).unwrap())
            .await
    }

    /// Build and issue an HTTP request to the given `url` with all the same cookies as the current
    /// session.
    ///
    /// Before the HTTP request is issued, the given `before` closure will be called with a handle
    /// to the `Request` about to be sent.
    pub async fn with_raw_client_for<F>(
        &mut self,
        method: Method,
        url: &str,
        before: F,
    ) -> Result<hyper::Response<hyper::Body>, error::CmdError>
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
        let old_url = self.current_url_().await?;
        let url = old_url.clone().join(&url)?;
        let cookie_url = url.clone().join("/please_give_me_your_cookies")?;
        self.goto(cookie_url.as_str()).await?;

        // TODO: go back before we return if this call errors:
        let cookies = self.issue(WebDriverCommand::GetCookies).await?;
        if !cookies.is_array() {
            // NOTE: this clone should _really_ not be necessary
            Err(error::CmdError::NotW3C(cookies.clone()))?;
        }
        self.back().await?;
        let ua = self.get_ua().await?;

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
        req = req
            .method(method)
            .uri(http::Uri::try_from(url.as_str()).unwrap());
        req = req.header(hyper::header::COOKIE, jar.join("; "));
        if let Some(s) = ua {
            req = req.header(hyper::header::USER_AGENT, s);
        }
        let req = before(req);
        let (tx, rx) = oneshot::channel();
        self.issue(Cmd::Raw { req, rsp: tx }).await?;
        match rx.await {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => unreachable!("Session ended prematurely: {:?}", e),
        }
    }

    /// Switches to the frame specified at the index.
    pub async fn enter_frame(mut self, index: Option<u16>) -> Result<Client, error::CmdError> {
        let params = SwitchToFrameParameters {
            id: index.map(FrameId::Short),
        };
        self.issue(WebDriverCommand::SwitchToFrame(params)).await?;
        Ok(self)
    }

    /// Switches to the parent of the frame the client is currently contained within.
    pub async fn enter_parent_frame(mut self) -> Result<Client, error::CmdError> {
        self.issue(WebDriverCommand::SwitchToParentFrame).await?;
        Ok(self)
    }

    /// Find an element on the page.
    pub async fn find(&mut self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        self.by(search.into()).await
    }

    /// Find elements on the page.
    pub async fn find_all(&mut self, search: Locator<'_>) -> Result<Vec<Element>, error::CmdError> {
        let res = self
            .issue(WebDriverCommand::FindElements(search.into()))
            .await?;
        let array = self.parse_lookup_all(res)?;
        Ok(array
            .into_iter()
            .map(move |e| Element {
                client: self.clone(),
                element: e,
            })
            .collect())
    }

    /// Wait for the given function to return `true` before proceeding.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub async fn wait_for<F, FF>(&mut self, mut is_ready: F) -> Result<(), error::CmdError>
    where
        F: FnMut(&mut Client) -> FF,
        FF: Future<Output = Result<bool, error::CmdError>>,
    {
        while !is_ready(self).await? {}
        Ok(())
    }

    /// Wait for the given element to be present on the page.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    pub async fn wait_for_find(&mut self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        let s: webdriver::command::LocatorParameters = search.into();
        loop {
            match self
                .by(webdriver::command::LocatorParameters {
                    using: s.using,
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

    /// Wait for the page to navigate to a new URL before proceeding.
    ///
    /// If the `current` URL is not provided, `self.current_url()` will be used. Note however that
    /// this introduces a race condition: the browser could finish navigating *before* we call
    /// `current_url()`, which would lead to an eternal wait.
    pub async fn wait_for_navigation(
        &mut self,
        current: Option<url::Url>,
    ) -> Result<(), error::CmdError> {
        let current = match current {
            Some(current) => current,
            None => self.current_url_().await?,
        };

        self.wait_for(move |c| {
            // TODO: get rid of this clone
            let current = current.clone();
            // TODO: and this one too
            let mut c = c.clone();
            async move { Ok(c.current_url().await? != current) }
        })
        .await
    }

    /// Locate a form on the page.
    ///
    /// Through the returned `Form`, HTML forms can be filled out and submitted.
    pub async fn form(&mut self, search: Locator<'_>) -> Result<Form, error::CmdError> {
        let l = search.into();
        let res = self.issue(WebDriverCommand::FindElement(l)).await?;
        let f = self.parse_lookup(res)?;
        Ok(Form {
            client: self.clone(),
            form: f,
        })
    }

    /// Gets the current window handle.
    pub async fn window(&mut self) -> Result<webdriver::common::WebWindow, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetWindowHandle).await?;
        match res {
            Json::String(x) => Ok(webdriver::common::WebWindow(x)),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Gets a list of all active windows (and tabs)
    pub async fn windows(&mut self) -> Result<Vec<webdriver::common::WebWindow>, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetWindowHandles).await?;
        match res {
            Json::Array(handles) => handles
                .into_iter()
                .map(|handle| match handle {
                    Json::String(x) => Ok(webdriver::common::WebWindow(x)),
                    v => Err(error::CmdError::NotW3C(v)),
                })
                .collect::<Result<Vec<_>, _>>(),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Switches to the chosen window.
    pub async fn switch_to_window(
        &mut self,
        window: webdriver::common::WebWindow,
    ) -> Result<(), error::CmdError> {
        let params = SwitchToWindowParameters { handle: window.0 };
        let _res = self.issue(WebDriverCommand::SwitchToWindow(params)).await?;
        Ok(())
    }

    /// Closes the current window.
    ///
    /// Will close the session if no other windows exist.
    ///
    /// Closing a window will not switch the client to one of the remaining windows.
    /// The switching must be done by calling `switch_to_window` using a still live window
    /// after the current window has been closed.
    pub async fn close_window(&mut self) -> Result<(), error::CmdError> {
        let _res = self.issue(WebDriverCommand::CloseWindow).await?;
        Ok(())
    }

    /// Creates a new window. If `is_tab` is `true`, then a tab will be created instead.
    ///
    /// Requires geckodriver > 0.24 and firefox > 66
    ///
    /// Windows are treated the same as tabs by the webdriver protocol.
    /// The functions `new_window`, `switch_to_window`, `close_window`, `window` and `windows`
    /// all operate on both tabs and windows.
    pub async fn new_window(
        &mut self,
        as_tab: bool,
    ) -> Result<webdriver::response::NewWindowResponse, error::CmdError> {
        let type_hint = if as_tab { "tab" } else { "window" }.to_string();
        let type_hint = Some(type_hint);
        let params = NewWindowParameters { type_hint };
        match self.issue(WebDriverCommand::NewWindow(params)).await? {
            Json::Object(mut obj) => {
                let handle = match obj
                    .remove("handle")
                    .and_then(|x| x.as_str().map(String::from))
                {
                    Some(handle) => handle,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let typ = match obj
                    .remove("type")
                    .and_then(|x| x.as_str().map(String::from))
                {
                    Some(typ) => typ,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                Ok(webdriver::response::NewWindowResponse { handle, typ })
            }
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    // helpers

    async fn by(
        &mut self,
        locator: webdriver::command::LocatorParameters,
    ) -> Result<Element, error::CmdError> {
        let res = self.issue(WebDriverCommand::FindElement(locator)).await?;
        let e = self.parse_lookup(res)?;
        Ok(Element {
            client: self.clone(),
            element: e,
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
                return Ok(webdriver::common::WebElement(wei));
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
    pub async fn attr(&mut self, attribute: &str) -> Result<Option<String>, error::CmdError> {
        let cmd =
            WebDriverCommand::GetElementAttribute(self.element.clone(), attribute.to_string());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up a DOM [property] for this element by name.
    ///
    /// `Ok(None)` is returned if the element does not have the given property.
    ///
    /// [property]: https://www.ecma-international.org/ecma-262/5.1/#sec-8.12.1
    pub async fn prop(&mut self, prop: &str) -> Result<Option<String>, error::CmdError> {
        let cmd = WebDriverCommand::GetElementProperty(self.element.clone(), prop.to_string());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Retrieve the text contents of this elment.
    pub async fn text(&mut self) -> Result<String, error::CmdError> {
        let cmd = WebDriverCommand::GetElementText(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
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
    pub async fn html(&mut self, inner: bool) -> Result<String, error::CmdError> {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        Ok(self.prop(prop).await?.unwrap())
    }

    /// Find the first matching descendant element.
    pub async fn find(&mut self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        let res = self
            .client
            .issue(WebDriverCommand::FindElementElement(
                self.element.clone(),
                search.into(),
            ))
            .await?;
        let e = self.client.parse_lookup(res)?;
        Ok(Element {
            client: self.client.clone(),
            element: e,
        })
    }
    /// Find all matching descendant elements.
    pub async fn find_all(&mut self, search: Locator<'_>) -> Result<Vec<Element>, error::CmdError> {
        let res = self
            .client
            .issue(WebDriverCommand::FindElementElements(
                self.element.clone(),
                search.into(),
            ))
            .await?;
        let array = self.client.parse_lookup_all(res)?;
        Ok(array
            .into_iter()
            .map(move |e| Element {
                client: self.client.clone(),
                element: e,
            })
            .collect())
    }

    /// Simulate the user clicking on this element.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub async fn click(mut self) -> Result<Client, error::CmdError> {
        let cmd = WebDriverCommand::ElementClick(self.element);
        let r = self.client.issue(cmd).await?;
        if r.is_null() || r.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(self.client)
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }

    /// Clear the value prop of this element
    pub async fn clear(&mut self) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::ElementClear(self.element.clone());
        let r = self.client.issue(cmd).await?;
        if r.is_null() {
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }

    /// Simulate the user sending keys to an element.
    pub async fn send_keys(&mut self, text: &str) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::ElementSendKeys(
            self.element.clone(),
            SendKeysParameters {
                text: text.to_owned(),
            },
        );
        let r = self.client.issue(cmd).await?;
        if r.is_null() {
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }

    /// Get back the [`Client`] hosting this `Element`.
    pub fn client(self) -> Client {
        self.client
    }

    /// Follow the `href` target of the element matching the given CSS selector *without* causing a
    /// click interaction.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub async fn follow(mut self) -> Result<Client, error::CmdError> {
        let cmd = WebDriverCommand::GetElementAttribute(self.element, "href".to_string());
        let href = self.client.issue(cmd).await?;
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

        let url = self.client.current_url_().await?;
        let href = url.join(&href)?;
        self.client.goto(href.as_str()).await?;
        Ok(self.client)
    }

    /// Find and click an `option` child element by its `value` attribute.
    pub async fn select_by_value(mut self, value: &str) -> Result<Client, error::CmdError> {
        let locator = format!("option[value='{}']", value);
        let locator = webdriver::command::LocatorParameters {
            using: webdriver::common::LocatorStrategy::CSSSelector,
            value: locator,
        };

        let cmd = WebDriverCommand::FindElementElement(self.element, locator);
        let v = self.client.issue(cmd).await?;
        Element {
            element: self.client.parse_lookup(v)?,
            client: self.client,
        }
        .click()
        .await
    }

    /// Switches to the frame contained within the element.
    pub async fn enter_frame(self) -> Result<Client, error::CmdError> {
        let Self {
            mut client,
            element,
        } = self;
        let params = SwitchToFrameParameters {
            id: Some(FrameId::Element(element)),
        };
        client
            .issue(WebDriverCommand::SwitchToFrame(params))
            .await?;
        Ok(client)
    }
}

impl Form {
    /// Find a form input using the given `locator` and set its value to `value`.
    pub async fn set(
        &mut self,
        locator: Locator<'_>,
        value: &str,
    ) -> Result<Self, error::CmdError> {
        let locator = WebDriverCommand::FindElementElement(self.form.clone(), locator.into());
        let value = Json::from(value);

        let res = self.client.issue(locator).await?;
        let field = self.client.parse_lookup(res)?;
        let mut args = vec![via_json!(&field), value];
        self.client.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "arguments[0].value = arguments[1]".to_string(),
            args: Some(args),
        };

        let res = self
            .client
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .await?;
        if res.is_null() {
            Ok(Form {
                client: self.client.clone(),
                form: self.form.clone(),
            })
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Find a form input with the given `name` and set its value to `value`.
    pub async fn set_by_name(&mut self, field: &str, value: &str) -> Result<Self, error::CmdError> {
        let locator = format!("input[name='{}']", field);
        let locator = Locator::Css(&locator);
        self.set(locator, value).await
    }

    /// Submit this form using the first available submit button.
    ///
    /// `false` is returned if no submit button was not found.
    pub async fn submit(self) -> Result<Client, error::CmdError> {
        self.submit_with(Locator::Css("input[type=submit],button[type=submit]"))
            .await
    }

    /// Submit this form using the button matched by the given selector.
    ///
    /// `false` is returned if a matching button was not found.
    pub async fn submit_with(mut self, button: Locator<'_>) -> Result<Client, error::CmdError> {
        let locator = WebDriverCommand::FindElementElement(self.form, button.into());
        let res = self.client.issue(locator).await?;
        let submit = self.client.parse_lookup(res)?;
        let res = self
            .client
            .issue(WebDriverCommand::ElementClick(submit))
            .await?;
        if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(self.client)
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form using the form submit button with the given label (case-insensitive).
    ///
    /// `false` is returned if a matching button was not found.
    pub async fn submit_using(self, button_label: &str) -> Result<Client, error::CmdError> {
        let escaped = button_label.replace('\\', "\\\\").replace('"', "\\\"");
        let btn = format!(
            "input[type=submit][value=\"{}\" i],\
             button[type=submit][value=\"{}\" i]",
            escaped, escaped
        );
        self.submit_with(Locator::Css(&btn)).await
    }

    /// Submit this form directly, without clicking any buttons.
    ///
    /// This can be useful to bypass forms that perform various magic when the submit button is
    /// clicked, or that hijack click events altogether (yes, I'm looking at you online
    /// advertisement code).
    ///
    /// Note that since no button is actually clicked, the `name=value` pair for the submit button
    /// will not be submitted. This can be circumvented by using `submit_sneaky` instead.
    pub async fn submit_direct(mut self) -> Result<Client, error::CmdError> {
        let mut args = vec![via_json!(&self.form)];
        self.client.fixup_elements(&mut args);
        // some sites are silly, and name their submit button "submit". this ends up overwriting
        // the "submit" function of the form with a reference to the submit button itself, so we
        // can't call .submit(). we get around this by creating a *new* form, and using *its*
        // submit() handler but with this pointed to the real form. solution from here:
        // https://stackoverflow.com/q/833032/472927#comment23038712_834197
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "document.createElement('form').submit.call(arguments[0])".to_string(),
            args: Some(args),
        };

        let res = self
            .client
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .await?;
        if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(self.client)
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
    pub async fn submit_sneaky(
        mut self,
        field: &str,
        value: &str,
    ) -> Result<Client, error::CmdError> {
        let mut args = vec![via_json!(&self.form), Json::from(field), Json::from(value)];
        self.client.fixup_elements(&mut args);
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

        let res = self
            .client
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .await?;
        if res.is_null() | res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Form {
                form: self.form,
                client: self.client,
            }
            .submit_direct()
            .await
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Get back the [`Client`] hosting this `Form`.
    pub fn client(self) -> Client {
        self.client
    }
}
