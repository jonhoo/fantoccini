use http::StatusCode;
use hyper::Error as HError;
use hyper_util::client::legacy::Error as HCError;
use serde::{Serialize, Serializer};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::Error as IOError;
use std::str::FromStr;
use url::ParseError;

/// An error occurred while attempting to establish a session for a new `Client`.
#[derive(Debug)]
pub enum NewSessionError {
    /// The given WebDriver URL is invalid.
    BadWebdriverUrl(ParseError),
    /// The WebDriver server could not be reached.
    Failed(HError),
    /// The WebDriver server could not be reached (error in hyper_util's legacy client).
    FailedC(HCError),
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
            NewSessionError::FailedC(..) => "webdriver server did not respond (legacy client)",
            NewSessionError::Lost(..) => "webdriver server disconnected",
            NewSessionError::NotW3C(..) => "webdriver server gave non-conformant response",
            NewSessionError::SessionNotCreated(..) => "webdriver did not create session",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            NewSessionError::BadWebdriverUrl(ref e) => Some(e),
            NewSessionError::Failed(ref e) => Some(e),
            NewSessionError::FailedC(ref e) => Some(e),
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
            NewSessionError::FailedC(ref e) => write!(f, "{}", e),
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
    /// See [the spec] for details about what each of these errors represent.
    ///
    /// [the spec]: https://www.w3.org/TR/webdriver/#handling-errors
    Standard(WebDriver),

    /// A bad URL was encountered during parsing.
    ///
    /// This normally happens if a link is clicked or the current URL is requested, but the URL in
    /// question is invalid or otherwise malformed.
    BadUrl(ParseError),

    /// A request to the WebDriver server failed.
    Failed(HError),

    /// A request to the WebDriver server failed (error in hyper_util's legacy client).
    FailedC(HCError),

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
    ImageDecodeError(base64::DecodeError),

    /// Timeout of a wait condition.
    ///
    /// When waiting for a for a condition using [`Client::wait`](crate::Client::wait), any of the
    /// consuming methods, waiting on some condition, may return this error, indicating that the
    /// timeout waiting for the condition occurred.
    WaitTimeout,
}

macro_rules! is_helper {
    ($($variant:ident => $name:ident$(,)?),*) => {
        $(
            /// Return true if this error matches
            #[doc = concat!("[`ErrorStatus::", stringify!($variant), "`].")]
            pub fn $name(&self) -> bool {
                matches!(self, CmdError::Standard(w) if w.error == ErrorStatus::$variant)
            }
        )*
    }
}

impl CmdError {
    is_helper! {
        DetachedShadowRoot => is_detached_shadow_root,
        ElementNotInteractable => is_element_not_interactable,
        ElementNotSelectable => is_element_not_selectable,
        InsecureCertificate => is_insecure_certificate,
        InvalidArgument => is_invalid_argument,
        InvalidCookieDomain => is_invalid_cookie_domain,
        InvalidCoordinates => is_invalid_coordinates,
        InvalidElementState => is_invalid_element_state,
        InvalidSelector => is_invalid_selector,
        InvalidSessionId => is_invalid_session_id,
        JavascriptError => is_javascript_error,
        MoveTargetOutOfBounds => is_move_target_out_of_bounds,
        NoSuchAlert => is_no_such_alert,
        NoSuchCookie => is_no_such_cookie,
        NoSuchElement => is_no_such_element,
        NoSuchFrame => is_no_such_frame,
        NoSuchShadowRoot => is_no_such_shadow_root,
        NoSuchWindow => is_no_such_window,
        ScriptTimeout => is_script_timeout,
        SessionNotCreated => is_session_not_created,
        StaleElementReference => is_stale_element_reference,
        Timeout => is_timeout,
        UnableToCaptureScreen => is_unable_to_capture_screen,
        UnableToSetCookie => is_unable_to_set_cookie,
        UnexpectedAlertOpen => is_unexpected_alert_open,
        UnknownCommand => is_unknown_command,
        UnknownError => is_unknown_error,
        UnknownMethod => is_unknown_method,
        UnknownPath => is_unknown_path,
        UnsupportedOperation => is_unsupported_operation
    }

    pub(crate) fn from_webdriver_error(e: WebDriver) -> Self {
        CmdError::Standard(e)
    }
}

impl Error for CmdError {
    fn description(&self) -> &str {
        match *self {
            CmdError::Standard(..) => "webdriver returned error",
            CmdError::BadUrl(..) => "bad url provided",
            CmdError::Failed(..) => "webdriver could not be reached",
            CmdError::FailedC(..) => "webdriver could not be reached (hyper_util)",
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
            CmdError::Standard(ref e) => Some(e),
            CmdError::BadUrl(ref e) => Some(e),
            CmdError::Failed(ref e) => Some(e),
            CmdError::FailedC(ref e) => Some(e),
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
            CmdError::Standard(ref e) => write!(f, "{}", e),
            CmdError::BadUrl(ref e) => write!(f, "{}", e),
            CmdError::Failed(ref e) => write!(f, "{}", e),
            CmdError::FailedC(ref e) => write!(f, "{}", e),
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

impl From<HCError> for CmdError {
    fn from(e: HCError) -> Self {
        CmdError::FailedC(e)
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

/// The error code returned from the WebDriver.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorStatus {
    /// The [element]'s [ShadowRoot] is not attached to the active document,
    /// or the reference is stale
    ///
    /// [element]: https://www.w3.org/TR/webdriver2/#dfn-elements
    /// [ShadowRoot]: https://www.w3.org/TR/webdriver2/#dfn-shadow-roots
    DetachedShadowRoot,

    /// The [`ElementClick`] command could not be completed because the
    /// [element] receiving the events is obscuring the element that was
    /// requested clicked.
    ///
    /// [`ElementClick`]: https://www.w3.org/TR/webdriver1/#dfn-element-click
    /// [element]: https://www.w3.org/TR/webdriver1/#dfn-elements
    ElementClickIntercepted,

    /// A [command] could not be completed because the element is not pointer-
    /// or keyboard interactable.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    ElementNotInteractable,

    /// An attempt was made to select an [element] that cannot be selected.
    ///
    /// [element]: https://www.w3.org/TR/webdriver1/#dfn-elements
    ElementNotSelectable,

    /// Navigation caused the user agent to hit a certificate warning, which is
    /// usually the result of an expired or invalid TLS certificate.
    InsecureCertificate,

    /// The arguments passed to a [command] are either invalid or malformed.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    InvalidArgument,

    /// An illegal attempt was made to set a cookie under a different domain
    /// than the current page.
    InvalidCookieDomain,

    /// The coordinates provided to an interactions operation are invalid.
    InvalidCoordinates,

    /// A [command] could not be completed because the element is an invalid
    /// state, e.g. attempting to click an element that is no longer attached
    /// to the document.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    InvalidElementState,

    /// Argument was an invalid selector.
    InvalidSelector,

    /// Occurs if the given session ID is not in the list of active sessions,
    /// meaning the session either does not exist or that it’s not active.
    InvalidSessionId,

    /// An error occurred while executing JavaScript supplied by the user.
    JavascriptError,

    /// The target for mouse interaction is not in the browser’s viewport and
    /// cannot be brought into that viewport.
    MoveTargetOutOfBounds,

    /// An attempt was made to operate on a modal dialogue when one was not
    /// open.
    NoSuchAlert,

    /// No cookie matching the given path name was found amongst the associated
    /// cookies of the current browsing context’s active document.
    NoSuchCookie,

    /// An [element] could not be located on the page using the given search
    /// parameters.
    ///
    /// [element]: https://www.w3.org/TR/webdriver1/#dfn-elements
    NoSuchElement,

    /// A [command] to switch to a frame could not be satisfied because the
    /// frame could not be found.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    NoSuchFrame,

    /// An [element]'s [ShadowRoot] was not found attached to the element.
    ///
    /// [element]: https://www.w3.org/TR/webdriver2/#dfn-elements
    /// [ShadowRoot]: https://www.w3.org/TR/webdriver2/#dfn-shadow-roots
    NoSuchShadowRoot,

    /// A [command] to switch to a window could not be satisfied because the
    /// window could not be found.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    NoSuchWindow,

    /// A script did not complete before its timeout expired.
    ScriptTimeout,

    /// A new session could not be created.
    SessionNotCreated,

    /// A [command] failed because the referenced [element] is no longer
    /// attached to the DOM.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    /// [element]: https://www.w3.org/TR/webdriver1/#dfn-elements
    StaleElementReference,

    /// An operation did not complete before its timeout expired.
    Timeout,

    /// A screen capture was made impossible.
    UnableToCaptureScreen,

    /// Setting the cookie’s value could not be done.
    UnableToSetCookie,

    /// A modal dialogue was open, blocking this operation.
    UnexpectedAlertOpen,

    /// The requested command could not be executed because it does not exist.
    UnknownCommand,

    /// An unknown error occurred in the remote end whilst processing the
    /// [command].
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    UnknownError,

    /// The requested [command] matched a known endpoint, but did not match a
    /// method for that endpoint.
    ///
    /// [command]: https://www.w3.org/TR/webdriver1/#dfn-commands
    UnknownMethod,

    /// Unknown WebDriver command.
    UnknownPath,

    /// Indicates that a command that should have executed properly is not
    /// currently supported.
    UnsupportedOperation,
}

impl ErrorStatus {
    /// Returns the correct HTTP status code associated with the error type.
    pub fn http_status(&self) -> StatusCode {
        use self::ErrorStatus::*;
        match *self {
            DetachedShadowRoot => StatusCode::NOT_FOUND,
            ElementClickIntercepted => StatusCode::BAD_REQUEST,
            ElementNotInteractable => StatusCode::BAD_REQUEST,
            ElementNotSelectable => StatusCode::BAD_REQUEST,
            InsecureCertificate => StatusCode::BAD_REQUEST,
            InvalidArgument => StatusCode::BAD_REQUEST,
            InvalidCookieDomain => StatusCode::BAD_REQUEST,
            InvalidCoordinates => StatusCode::BAD_REQUEST,
            InvalidElementState => StatusCode::BAD_REQUEST,
            InvalidSelector => StatusCode::BAD_REQUEST,
            InvalidSessionId => StatusCode::NOT_FOUND,
            JavascriptError => StatusCode::INTERNAL_SERVER_ERROR,
            MoveTargetOutOfBounds => StatusCode::INTERNAL_SERVER_ERROR,
            NoSuchAlert => StatusCode::NOT_FOUND,
            NoSuchCookie => StatusCode::NOT_FOUND,
            NoSuchElement => StatusCode::NOT_FOUND,
            NoSuchFrame => StatusCode::NOT_FOUND,
            NoSuchShadowRoot => StatusCode::NOT_FOUND,
            NoSuchWindow => StatusCode::NOT_FOUND,
            ScriptTimeout => StatusCode::INTERNAL_SERVER_ERROR,
            SessionNotCreated => StatusCode::INTERNAL_SERVER_ERROR,
            StaleElementReference => StatusCode::NOT_FOUND,
            Timeout => StatusCode::INTERNAL_SERVER_ERROR,
            UnableToCaptureScreen => StatusCode::BAD_REQUEST,
            UnableToSetCookie => StatusCode::INTERNAL_SERVER_ERROR,
            UnexpectedAlertOpen => StatusCode::INTERNAL_SERVER_ERROR,
            UnknownCommand => StatusCode::NOT_FOUND,
            UnknownError => StatusCode::INTERNAL_SERVER_ERROR,
            UnknownMethod => StatusCode::METHOD_NOT_ALLOWED,
            UnknownPath => StatusCode::NOT_FOUND,
            UnsupportedOperation => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for ErrorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for ErrorStatus {}

impl Serialize for ErrorStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.description().serialize(serializer)
    }
}

// This macro implements conversions between the error string literal and the
// corresponding ErrorStatus variant.
//
// In cases where multiple different string literals map to the same ErrorStatus
// variant, only the first string literal will be returned when converting from
// ErrorStatus to a static string.
macro_rules! define_error_strings {
    ($($variant:ident => $error_str:literal $(| $error_str_aliases:literal)*$(,)?),*) => {
        impl ErrorStatus {
            /// Get the error string associated with this `ErrorStatus`.
            pub fn description(&self) -> &'static str {
                use self::ErrorStatus::*;
                match self {
                    $(
                        $variant => $error_str,
                    )*
                }
            }
        }

        impl FromStr for ErrorStatus {
            type Err = CmdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                use self::ErrorStatus::*;
                let status: ErrorStatus = match s {
                    $(
                        $error_str$( | $error_str_aliases)* => $variant,
                    )*
                    _ => return Err(CmdError::NotW3C(serde_json::Value::String(s.to_string()))),
                };
                Ok(status)
            }
        }
    }
}

define_error_strings! {
    DetachedShadowRoot => "detached shadow root",
    ElementClickIntercepted => "element click intercepted",
    ElementNotInteractable => "element not interactable" | "element not visible",
    ElementNotSelectable => "element not selectable",
    InsecureCertificate => "insecure certificate",
    InvalidArgument => "invalid argument",
    InvalidCookieDomain => "invalid cookie domain",
    InvalidCoordinates => "invalid coordinates" | "invalid element coordinates",
    InvalidElementState => "invalid element state",
    InvalidSelector => "invalid selector",
    InvalidSessionId => "invalid session id",
    JavascriptError => "javascript error",
    MoveTargetOutOfBounds => "move target out of bounds",
    NoSuchAlert => "no such alert",
    NoSuchCookie => "no such cookie",
    NoSuchElement => "no such element",
    NoSuchFrame => "no such frame",
    NoSuchShadowRoot => "no such shadow root",
    NoSuchWindow => "no such window",
    ScriptTimeout => "script timeout",
    SessionNotCreated => "session not created",
    StaleElementReference => "stale element reference",
    Timeout => "timeout",
    UnableToCaptureScreen => "unable to capture screen",
    UnableToSetCookie => "unable to set cookie",
    UnexpectedAlertOpen => "unexpected alert open",
    UnknownCommand => "unknown command",
    UnknownError => "unknown error",
    UnknownMethod => "unknown method",
    UnknownPath => "unknown path",
    UnsupportedOperation => "unsupported operation",
}

impl TryFrom<&str> for ErrorStatus {
    type Error = CmdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<CmdError> for ErrorStatus {
    type Error = CmdError;

    fn try_from(value: CmdError) -> Result<Self, Self::Error> {
        match value {
            CmdError::Standard(w) => Ok(w.error),
            e => Err(e),
        }
    }
}

/// Error returned by WebDriver.
#[derive(Debug, Serialize)]
pub struct WebDriver {
    /// Code of this error provided by WebDriver.
    pub error: ErrorStatus,

    /// Description of this error provided by WebDriver.
    pub message: Cow<'static, str>,

    /// Stacktrace of this error provided by WebDriver.
    pub stacktrace: String,

    /// Optional [error data], populated by some commands.
    ///
    /// [error data]: https://www.w3.org/TR/webdriver1/#dfn-error-data
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for WebDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for WebDriver {}

impl WebDriver {
    /// Create a new WebDriver error struct.
    pub fn new(error: ErrorStatus, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            error,
            message: message.into(),
            stacktrace: String::new(),
            data: None,
        }
    }

    /// Include a stacktrace in the error details.
    pub fn with_stacktrace(mut self, stacktrace: String) -> Self {
        self.stacktrace = stacktrace;
        self
    }

    /// Include optional [error data].
    ///
    /// [error data]: https://www.w3.org/TR/webdriver1/#dfn-error-data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Returns [code] of this error provided by WebDriver.
    ///
    /// [code]: https://www.w3.org/TR/webdriver/#dfn-error-code
    pub fn error(&self) -> String {
        self.error.to_string()
    }

    /// Returns [HTTP Status] of this error provided by WebDriver.
    ///
    /// [HTTP Status]: https://www.w3.org/TR/webdriver/#dfn-error-code
    pub fn http_status(&self) -> StatusCode {
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
