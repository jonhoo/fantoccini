use hyper::Error as HError;
use serde::Serialize;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::io::Error as IOError;
use url::ParseError;
use webdriver::error as webdriver;

/// An error occurred while attempting to establish a session for a new `Client`.
#[derive(Debug)]
pub enum NewSessionError {
    /// The given WebDriver URL is invalid.
    BadWebdriverUrl(ParseError),
    /// The WebDriver server could not be reached.
    Failed(HError),
    /// The connection to the WebDriver server was lost.
    Lost(IOError),
    /// The server did not give a WebDriver-conforming response.
    NotW3C(serde_json::Value),
    /// The WebDriver server refused to create a new session.
    SessionNotCreated(WebDriver),
}

impl Error for NewSessionError {
    fn description(&self) -> &str {
        match *self {
            NewSessionError::BadWebdriverUrl(..) => "webdriver url is invalid",
            NewSessionError::Failed(..) => "webdriver server did not respond",
            NewSessionError::Lost(..) => "webdriver server disconnected",
            NewSessionError::NotW3C(..) => "webdriver server gave non-conformant response",
            NewSessionError::SessionNotCreated(..) => "webdriver did not create session",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            NewSessionError::BadWebdriverUrl(ref e) => Some(e),
            NewSessionError::Failed(ref e) => Some(e),
            NewSessionError::Lost(ref e) => Some(e),
            NewSessionError::NotW3C(..) => None,
            NewSessionError::SessionNotCreated(ref e) => Some(e),
        }
    }
}

impl fmt::Display for NewSessionError {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.description())?;
        match *self {
            NewSessionError::BadWebdriverUrl(ref e) => write!(f, "{}", e),
            NewSessionError::Failed(ref e) => write!(f, "{}", e),
            NewSessionError::Lost(ref e) => write!(f, "{}", e),
            NewSessionError::NotW3C(ref e) => write!(f, "{:?}", e),
            NewSessionError::SessionNotCreated(ref e) => write!(f, "{}", e),
        }
    }
}

/// An error occurred while executing some browser action.
#[derive(Debug)]
pub enum CmdError {
    /// A standard WebDriver error occurred.
    ///
    /// See [the spec] for details about what each of these errors represent. Note that for
    /// convenience `NoSuchElement` has been extracted into its own top-level variant.
    ///
    /// [the spec]: https://www.w3.org/TR/webdriver/#handling-errors
    Standard(WebDriver),

    /// No element was found matching the given locator.
    ///
    /// This variant lifts the ["no such element"] error variant from `Standard` to simplify
    /// checking for it in user code.
    ///
    /// It is also used for the ["stale element reference"] error variant.
    ///
    /// ["no such element"]: https://www.w3.org/TR/webdriver/#dfn-no-such-element
    /// ["stale element reference"]: https://www.w3.org/TR/webdriver/#dfn-stale-element-reference
    NoSuchElement(WebDriver),

    /// The requested window does not exist.
    ///
    /// This variant lifts the ["no such window"] error variant from `Standard` to simplify
    /// checking for it in user code.
    ///
    /// ["no such window"]: https://www.w3.org/TR/webdriver/#dfn-no-such-window
    NoSuchWindow(WebDriver),

    /// The requested alert does not exist.
    ///
    /// This variant lifts the ["no such alert"] error variant from `Standard` to simplify
    /// checking for it in user code.
    ///
    /// ["no such alert"]: https://www.w3.org/TR/webdriver/#dfn-no-such-alert
    NoSuchAlert(WebDriver),

    /// A bad URL was encountered during parsing.
    ///
    /// This normally happens if a link is clicked or the current URL is requested, but the URL in
    /// question is invalid or otherwise malformed.
    BadUrl(ParseError),

    /// A request to the WebDriver server failed.
    Failed(HError),

    /// The connection to the WebDriver server was lost.
    Lost(IOError),

    /// The WebDriver server responded with a non-standard, non-JSON reply.
    NotJson(String),

    /// The WebDriver server responded to a command with an invalid JSON response.
    Json(serde_json::Error),

    /// The WebDriver server produced a response that does not conform to the [W3C WebDriver
    /// specification][spec].
    ///
    /// Note: if you are trying to use `phantomjs` or `chromedriver`, note that these WebDriver
    /// implementations do *not* conform to the spec at this time. For example, `chromedriver`
    /// does not place `sessionId` for `NewSession` or errors under the `value` key in responses,
    /// and does not correctly encode and decode `WebElement` references.
    ///
    /// [spec]: https://www.w3.org/TR/webdriver/
    NotW3C(serde_json::Value),

    /// A function was invoked with an invalid argument.
    InvalidArgument(String, String),

    /// Could not decode a base64 image
    ImageDecodeError(::base64::DecodeError),

    /// Timeout of a wait condition.
    ///
    /// When waiting for a for a condition using [`Client::wait`](crate::Client::wait), any of the
    /// consuming methods, waiting on some condition, may return this error, indicating that the
    /// timeout waiting for the condition occurred.
    WaitTimeout,
}

impl CmdError {
    /// Returns true if this error indicates that a matching element was not found.
    ///
    /// Equivalent to
    /// ```no_run
    /// # use fantoccini::error::CmdError;
    /// # let e = CmdError::NotJson(String::new());
    /// let is_miss = if let CmdError::NoSuchElement(..) = e {
    ///   true
    /// } else {
    ///   false
    /// };
    /// ```
    pub fn is_miss(&self) -> bool {
        matches!(self, CmdError::NoSuchElement(..))
    }

    pub(crate) fn from_webdriver_error(e: webdriver::WebDriverError) -> Self {
        match e {
            webdriver::WebDriverError {
                error: webdriver::ErrorStatus::NoSuchElement,
                ..
            } => CmdError::NoSuchElement(WebDriver::from_upstream_error(e)),
            webdriver::WebDriverError {
                error: webdriver::ErrorStatus::NoSuchWindow,
                ..
            } => CmdError::NoSuchWindow(WebDriver::from_upstream_error(e)),
            webdriver::WebDriverError {
                error: webdriver::ErrorStatus::NoSuchAlert,
                ..
            } => CmdError::NoSuchAlert(WebDriver::from_upstream_error(e)),
            _ => CmdError::Standard(WebDriver::from_upstream_error(e)),
        }
    }
}

impl Error for CmdError {
    fn description(&self) -> &str {
        match *self {
            CmdError::Standard(..) => "webdriver returned error",
            CmdError::NoSuchElement(..) => "no element found matching selector",
            CmdError::NoSuchWindow(..) => "no window is currently selected",
            CmdError::NoSuchAlert(..) => "no alert is currently visible",
            CmdError::BadUrl(..) => "bad url provided",
            CmdError::Failed(..) => "webdriver could not be reached",
            CmdError::Lost(..) => "webdriver connection lost",
            CmdError::NotJson(..) => "webdriver returned invalid response",
            CmdError::Json(..) => "webdriver returned incoherent response",
            CmdError::NotW3C(..) => "webdriver returned non-conforming response",
            CmdError::InvalidArgument(..) => "invalid argument provided",
            CmdError::ImageDecodeError(..) => "error decoding image",
            CmdError::WaitTimeout => "timeout waiting on condition",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            CmdError::Standard(ref e)
            | CmdError::NoSuchElement(ref e)
            | CmdError::NoSuchWindow(ref e)
            | CmdError::NoSuchAlert(ref e) => Some(e),
            CmdError::BadUrl(ref e) => Some(e),
            CmdError::Failed(ref e) => Some(e),
            CmdError::Lost(ref e) => Some(e),
            CmdError::Json(ref e) => Some(e),
            CmdError::ImageDecodeError(ref e) => Some(e),
            CmdError::NotJson(_)
            | CmdError::NotW3C(_)
            | CmdError::InvalidArgument(..)
            | CmdError::WaitTimeout => None,
        }
    }
}

impl fmt::Display for CmdError {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.description())?;
        match *self {
            CmdError::Standard(ref e)
            | CmdError::NoSuchElement(ref e)
            | CmdError::NoSuchWindow(ref e)
            | CmdError::NoSuchAlert(ref e) => write!(f, "{}", e),
            CmdError::BadUrl(ref e) => write!(f, "{}", e),
            CmdError::Failed(ref e) => write!(f, "{}", e),
            CmdError::Lost(ref e) => write!(f, "{}", e),
            CmdError::NotJson(ref e) => write!(f, "{}", e),
            CmdError::Json(ref e) => write!(f, "{}", e),
            CmdError::NotW3C(ref e) => write!(f, "{:?}", e),
            CmdError::ImageDecodeError(ref e) => write!(f, "{:?}", e),
            CmdError::InvalidArgument(ref arg, ref msg) => {
                write!(f, "Invalid argument `{}`: {}", arg, msg)
            }
            CmdError::WaitTimeout => Ok(()),
        }
    }
}

impl From<IOError> for CmdError {
    fn from(e: IOError) -> Self {
        CmdError::Lost(e)
    }
}

impl From<ParseError> for CmdError {
    fn from(e: ParseError) -> Self {
        CmdError::BadUrl(e)
    }
}

impl From<HError> for CmdError {
    fn from(e: HError) -> Self {
        CmdError::Failed(e)
    }
}

impl From<serde_json::Error> for CmdError {
    fn from(e: serde_json::Error) -> Self {
        CmdError::Json(e)
    }
}

/// Error of attempting to create an invalid [`WindowHandle`] from a
/// [`"current"` string][1].
///
/// [`WindowHandle`]: crate::wd::WindowHandle
/// [1]: https://www.w3.org/TR/webdriver/#dfn-window-handles
#[derive(Clone, Copy, Debug)]
pub struct InvalidWindowHandle;

impl fmt::Display for InvalidWindowHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, r#"Window handle cannot be "current""#)
    }
}

impl Error for InvalidWindowHandle {}

impl From<InvalidWindowHandle> for CmdError {
    fn from(_: InvalidWindowHandle) -> Self {
        Self::NotW3C(serde_json::Value::String("current".to_string()))
    }
}

/// Error returned by WebDriver.
#[derive(Debug, Serialize)]
pub struct WebDriver {
    /// Code of this error provided by WebDriver.
    ///
    /// Intentionally made private, so library users cannot match on it.
    pub(crate) error: webdriver::ErrorStatus,

    /// Description of this error provided by WebDriver.
    pub message: Cow<'static, str>,

    /// Stacktrace of this error provided by WebDriver.
    pub stacktrace: Cow<'static, str>,
}

impl fmt::Display for WebDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for WebDriver {}

impl WebDriver {
    pub(crate) fn from_upstream_error(e: webdriver::WebDriverError) -> Self {
        Self {
            error: e.error,
            message: e.message,
            stacktrace: e.stack,
        }
    }

    /// Returns [code] of this error provided by WebDriver.
    ///
    /// [code]: https://www.w3.org/TR/webdriver/#dfn-error-code
    pub fn error(&self) -> &'static str {
        self.error.error_code()
    }

    /// Returns [HTTP Status] of this error provided by WebDriver.
    ///
    /// [HTTP Status]: https://www.w3.org/TR/webdriver/#dfn-error-code
    pub fn http_status(&self) -> http::StatusCode {
        self.error.http_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_display_error_doesnt_stackoverflow() {
        println!("{}", CmdError::NotJson("test".to_string()));
        println!("{}", NewSessionError::Lost(IOError::last_os_error()));
    }
}
