//! WebDriver types and declarations.

use crate::error;
#[cfg(doc)]
use crate::Client;
use http::Method;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Debug;
use std::time::Duration;
use url::{ParseError, Url};
use webdriver::command::TimeoutsParameters;

/// A command that can be sent to the WebDriver.
///
/// Anything that implements this command can be sent to [`Client::issue_cmd()`] in order
/// to send custom commands to the WebDriver instance.
pub trait WebDriverCompatibleCommand: Debug {
    /// The endpoint to send the request to.
    fn endpoint(
        &self,
        base_url: &url::Url,
        session_id: Option<&str>,
    ) -> Result<url::Url, url::ParseError>;

    /// The HTTP request method to use, and the request body for the request.
    ///
    /// The `url` will be the one returned from the `endpoint()` method above.
    fn method_and_body(&self, request_url: &url::Url) -> (http::Method, Option<String>);

    /// Return true if this command starts a new WebDriver session.
    fn is_new_session(&self) -> bool {
        false
    }

    /// Return true if this session should only support the legacy webdriver protocol.
    ///
    /// This only applies to the obsolete JSON Wire Protocol and should return `false`
    /// for all implementations that follow the W3C specification.
    ///
    /// See <https://www.selenium.dev/documentation/legacy/json_wire_protocol/> for more
    /// details about JSON Wire Protocol.
    fn is_legacy(&self) -> bool {
        false
    }
}

/// Blanket implementation for &T, for better ergonomics.
impl<T> WebDriverCompatibleCommand for &T
where
    T: WebDriverCompatibleCommand,
{
    fn endpoint(&self, base_url: &Url, session_id: Option<&str>) -> Result<Url, ParseError> {
        T::endpoint(self, base_url, session_id)
    }

    fn method_and_body(&self, request_url: &Url) -> (Method, Option<String>) {
        T::method_and_body(self, request_url)
    }

    fn is_new_session(&self) -> bool {
        T::is_new_session(self)
    }

    fn is_legacy(&self) -> bool {
        T::is_legacy(self)
    }
}

/// Blanket implementation for Box<T>, for better ergonomics.
impl<T> WebDriverCompatibleCommand for Box<T>
where
    T: WebDriverCompatibleCommand,
{
    fn endpoint(&self, base_url: &Url, session_id: Option<&str>) -> Result<Url, ParseError> {
        T::endpoint(self, base_url, session_id)
    }

    fn method_and_body(&self, request_url: &Url) -> (Method, Option<String>) {
        T::method_and_body(self, request_url)
    }

    fn is_new_session(&self) -> bool {
        T::is_new_session(self)
    }

    fn is_legacy(&self) -> bool {
        T::is_legacy(self)
    }
}

/// A [handle][1] to a browser window.
///
/// Should be obtained it via [`Client::window()`] method (or similar).
///
/// [1]: https://www.w3.org/TR/webdriver/#dfn-window-handles
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowHandle(String);

impl From<WindowHandle> for String {
    fn from(w: WindowHandle) -> Self {
        w.0
    }
}

impl<'a> TryFrom<Cow<'a, str>> for WindowHandle {
    type Error = error::InvalidWindowHandle;

    /// Makes the given string a [`WindowHandle`].
    ///
    /// Avoids allocation if possible.
    ///
    /// # Errors
    ///
    /// If the given string is [`"current"`][1].
    ///
    /// [1]: https://www.w3.org/TR/webdriver/#dfn-window-handles
    fn try_from(s: Cow<'a, str>) -> Result<Self, Self::Error> {
        if s != "current" {
            Ok(Self(s.into_owned()))
        } else {
            Err(error::InvalidWindowHandle)
        }
    }
}

impl TryFrom<String> for WindowHandle {
    type Error = error::InvalidWindowHandle;

    /// Makes the given [`String`] a [`WindowHandle`].
    ///
    /// # Errors
    ///
    /// If the given [`String`] is [`"current"`][1].
    ///
    /// [1]: https://www.w3.org/TR/webdriver/#dfn-window-handles
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Owned(s))
    }
}

impl TryFrom<&str> for WindowHandle {
    type Error = error::InvalidWindowHandle;

    /// Makes the given string a [`WindowHandle`].
    ///
    /// Allocates if succeeds.
    ///
    /// # Errors
    ///
    /// If the given string is [`"current"`][1].
    ///
    /// [1]: https://www.w3.org/TR/webdriver/#dfn-window-handles
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Borrowed(s))
    }
}

/// A type of a new browser window.
///
/// Returned by [`Client::new_window()`] method.
///
/// [`Client::new_window()`]: crate::Client::new_window
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewWindowType {
    /// Opened in a tab.
    Tab,

    /// Opened in a separate window.
    Window,
}

impl fmt::Display for NewWindowType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tab => write!(f, "tab"),
            Self::Window => write!(f, "window"),
        }
    }
}

/// Dynamic set of [WebDriver capabilities][1].
///
/// [1]: https://www.w3.org/TR/webdriver/#dfn-capability
pub type Capabilities = serde_json::Map<String, serde_json::Value>;

/// An element locator.
///
/// See [the specification][1] for more details.
///
/// [1]: https://www.w3.org/TR/webdriver1/#locator-strategies
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum Locator<'a> {
    /// Find an element matching the given [CSS selector][1].
    ///
    /// [1]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
    Css(&'a str),

    /// Find an element using the given [`id`][1].
    ///
    /// [1]: https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/id
    Id(&'a str),

    /// Find a link element with the given link text.
    ///
    /// The text matching is exact.
    LinkText(&'a str),

    /// Find an element using the given [XPath expression][1].
    ///
    /// You can address pretty much any element this way, if you're willing to
    /// put in the time to find the right XPath.
    ///
    /// [1]: https://developer.mozilla.org/en-US/docs/Web/XPath
    XPath(&'a str),
}

impl<'a> Locator<'a> {
    pub(crate) fn into_parameters(self) -> webdriver::command::LocatorParameters {
        use webdriver::command::LocatorParameters;
        use webdriver::common::LocatorStrategy;

        match self {
            Locator::Css(s) => LocatorParameters {
                using: LocatorStrategy::CSSSelector,
                value: s.to_string(),
            },
            Locator::Id(s) => LocatorParameters {
                using: LocatorStrategy::XPath,
                value: format!("//*[@id=\"{}\"]", s),
            },
            Locator::XPath(s) => LocatorParameters {
                using: LocatorStrategy::XPath,
                value: s.to_string(),
            },
            Locator::LinkText(s) => LocatorParameters {
                using: LocatorStrategy::LinkText,
                value: s.to_string(),
            },
        }
    }
}

/// The WebDriver status as returned by [`Client::status()`].
///
/// See [8.3 Status](https://www.w3.org/TR/webdriver1/#status) of the WebDriver standard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDriverStatus {
    /// True if the webdriver is ready to start a new session.
    ///
    /// NOTE: Geckodriver will return `false` if a session has already started, since it
    ///       only supports a single session.
    pub ready: bool,
    /// The current status message.
    pub message: String,
}

/// Timeout configuration, for various timeout settings.
///
/// Used by [`Client::get_timeouts()`] and [`Client::update_timeouts()`].
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimeoutConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<u64>,
    #[serde(rename = "pageLoad", skip_serializing_if = "Option::is_none")]
    page_load: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    implicit: Option<u64>,
}

impl Default for TimeoutConfiguration {
    fn default() -> Self {
        TimeoutConfiguration::new(
            Some(Duration::from_secs(60)),
            Some(Duration::from_secs(60)),
            Some(Duration::from_secs(0)),
        )
    }
}

impl TimeoutConfiguration {
    /// Create new timeout configuration.
    ///
    /// The various settings are as follows:
    /// - script     Determines when to interrupt a script that is being evaluated.
    ///              Default is 60 seconds.
    /// - page_load  Provides the timeout limit used to interrupt navigation of the browsing
    ///              context. Default is 60 seconds.
    /// - implicit   Gives the timeout of when to abort locating an element. Default is 0 seconds.
    ///
    /// NOTE: It is recommended to leave the `implicit` timeout at 0 seconds, because that makes
    ///       it possible to check for the non-existence of an element without an implicit delay.
    ///       Also see [`Client::wait()`] for element polling functionality.
    pub fn new(
        script: Option<Duration>,
        page_load: Option<Duration>,
        implicit: Option<Duration>,
    ) -> Self {
        TimeoutConfiguration {
            script: script.map(|x| x.as_millis() as u64),
            page_load: page_load.map(|x| x.as_millis() as u64),
            implicit: implicit.map(|x| x.as_millis() as u64),
        }
    }

    /// Get the script timeout.
    pub fn script(&self) -> Option<Duration> {
        self.script.map(Duration::from_millis)
    }

    /// Set the script timeout.
    pub fn set_script(&mut self, timeout: Option<Duration>) {
        self.script = timeout.map(|x| x.as_millis() as u64);
    }

    /// Get the page load timeout.
    pub fn page_load(&self) -> Option<Duration> {
        self.page_load.map(Duration::from_millis)
    }

    /// Set the page load timeout.
    pub fn set_page_load(&mut self, timeout: Option<Duration>) {
        self.page_load = timeout.map(|x| x.as_millis() as u64);
    }

    /// Get the implicit wait timeout.
    pub fn implicit(&self) -> Option<Duration> {
        self.implicit.map(Duration::from_millis)
    }

    /// Set the implicit wait timeout.
    pub fn set_implicit(&mut self, timeout: Option<Duration>) {
        self.implicit = timeout.map(|x| x.as_millis() as u64);
    }
}

impl TimeoutConfiguration {
    pub(crate) fn into_params(self) -> TimeoutsParameters {
        TimeoutsParameters {
            script: self.script.map(Some),
            page_load: self.page_load,
            implicit: self.implicit,
        }
    }
}
