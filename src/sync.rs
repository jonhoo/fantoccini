//! This module wraps async Client, Form and Element with synchronous versions.

use tokio::runtime::current_thread::Runtime;
use crate::session;
use crate::error;

use webdriver::command::{SendKeysParameters, WebDriverCommand};
use webdriver::common::ELEMENT_KEY;
use webdriver::error::WebDriverError;

/// A WebDriver client tied to a single browser session.
pub struct Client {
    rt: Runtime,
    client: session::Client,
}

impl Client {
    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    ///
    /// Calls `with_capabilities` with an empty capabilities list.
    pub fn new(webdriver: &str) -> Result<Self, error::NewSessionError> {
        let mut rt = Runtime::new().unwrap();
        let client = rt.block_on(async {
            session::Client::new(webdriver).await
        }).unwrap();

        Ok(Self {
            rt,
            client,
        })
    }

    /// Create a new `Client` associated with a new WebDriver session on the server at the given
    /// URL.
    ///
    /// The given capabilities will be requested in `alwaysMatch` or `desiredCapabilities`
    /// depending on the protocol version supported by the server.
    ///
    /// Returns a handle for issuing additional WebDriver tasks.
    ///
    /// Note that most callers should explicitly call `Client::close`. If `close` is not 
    /// explicitly called, a session close request will be spawned on the given `handle` 
    /// when the last instance of this `Client` is dropped.
    pub fn with_capabilities(webdriver: &str, cap: webdriver::capabilities::Capabilities,) -> Result<Self, error::NewSessionError> {
        let mut rt = Runtime::new().unwrap();
        let client = rt.block_on(async {
            session::Session::with_capabilities(webdriver, cap).await
        });
        let client = match client {
            Ok(new_client) => new_client,
            Err(error) => return Err(error), 
        };
        Ok(Self {
            rt,
            client,
        })
    }

    /// Navigate directly to the given URL.
    pub fn goto(&mut self, url: &str) -> Result<(), error::CmdError> {
        let client = &mut self.client;
        self.rt.block_on(async {
            client.goto(url).await
        })
    }
}