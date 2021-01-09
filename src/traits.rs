/// Trait implemented for HttpsConnectors
///
/// This trait is sealed
pub trait NewConnector: private::Sealed {
    /// Construct the HttpsConnector
    fn new() -> Self;
}

/// Convenient methods for webdriver capabilities
pub trait CapabilitiesExt {
    /// Run the geckodriver headless
    fn headless_firefox(self) -> Self;
    /// Run chrome headless
    fn headless_chrome(self) -> Self;
}

impl CapabilitiesExt for webdriver::capabilities::Capabilities {
    fn headless_firefox(mut self) -> Self {
        let arg = serde_json::json!({"args": ["-headless"]});
        self.insert("moz:firefoxOptions".to_string(), arg);
        self
    }
    fn headless_chrome(mut self) -> Self {
        let arg = serde_json::json!({"args": ["-headless"]});
        self.insert("goog:chromeOptions".to_string(), arg);
        self
    }
}

mod private {
    pub trait Sealed {}
    #[cfg(feature = "rustls-tls")]
    impl Sealed for hyper_rustls::HttpsConnector<hyper::client::HttpConnector> {}
    #[cfg(feature = "openssl-tls")]
    impl Sealed for hyper_tls::HttpsConnector<hyper::client::HttpConnector> {}

}