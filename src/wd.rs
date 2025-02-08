//! WebDriver types and declarations.
use crate::error;
#[cfg(doc)]
use crate::Client;
use http::Method;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use std::fmt::Debug;
use std::time::Duration;
use std::{borrow::Cow, ops::RangeInclusive};
use url::{ParseError, Url};
use webdriver::command::{PrintParameters, TimeoutsParameters};

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

/// The response obtained when opening the WebDriver session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NewSessionResponse {
    #[serde(rename = "sessionId")]
    session_id: String,
    capabilities: Option<Capabilities>,
}

impl NewSessionResponse {
    /// Get the session id.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the remote end capabilities.
    pub fn capabilities(&self) -> Option<&Capabilities> {
        self.capabilities.as_ref()
    }

    pub(crate) fn from_wd(nsr: webdriver::response::NewSessionResponse) -> Self {
        NewSessionResponse {
            session_id: nsr.session_id,
            capabilities: nsr.capabilities.as_object().cloned(),
        }
    }
}

/// The builder of [`PrintConfiguration`].
#[derive(Debug)]
pub struct PrintConfigurationBuilder {
    orientation: PrintOrientation,
    scale: f64,
    background: PrintBackground,
    size: PrintSize,
    margins: PrintMargins,
    page_ranges: Vec<PrintPageRange>,
    shrink_to_fit: bool,
}

impl Default for PrintConfigurationBuilder {
    fn default() -> Self {
        Self {
            orientation: PrintOrientation::default(),
            scale: 1.0,
            background: PrintBackground::default(),
            size: PrintSize::default(),
            margins: PrintMargins::default(),
            page_ranges: Vec::default(),
            shrink_to_fit: true,
        }
    }
}

impl PrintConfigurationBuilder {
    /// Builds the [`PrintConfiguration`].
    ///
    /// Returns None if:
    ///  - the scale, the margins or the size are infinite, NaN or negative
    ///  - the margins overflow the size (e.g. margins.left + margins.right >= page.width)
    pub fn build(self) -> Option<PrintConfiguration> {
        let must_be_finite_and_positive = [
            self.scale,
            self.margins.top,
            self.margins.left,
            self.margins.right,
            self.margins.bottom,
            self.size.width,
            self.size.height,
        ];
        if !must_be_finite_and_positive
            .into_iter()
            .all(|n| n.is_finite() && n.is_sign_positive())
        {
            return None;
        }

        if (self.margins.top + self.margins.bottom) >= self.size.height
            || (self.margins.left + self.margins.right) >= self.size.width
        {
            return None;
        }

        Some(PrintConfiguration {
            orientation: self.orientation,
            scale: self.scale,
            background: self.background,
            size: self.size,
            margins: self.margins,
            page_ranges: self.page_ranges,
            shrink_to_fit: self.shrink_to_fit,
        })
    }

    /// Sets the orientation of the printed page.
    ///
    /// Default: [`PrintOrientation::Portrait`].
    pub fn orientation(mut self, orientation: PrintOrientation) -> Self {
        self.orientation = orientation;

        self
    }

    /// Sets the scale of the printed page.
    ///
    /// Default: 1.
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;

        self
    }

    /// Sets whether or not to print the backgrounds of the page.
    ///
    /// Default: [`PrintBackground::Exclude`].
    pub fn background(mut self, background: PrintBackground) -> Self {
        self.background = background;

        self
    }

    /// Sets the size of the printed page.
    ///
    /// Default: `21.59x27.79 cm`.
    pub fn size(mut self, size: PrintSize) -> Self {
        self.size = size;

        self
    }

    /// Sets the margins of the printed page.
    ///
    /// Default: `1x1x1x1 cm`.
    pub fn margins(mut self, margins: PrintMargins) -> Self {
        self.margins = margins;

        self
    }

    /// Sets ranges of pages to print.
    ///
    /// An empty `ranges` prints all pages.
    /// Default: all.
    pub fn page_ranges(mut self, ranges: Vec<PrintPageRange>) -> Self {
        self.page_ranges = ranges;

        self
    }

    /// Sets whether or not to resize the content to fit the page width,
    /// overriding any page width specified in the content of pages to print.
    ///
    /// Default: true.
    pub fn shrink_to_fit(mut self, shrink_to_fit: bool) -> Self {
        self.shrink_to_fit = shrink_to_fit;

        self
    }
}

/// The print configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintConfiguration {
    orientation: PrintOrientation,
    scale: f64,
    background: PrintBackground,
    size: PrintSize,
    margins: PrintMargins,
    page_ranges: Vec<PrintPageRange>,
    shrink_to_fit: bool,
}

impl PrintConfiguration {
    /// Creates a [`PrintConfigurationBuilder`] to configure a [`PrintConfiguration`].
    pub fn builder() -> PrintConfigurationBuilder {
        PrintConfigurationBuilder::default()
    }

    pub(crate) fn into_params(self) -> PrintParameters {
        PrintParameters {
            orientation: self.orientation.into_params(),
            scale: self.scale,
            background: self.background.into_params(),
            page: self.size.into_params(),
            margin: self.margins.into_params(),
            page_ranges: self
                .page_ranges
                .into_iter()
                .map(|page_range| page_range.into_params())
                .collect(),
            shrink_to_fit: self.shrink_to_fit,
        }
    }
}

impl Default for PrintConfiguration {
    fn default() -> Self {
        PrintConfiguration {
            orientation: PrintOrientation::default(),
            scale: 1.0,
            background: PrintBackground::default(),
            size: PrintSize::default(),
            margins: PrintMargins::default(),
            page_ranges: Vec::new(),
            shrink_to_fit: true,
        }
    }
}

/// The orientation of the print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintOrientation {
    /// Landscape orientation.
    Landscape,
    #[default]
    /// Portrait orientation.
    Portrait,
}

impl PrintOrientation {
    pub(crate) fn into_params(self) -> webdriver::command::PrintOrientation {
        match self {
            Self::Landscape => webdriver::command::PrintOrientation::Landscape,
            Self::Portrait => webdriver::command::PrintOrientation::Portrait,
        }
    }
}

/// Whether to print backgrounds or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintBackground {
    /// Include the backgrounds in the print.
    Include,
    #[default]
    /// Exclude the backgrounds from the print.
    Exclude,
}

impl PrintBackground {
    pub(crate) fn into_params(self) -> bool {
        match self {
            Self::Include => true,
            Self::Exclude => false,
        }
    }
}

/// The size of the printed page in centimeters.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintSize {
    /// The width in centimeters.
    pub width: f64,
    /// The height in centimeters.
    pub height: f64,
}

impl PrintSize {
    pub(crate) fn into_params(self) -> webdriver::command::PrintPage {
        webdriver::command::PrintPage {
            width: self.width,
            height: self.height,
        }
    }
}

impl Default for PrintSize {
    fn default() -> Self {
        Self {
            width: 21.59,
            height: 27.94,
        }
    }
}

/// The range of the pages to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintPageRange {
    range: RangeInclusive<u64>,
}

impl PrintPageRange {
    /// A single page to print.
    pub const fn single(page: u64) -> Self {
        Self { range: page..=page }
    }

    /// A range of pages to print.
    ///
    /// Returns None if the range start is greater than the range end.
    pub const fn range(range: RangeInclusive<u64>) -> Option<Self> {
        if *range.start() <= *range.end() {
            Some(Self { range })
        } else {
            None
        }
    }

    pub(crate) fn into_params(self) -> webdriver::command::PrintPageRange {
        let (start, end) = self.range.into_inner();

        if start == end {
            webdriver::command::PrintPageRange::Integer(start)
        } else {
            webdriver::command::PrintPageRange::Range(format!("{start}-{end}"))
        }
    }
}

/// The margins of the printed page in centimeters.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintMargins {
    /// The top margin in centimeters.
    pub top: f64,
    /// The bottom margin in centimeters.
    pub bottom: f64,
    /// The left margin in centimeters.
    pub left: f64,
    /// The right margin in centimeters.
    pub right: f64,
}

impl PrintMargins {
    pub(crate) fn into_params(self) -> webdriver::command::PrintMargins {
        webdriver::command::PrintMargins {
            top: self.top,
            bottom: self.bottom,
            left: self.left,
            right: self.right,
        }
    }
}

impl Default for PrintMargins {
    fn default() -> Self {
        Self {
            top: 1.0,
            bottom: 1.0,
            left: 1.0,
            right: 1.0,
        }
    }
}
