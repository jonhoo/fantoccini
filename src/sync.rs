use tokio::runtime::current_thread::Runtime;
use crate::session;
use crate::error;

use webdriver::command::{SendKeysParameters, WebDriverCommand};
use webdriver::common::ELEMENT_KEY;
use webdriver::error::WebDriverError;

/// sync client
pub struct Client {
    rt: Runtime,
    client: session::Client,
}

impl Client {
    /// create new sync client
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

    /// sync capabilities
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

    /// sync goto
    pub fn goto(&mut self, url: &str) -> Result<(), error::CmdError> {
        let client = &mut self.client;
        self.rt.block_on(async {
            client.goto(url).await
        })
    }
    
    // /// sync find
    // pub fn find(&mut self, search: crate::Locator) -> Result<Element, error::CmdError> {
    //     let client = &mut self.client;
    //     self.rt.block_on(async {
    //         client.find(search).await
    //     })
    // }

    /// sync wait for find
    pub fn wait_for_find(mut self, search: crate::Locator) -> Result<Element, error::CmdError> {
        let mut client = self.client.clone();
        let element = self.rt.block_on(async {
            client.wait_for_find(search).await
        });
        match element {
            Ok(element) => Ok(Element {client: self, element}),
            Err(error) => Err(error),
        }

    }


}

/// sync Element
pub struct Element {
    client: Client,
    // element: webdriver::common::WebElement,
    element: crate::Element,
}

impl Element { 
    /// click 
    pub fn click(mut self) -> Result<Client, error::CmdError> {
        let mut element = self.element;
        let client = self.client.rt.block_on(async {
            element.click().await
        });
        match client {
            Ok(client) => Ok(self.client),
            Err(error) => Err(error),
        }
        
    }
}
