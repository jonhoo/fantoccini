//! Allow to wait for conditions.
//!
//! Sometimes it is necessary to wait for a browser to achieve a certain state. For example,
//! navigating to a page may be take bit of time. And the time may vary between different
//! environments and test runs. Static delays can work around this issue, but also prolong the
//! test runs unnecessarily. Longer delays have less flaky tests, but even more unnecessary wait
//! time.
//!
//! To wait as optimal as possible, you can use asynchronous wait operations, which periodically
//! check for the expected state, re-try if necessary, but also fail after a certain time and still
//! allow you to fail the test. Allow for longer grace periods, and only spending the time waiting
//! when necessary.
//!
//! # Basic usage
//!
//! By default all wait operations will time-out after 30 seconds and will re-check every
//! 250 milliseconds. You can configure this using the [`Wait::at_most`] and [`Wait::every`]
//! methods or use [`Wait::forever`] to wait indefinitely.
//!
//! Once configured, you can start waiting on some condition by using the `Wait::for_*` methods.
//! For example:
//!
//! ```no_run
//! # use fantoccini::{ClientBuilder, Locator};
//! # #[tokio::main]
//! # async fn main() -> Result<(), fantoccini::error::CmdError> {
//! # #[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
//! # let mut client = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(feature = "rustls-tls")]
//! # let mut client = ClientBuilder::rustls().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
//! # #[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
//! # let mut client: fantoccini::Client = unreachable!("no tls provider available");
//! // -- snip wrapper code --
//! let button = client.wait().for_element(Locator::Css(
//!     r#"a.button-download[href="/learn/get-started"]"#,
//! )).await?;
//! // -- snip wrapper code --
//! # client.close().await
//! # }
//! ```
//!
//! # Error handling
//!
//! When a wait operation times out, it will return a [`CmdError::WaitTimeout`]. When a wait
//! condition check returns an error, the wait operation will be aborted, and the error returned.

use crate::elements::Element;
use crate::error::CmdError;
use crate::wd::Locator;
use crate::Client;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_PERIOD: Duration = Duration::from_millis(250);

/// Used for setting up a wait operation on the client.
#[derive(Debug)]
pub struct Wait<'c> {
    client: &'c mut Client,
    timeout: Option<Duration>,
    period: Duration,
}

macro_rules! wait_on {
    ($self:ident, $ready:expr) => {{
        let start = Instant::now();
        loop {
            match $self.timeout {
                Some(timeout) if start.elapsed() > timeout => break Err(CmdError::WaitTimeout),
                _ => {}
            }
            match $ready? {
                Some(result) => break Ok(result),
                None => {
                    tokio::time::sleep($self.period).await;
                }
            };
        }
    }};
}

impl<'c> Wait<'c> {
    /// Create a new wait operation from a client.
    ///
    /// This only starts the process of building a new wait operation. Waiting, and checking, will
    /// only begin once one of the consuming methods has been called.
    ///
    /// ```no_run
    /// # use fantoccini::{ClientBuilder, Locator};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fantoccini::error::CmdError> {
    /// # #[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
    /// # let mut client = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
    /// # #[cfg(feature = "rustls-tls")]
    /// # let mut client = ClientBuilder::rustls().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
    /// # #[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
    /// # let mut client: fantoccini::Client = unreachable!("no tls provider available");
    /// // -- snip wrapper code --
    /// let button = client.wait().for_element(Locator::Css(
    ///     r#"a.button-download[href="/learn/get-started"]"#,
    /// )).await?;
    /// // -- snip wrapper code --
    /// # client.close().await
    /// # }
    /// ```
    pub fn new(client: &'c mut Client) -> Self {
        Self {
            client,
            timeout: Some(DEFAULT_TIMEOUT),
            period: DEFAULT_PERIOD,
        }
    }

    /// Set the timeout until the operation should wait.
    pub fn at_most(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Wait forever.
    pub fn forever(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Sets the period to delay checks.
    pub fn every(mut self, period: Duration) -> Self {
        self.period = period;
        self
    }

    /// Wait until a particular element can be found.
    pub async fn for_element(self, search: Locator<'_>) -> Result<Element, CmdError> {
        wait_on!(self, {
            match self.client.by(search.into_parameters()).await {
                Ok(element) => Ok(Some(element)),
                Err(CmdError::NoSuchElement(_)) => Ok(None),
                Err(err) => Err(err),
            }
        })
    }

    /// Wait until a given URL is reached.
    pub async fn for_url(self, url: url::Url) -> Result<(), CmdError> {
        wait_on!(self, {
            Ok::<_, CmdError>(if self.client.current_url().await? == url {
                Some(())
            } else {
                None
            })
        })
    }
}
