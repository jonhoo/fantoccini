use crate::{error, Client, Element, Locator};

use futures_util::future::{select, Either};
use futures_util::pin_mut;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::time::Instant;

use webdriver::command::{LocatorParameters, WebDriverCommand};
use webdriver::common::WebElement;

type PinBoxFut<T> = Pin<Box<dyn Future<Output = Result<T, error::CmdError>> + Send>>;
type PinMutFut<'a, T> = Pin<&'a mut (dyn Future<Output = Result<T, error::CmdError>> + Send)>;

mod sealed {
    use super::PinBoxFut;
    use crate::{error, Client};

    pub trait Command {
        type Output;
        fn invoke(&self, client: Client) -> PinBoxFut<Self::Output>;
        fn handle_error(error: error::CmdError) -> Result<(), error::CmdError>;
    }
}

use sealed::*;

/// TODO
#[derive(Debug)]
pub struct FindDescendant {
    search: LocatorParameters,
    element: WebElement,
}

impl Command for FindDescendant {
    type Output = Element;

    fn invoke(&self, mut client: Client) -> PinBoxFut<Element> {
        let search = LocatorParameters {
            using: self.search.using,
            value: self.search.value.clone(),
        };

        let cmd = WebDriverCommand::FindElementElement(self.element.clone(), search);

        Box::pin(async move {
            let res = client.issue(cmd).await?;
            let e = client.parse_lookup(res)?;

            Ok(Element { client, element: e })
        })
    }

    fn handle_error(error: error::CmdError) -> Result<(), error::CmdError> {
        match error {
            error::CmdError::NoSuchElement(_) => Ok(()),
            err => Err(err),
        }
    }
}

/// TODO
#[derive(Debug)]
pub struct Find(LocatorParameters);

impl Command for Find {
    type Output = Element;

    fn invoke(&self, mut client: Client) -> PinBoxFut<Element> {
        let locator = LocatorParameters {
            using: self.0.using,
            value: self.0.value.clone(),
        };

        Box::pin(async move { client.by(locator).await })
    }

    fn handle_error(error: error::CmdError) -> Result<(), error::CmdError> {
        match error {
            error::CmdError::NoSuchElement(_) => Ok(()),
            err => Err(err),
        }
    }
}

enum State<T>
where
    T: Command,
{
    Ready(T),
    Once(PinBoxFut<T::Output>),
}

impl<T> fmt::Debug for State<T>
where
    T: fmt::Debug + Command,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Ready(t) => write!(f, "State::Ready({:?})", t),
            State::Once(_) => write!(f, "State::Once(...)"),
        }
    }
}

impl<T> State<T>
where
    T: Command,
{
    fn once(&mut self) -> PinMutFut<'_, T::Output> {
        match self {
            State::Once(ref mut p) => p.as_mut(),
            _ => panic!(),
        }
    }
}

/// TODO
#[derive(Debug)]
pub struct Retry<T>
where
    T: Command,
{
    client: Client,
    state: State<T>,
}

impl<T> Future for Retry<T>
where
    T: Unpin + Command,
{
    type Output = Result<T::Output, error::CmdError>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let future = match this.state {
            State::Ready(ref mut factory) => {
                this.state = State::Once(factory.invoke(this.client.clone()));
                this.state.once()
            }
            State::Once(ref mut f) => f.as_mut(),
        };

        future.poll(ctx)
    }
}

impl Retry<FindDescendant> {
    pub(crate) fn find_descendant(
        client: Client,
        element: WebElement,
        search: Locator<'_>,
    ) -> Self {
        Self {
            client,
            state: State::Ready(FindDescendant {
                search: search.into(),
                element,
            }),
        }
    }
}

impl Retry<Find> {
    pub(crate) fn find(client: Client, locator: Locator<'_>) -> Self {
        Self {
            client,
            state: State::Ready(Find(locator.into())),
        }
    }
}

impl<T> Retry<T>
where
    T: Command,
{
    /// TODO
    pub async fn retry_forever(self) -> Result<T::Output, error::CmdError> {
        let factory = match self.state {
            State::Ready(f) => f,
            _ => panic!(),
        };

        loop {
            match factory.invoke(self.client.clone()).await {
                Ok(x) => return Ok(x),
                Err(e) => T::handle_error(e)?,
            }
        }
    }

    /// TODO
    pub async fn retry_for(self, duration: Duration) -> Result<T::Output, error::CmdError> {
        let a = self.retry_forever();
        let b = tokio::time::delay_for(duration);

        pin_mut!(a);

        match select(a, b).await {
            Either::Left(l) => l.0,
            Either::Right(_) => Err(error::CmdError::RetriesExhausted),
        }
    }

    /// TODO
    pub async fn retry_until(self, deadline: Instant) -> Result<T::Output, error::CmdError> {
        let a = self.retry_forever();
        let b = tokio::time::delay_until(deadline);

        pin_mut!(a);

        match select(a, b).await {
            Either::Left(l) => l.0,
            Either::Right(_) => Err(error::CmdError::RetriesExhausted),
        }
    }
}
