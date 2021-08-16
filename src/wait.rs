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
//! methods or use [`Wait::forver`] to wait indefinitely.
//!
//! Once configured, you can start waiting on some condition by using the [`Wait::on`] method. It
//! accepts any type implementing the [`WaitCondition`] trait. For example:
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
//! let button = client.wait().on(Locator::Css(
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
//!
//! # Custom conditions
//!
//! You can implement custom conditions either by implementing the [`WaitCondition`] trait or by
//! using closures. Due to lifetime and async trait difficulties, two newtypes and two dedicated
//! functions exists to simplify the usage of closures. Also see: [`Closure`], [`Predicate`].

use crate::error::CmdError;
use crate::{elements::Element, error, Client, Locator};
use futures_util::TryFutureExt;
use std::{
    future::Future,
    pin::Pin,
    time::{Duration, Instant},
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_PERIOD: Duration = Duration::from_millis(250);

/// Used for setting up a wait operation on the client.
#[derive(Debug)]
pub struct Wait<'c> {
    client: &'c mut Client,
    timeout: Option<Duration>,
    period: Duration,
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
    /// let button = client.wait().on(Locator::Css(
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

    /// Wait until a condition exists or a timeout is hit.
    pub async fn until_some<T, F>(self, f: F) -> Result<T, error::CmdError>
    where
        F: for<'f> FnMut(
            &'f mut Client,
        ) -> Pin<
            Box<dyn Future<Output = Result<Option<T>, error::CmdError>> + 'f + Send>,
        >,
    {
        self.on(Condition(f)).await
    }

    /// Wait until a condition exists or a timeout is hit.
    pub async fn on<C, T>(self, mut condition: C) -> Result<T, error::CmdError>
    where
        C: for<'a> WaitCondition<'a, T>,
    {
        let start = Instant::now();
        loop {
            match self.timeout {
                Some(timeout) if start.elapsed() > timeout => {
                    break Err(error::CmdError::WaitTimeout)
                }
                _ => {}
            }
            match condition.ready(self.client).await? {
                Some(result) => break Ok(result),
                None => {
                    tokio::time::sleep(self.period).await;
                }
            };
        }
    }

    /// Wait for a predicate.
    pub async fn until<F>(self, predicate: F) -> Result<(), error::CmdError>
    where
        F: for<'f> FnMut(
            &'f mut Client,
        ) -> Pin<
            Box<dyn Future<Output = Result<bool, error::CmdError>> + 'f + Send>,
        >,
    {
        self.on(Predicate(predicate)).await
    }
}

/// A condition to wait for.
///
/// This is implemented by different types directly, like [`Locator`]. But you can also use
/// custom logic, using the [`Condition`] and [`Predicate`] newtypes.
pub trait WaitCondition<'a, T> {
    /// Check if the condition is ready. If it is, return `Some(...)`, otherwise return `None`.
    fn ready(
        &'a mut self,
        client: &'a mut Client,
    ) -> Pin<Box<dyn Future<Output = Result<Option<T>, error::CmdError>> + 'a + Send>>;
}

impl<'a> WaitCondition<'a, Element> for Locator<'_> {
    fn ready(
        &'a mut self,
        client: &'a mut Client,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Element>, CmdError>> + 'a + Send>> {
        let locator: webdriver::command::LocatorParameters = (*self).into();
        Box::pin(async move {
            match client.by(locator).await {
                Ok(element) => Ok(Some(element)),
                Err(error::CmdError::NoSuchElement(_)) => Ok(None),
                Err(err) => Err(err),
            }
        })
    }
}

impl<'a> WaitCondition<'a, ()> for url::Url {
    fn ready(
        &'a mut self,
        client: &'a mut Client,
    ) -> Pin<Box<dyn Future<Output = Result<Option<()>, CmdError>> + 'a + Send>> {
        Box::pin(async move {
            Ok(match &client.current_url().await? == self {
                true => Some(()),
                false => None,
            })
        })
    }
}

impl<'a, F, Fut, T> WaitCondition<'a, T> for F
where
    F: FnMut(&'a mut Client) -> Pin<Box<Fut>>,
    Fut: Future<Output = Result<Option<T>, error::CmdError>> + 'a + Send,
{
    fn ready(
        &'a mut self,
        client: &'a mut Client,
    ) -> Pin<Box<dyn Future<Output = Result<Option<T>, CmdError>> + 'a + Send>> {
        self(client)
    }
}

/// Newtype for condition functions.
///
/// A condition is successful once it returns some value. It will be re-tried as long as it
/// returns [`None`].
///
/// Instead of using the newtype, you can also use the [`Wait::on_condition`] method.
#[derive(Debug)]
pub struct Condition<F, T>(pub F)
where
    F: for<'a> FnMut(
        &'a mut Client,
    ) -> Pin<
        Box<dyn Future<Output = Result<Option<T>, error::CmdError>> + 'a + Send>,
    >;

impl<'a, F, T> WaitCondition<'a, T> for Condition<F, T>
where
    F: for<'f> FnMut(
        &'f mut Client,
    ) -> Pin<
        Box<dyn Future<Output = Result<Option<T>, error::CmdError>> + 'f + Send>,
    >,
{
    fn ready(
        &'a mut self,
        client: &'a mut Client,
    ) -> Pin<Box<dyn Future<Output = Result<Option<T>, CmdError>> + 'a + Send>> {
        self.0(client)
    }
}

/// Newtype for predicate functions.
///
/// A condition is successful once it returns `true`. It will be re-tried as long as it
/// returns `false`.
///
/// Instead of using the newtype, you can also use the [`Wait::on_predicate`] method.
#[derive(Debug)]
pub struct Predicate<F>(pub F)
where
    F: for<'a> FnMut(
        &'a mut Client,
    )
        -> Pin<Box<dyn Future<Output = Result<bool, error::CmdError>> + 'a + Send>>;

impl<'a, F> WaitCondition<'a, ()> for Predicate<F>
where
    F: for<'f> FnMut(
        &'f mut Client,
    )
        -> Pin<Box<dyn Future<Output = Result<bool, error::CmdError>> + 'f + Send>>,
{
    fn ready(
        &'a mut self,
        client: &'a mut Client,
    ) -> Pin<Box<dyn Future<Output = Result<Option<()>, CmdError>> + 'a + Send>> {
        Box::pin(self.0(client).map_ok(|result| match result {
            true => Some(()),
            false => None,
        }))
    }
}
