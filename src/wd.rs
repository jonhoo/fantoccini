//! WebDriver types and declarations.

use crate::error;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt;

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
