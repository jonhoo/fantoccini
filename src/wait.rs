use crate::{elements::Element, error, Client, Locator};
use core::fmt;
use futures_util::TryFutureExt;
use std::{
    error::Error,
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
    pub async fn on<T, F>(self, mut f: F) -> Result<T, WaitError>
    where
        F: for<'f> FnMut(
            &'f mut Client,
        )
            -> Pin<Box<dyn Future<Output = Result<Option<T>, error::CmdError>> + 'f>>,
    {
        let start = Instant::now();
        loop {
            match self.timeout {
                Some(timeout) if start.elapsed() > timeout => break Err(WaitError::Timeout),
                _ => {}
            }
            match f(self.client).await? {
                Some(result) => break Ok(result),
                None => {
                    tokio::time::sleep(self.period).await;
                }
            };
        }
    }

    /// Wait for an element.
    pub async fn on_element(self, locator: Locator<'static>) -> Result<Element, WaitError> {
        self.on(move |client| {
            Box::pin(async move {
                match client.find(locator).await {
                    Ok(element) => Ok(Some(element)),
                    Err(error::CmdError::NoSuchElement(_)) => Ok(None),
                    Err(err) => Err(err),
                }
            })
        })
        .await
    }

    /// Wait for a predicate.
    pub async fn on_predicate<F>(self, mut predicate: F) -> Result<(), WaitError>
    where
        F: for<'f> FnMut(
            &'f mut Client,
        )
            -> Pin<Box<dyn Future<Output = Result<bool, error::CmdError>> + 'f>>,
    {
        self.on(|client| {
            Box::pin(predicate(client).map_ok(|result| match result {
                true => Some(()),
                false => None,
            }))
        })
        .await
    }
}

/// Allow creating a wait operation.
pub trait CanWait {
    /// Create a new wait operation.
    fn wait(&mut self) -> Wait<'_>;
}

impl CanWait for Client {
    fn wait(&mut self) -> Wait<'_> {
        Wait::new(self)
    }
}

/// An error that can occur when waiting.
#[derive(Debug)]
pub enum WaitError {
    /// The wait operation timed out
    Timeout,
    /// The client reported an error
    Client(error::CmdError),
}

impl Error for WaitError {
    fn description(&self) -> &str {
        match self {
            Self::Timeout => "timeout waiting on condition",
            Self::Client(..) => "webdriver returned error",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match self {
            Self::Timeout => None,
            Self::Client(err) => Some(err),
        }
    }
}

impl fmt::Display for WaitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "Timeout"),
            Self::Client(cmd) => write!(f, "Client error: {}", cmd),
        }
    }
}

impl From<error::CmdError> for WaitError {
    fn from(err: error::CmdError) -> Self {
        Self::Client(err)
    }
}
