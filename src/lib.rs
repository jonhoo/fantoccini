//! A medium-level API for programmatically interacting with web pages through WebDriver.
//!
//! This crate uses the [WebDriver protocol] to drive a conforming (potentially headless) browser
//! through relatively operations such as "click this element", "submit this form", etc.
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
//! # Feature flags
//!
//! The following feature flags exist for this crate.
//!
//! - `native-tls`: Enable [ergonomic https connection](ClientBuilder::native) using [`native-tls`](https://crates.io/crates/native-tls) (enabled by default).
//! - `rustls-tls`: Enable [ergonomic https connection](ClientBuilder::rustls) using Rusttls.
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
//! use fantoccini::{ClientBuilder, Locator};
//!
//! // let's set up the sequence of steps we want the browser to take
//! #[tokio::main]
//! async fn main() -> Result<(), fantoccini::error::CmdError> {
//!     // Connecting using "native" TLS (with feature `native-tls`; on by default)
//!     # #[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
//!     let c = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//!     // Connecting using Rustls (with feature `rustls-tls`)
//!     # #[cfg(feature = "rustls-tls")]
//!     let c = ClientBuilder::rustls().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//!     # #[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
//!     # let c: fantoccini::Client = unreachable!("no tls provider available");
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
//! # use fantoccini::{ClientBuilder, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # #[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
//! # let c = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(feature = "rustls-tls")]
//! # let c = ClientBuilder::rustls().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
//! # let c: fantoccini::Client = unreachable!("no tls provider available");
//! // -- snip wrapper code --
//! // go to the Wikipedia frontpage this time
//! c.goto("https://www.wikipedia.org/").await?;
//! // find the search form, fill it out, and submit it
//! let f = c.form(Locator::Css("#search-form")).await?;
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
//! # use fantoccini::{ClientBuilder, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # #[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
//! # let c = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(feature = "rustls-tls")]
//! # let c = ClientBuilder::rustls().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
//! # let c: fantoccini::Client = unreachable!("no tls provider available");
//! // -- snip wrapper code --
//! // go back to the frontpage
//! c.goto("https://www.wikipedia.org/").await?;
//! // find the source for the Wikipedia globe
//! let img = c.find(Locator::Css("img.central-featured-logo")).await?;
//! let src = img.attr("src").await?.expect("image should have a src");
//! // now build a raw HTTP client request (which also has all current cookies)
//! let raw = img.client().raw_client_for(hyper::Method::GET, &src).await?;
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
#![warn(missing_debug_implementations, rust_2018_idioms, rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples, rustdoc::private_doc_tests)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use crate::wd::Capabilities;
use hyper::client::connect;

macro_rules! via_json {
    ($x:expr) => {{
        serde_json::from_str(&serde_json::to_string($x).unwrap()).unwrap()
    }};
}

/// Error types.
pub mod error;

/// The long-running session future we spawn for multiplexing onto a running WebDriver instance.
mod session;

/// A [builder] for WebDriver [`Client`] instances.
///
/// You will likely want to use [`native`](ClientBuilder::native) or
/// [`rustls`](ClientBuilder::rustls) (depending on your preference) to start the builder. If you
/// want to supply your own connector, use [`new`](ClientBuilder::new).
///
/// To connect to the WebDriver instance, call [`connect`](ClientBuilder::connect).
///
/// [builder]: https://rust-lang.github.io/api-guidelines/type-safety.html#c-builder
#[derive(Default, Clone, Debug)]
pub struct ClientBuilder<C>
where
    C: connect::Connect + Send + Sync + Clone + Unpin,
{
    capabilities: Option<Capabilities>,
    connector: C,
}

#[cfg(feature = "rustls-tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
impl ClientBuilder<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>> {
    /// Build a [`Client`] that will connect using [Rustls](https://crates.io/crates/rustls).
    pub fn rustls() -> Self {
        Self::new(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .build(),
        )
    }
}

#[cfg(feature = "native-tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
impl ClientBuilder<hyper_tls::HttpsConnector<hyper::client::HttpConnector>> {
    /// Build a [`Client`] that will connect using [`native-tls`](https://crates.io/crates/native-tls).
    pub fn native() -> Self {
        Self::new(hyper_tls::HttpsConnector::new())
    }
}
impl<C> ClientBuilder<C>
where
    C: connect::Connect + Send + Sync + Clone + Unpin + 'static,
{
    /// Build a [`Client`] that will connect using the given HTTP `connector`.
    pub fn new(connector: C) -> Self {
        Self {
            capabilities: None,
            connector,
        }
    }

    /// Pass the given [WebDriver capabilities][1] to the browser.
    ///
    /// The WebDriver specification has a list of [standard
    /// capabilities](https://www.w3.org/TR/webdriver1/#capabilities), which are given below. In
    /// addition, most browser vendors support a number of browser-specific capabilities stored
    /// in an object under a prefixed key like
    /// [`moz:firefoxOptions`](https://developer.mozilla.org/en-US/docs/Web/WebDriver/Capabilities/firefoxOptions)
    /// or
    /// [`goog:chromeOptions`](https://sites.google.com/a/chromium.org/chromedriver/capabilities).
    ///
    /// The standard options are given below. See the
    /// [specification](https://www.w3.org/TR/webdriver1/#capabilities) for more details.
    ///
    /// | Capability | Key | Value Type | Description |
    /// |------------|-----|------------|-------------|
    /// | Browser name | `"browserName"` | string | Identifies the user agent. |
    /// | Browser version | `"browserVersion"` | string | Identifies the version of the user agent. |
    /// | Platform name | `"platformName"` | string | Identifies the operating system of the endpoint node. |
    /// | Accept insecure TLS certificates | `"acceptInsecureCerts"` | boolean | Indicates whether untrusted and self-signed TLS certificates are implicitly trusted on navigation for the duration of the session. |
    /// | Page load strategy | `"pageLoadStrategy"` | string | Defines the current session’s page load strategy. |
    /// | Proxy configuration | `"proxy"` | JSON Object | Defines the current session’s proxy configuration. |
    /// | Window dimensioning/positioning | `"setWindowRect"` | boolean | Indicates whether the remote end supports all of the commands in Resizing and Positioning Windows. |
    /// | Session timeouts configuration | `"timeouts"` | JSON Object | Describes the timeouts imposed on certain session operations. |
    /// | Unhandled prompt behavior | `"unhandledPromptBehavior"` | string | Describes the current session’s user prompt handler. |
    ///
    /// [1]: https://www.w3.org/TR/webdriver/#dfn-capability
    pub fn capabilities(&mut self, cap: Capabilities) -> &mut Self {
        self.capabilities = Some(cap);
        self
    }

    /// Connect to the WebDriver session at the `webdriver` URL.
    pub async fn connect(&self, webdriver: &str) -> Result<Client, error::NewSessionError> {
        if let Some(ref cap) = self.capabilities {
            Client::with_capabilities_and_connector(webdriver, cap, self.connector.clone()).await
        } else {
            Client::new_with_connector(webdriver, self.connector.clone()).await
        }
    }
}

pub mod client;
#[doc(inline)]
pub use client::Client;

pub mod actions;
pub mod cookies;
pub mod elements;
pub mod key;

pub mod wait;

pub mod wd;
#[doc(inline)]
pub use wd::Locator;
