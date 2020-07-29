/// The module contains a wait primitive
use crate::Client;
use std::future::Future;
use std::time::Duration;

const DEFAULT_POOLING_INTERVAL: Duration = Duration::from_millis(500);

///
#[derive(Debug)]
pub struct Wait<'a> {
    waiter: DefaultWait<&'a mut Client>,
}

impl<'a> Wait<'a> {
    ///
    pub fn new(client: &'a mut Client, timeout: Duration) -> Self {
        Self {
            waiter: DefaultWait::new(client, timeout, DEFAULT_POOLING_INTERVAL),
        }
    }

    pub(crate) async fn until<F, FF, R>(&mut self, condition: F) -> Result<R, Duration>
    where
        F: FnMut(&mut &mut Client) -> FF,
        FF: Future<Output = Option<R>>,
    {
        self.waiter.until(condition).await
    }
}

///
#[derive(Debug)]
pub(crate) struct DefaultWait<T> {
    input: T,
    timeout: Duration,
    pooling_interval: Duration,
}

impl<T> DefaultWait<T> {
    ///
    pub(crate) fn new(input: T, timeout: Duration, pooling_interval: Duration) -> Self {
        Self {
            input,
            timeout,
            pooling_interval,
        }
    }

    ///
    pub(crate) async fn until<F, FF, R>(&mut self, mut condition: F) -> Result<R, Duration>
    where
        F: FnMut(&mut T) -> FF,
        FF: Future<Output = Option<R>>,
    {
        let now = std::time::Instant::now();

        loop {
            match condition(&mut self.input).await {
                Some(result) => return Ok(result),
                None => (),
            }

            if now.elapsed() > self.timeout {
                return Err(now.elapsed() - self.timeout)?;
            }

            tokio::time::delay_for(self.pooling_interval).await;
        }
    }
}
