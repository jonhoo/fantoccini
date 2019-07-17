use hyper::error as herror;
use std::error::Error;
use std::fmt;
use std::io::Error as IOError;
use url::ParseError;
use webdriver::error as wderror;

/// An error occured while attempting to establish a session for a new `Client`.
#[derive(Debug)]
pub enum NewSessionError {
    /// The given WebDriver URL is invalid.
    BadWebdriverUrl(ParseError),
    /// The WebDriver server could not be reached.
    Failed(herror::Error),
    /// The connection to the WebDriver server was lost.
    Lost(IOError),
    /// The server did not give a WebDriver-conforming response.
    NotW3C(serde_json::Value),
    /// The WebDriver server refused to create a new session.
    SessionNotCreated(wderror::WebDriverError),
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    Standard(wderror::WebDriverError),

    /// No element was found matching the given locator.
    ///
    /// This variant lifts the ["no such element"] error variant from `Standard` to simplify
    /// checking for it in user code.
    ///
    /// ["no such element"]: https://www.w3.org/TR/webdriver/#dfn-no-such-element
    NoSuchElement(wderror::WebDriverError),

    /// A bad URL was encountered during parsing.
    ///
    /// This normally happens if a link is clicked or the current URL is requested, but the URL in
    /// question is invalid or otherwise malformed.
    BadUrl(ParseError),

    /// A request to the WebDriver server failed.
    Failed(herror::Error),

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
        if let CmdError::NoSuchElement(..) = *self {
            true
        } else {
            false
        }
    }
}

impl Error for CmdError {
    fn description(&self) -> &str {
        match *self {
            CmdError::Standard(..) => "webdriver returned error",
            CmdError::NoSuchElement(..) => "no element found matching selector",
            CmdError::BadUrl(..) => "bad url provided",
            CmdError::Failed(..) => "webdriver could not be reached",
            CmdError::Lost(..) => "webdriver connection lost",
            CmdError::NotJson(..) => "webdriver returned invalid response",
            CmdError::Json(..) => "webdriver returned incoherent response",
            CmdError::NotW3C(..) => "webdriver returned non-conforming response",
            CmdError::InvalidArgument(..) => "invalid argument provided",
            CmdError::ImageDecodeError(..) => "error decoding image",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            CmdError::Standard(ref e) | CmdError::NoSuchElement(ref e) => Some(e),
            CmdError::BadUrl(ref e) => Some(e),
            CmdError::Failed(ref e) => Some(e),
            CmdError::Lost(ref e) => Some(e),
            CmdError::Json(ref e) => Some(e),
            CmdError::ImageDecodeError(ref e) => Some(e),
            CmdError::NotJson(_) | CmdError::NotW3C(_) | CmdError::InvalidArgument(..) => None,
        }
    }
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: ", self.description())?;
        match *self {
            CmdError::Standard(ref e) | CmdError::NoSuchElement(ref e) => write!(f, "{}", e),
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

impl From<herror::Error> for CmdError {
    fn from(e: herror::Error) -> Self {
        CmdError::Failed(e)
    }
}

impl From<wderror::WebDriverError> for CmdError {
    fn from(e: wderror::WebDriverError) -> Self {
        if let wderror::WebDriverError {
            error: wderror::ErrorStatus::NoSuchElement,
            ..
        } = e
        {
            CmdError::NoSuchElement(e)
        } else {
            CmdError::Standard(e)
        }
    }
}

impl From<serde_json::Error> for CmdError {
    fn from(e: serde_json::Error) -> Self {
        CmdError::Json(e)
    }
}
