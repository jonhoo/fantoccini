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
//! # Feature flags
//! - `rustls-tls`: Use rustls for https, enabled by default
//! - `openssl-tls`: Use openssl for https
//!
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
//! # use hyper::client::HttpConnector;
//! # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//! # use hyper_rustls::HttpsConnector;
//!
//! // let's set up the sequence of steps we want the browser to take
//! #[tokio::main]
//! async fn main() -> Result<(), fantoccini::error::CmdError> {
//!     // With --all-features
//!     # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//!     let mut c: Client<HttpsConnector<HttpConnector>> = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//!     // With either `rustls-tls` or `openssl-tls` enabled
//!     # #[cfg(any(all(feature = "rustls-tls", not(feature = "openssl-tls")), all(feature = "openssl-tls", not(feature = "rustls-tls"))))]
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
//! # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//! # use hyper_rustls::HttpsConnector;
//! # use hyper::client::HttpConnector;
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//! # let mut c: Client<HttpsConnector<HttpConnector>> = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(any(all(feature = "rustls-tls", not(feature = "openssl-tls")), all(feature = "openssl-tls", not(feature = "rustls-tls"))))]
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
//! # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//! # use hyper_rustls::HttpsConnector;
//! # use hyper::client::HttpConnector;
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # #[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
//! # let mut c: Client<HttpsConnector<HttpConnector>> = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(any(all(feature = "rustls-tls", not(feature = "openssl-tls")), all(feature = "openssl-tls", not(feature = "rustls-tls"))))]
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



pub use hyper::Method;

/// Error types.
pub mod error;

/// The long-running session future we spawn for multiplexing onto a running WebDriver instance.
mod session;
mod traits;
mod client;
pub use traits::{NewConnector, CapabilitiesExt};

/// Type alias for a Client with a Rustls connector
///
/// Available when only one of the two Https features is enabled
#[cfg(all(feature = "rustls-tls", not(feature = "openssl-tls")))]
pub type Client =  session::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>;

/// Type alias for a Client with an OpenSSL connector
///
/// Available when only one of the two Https features is enabled
#[cfg(all(feature = "openssl-tls", not(feature = "rustls-tls")))]
pub type Client = session::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;
#[cfg(all(feature = "rustls-tls", feature = "openssl-tls"))]
pub use session::Client;

pub use client::{Element, Form, Locator};