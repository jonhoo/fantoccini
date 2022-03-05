//! WebDriver client implementation.

use crate::actions::Actions;
use crate::elements::{Element, Form};
use crate::error;
use crate::session::{Cmd, Session, Task};
use crate::wait::Wait;
use crate::wd::{
    Capabilities, Locator, NewWindowType, TimeoutConfiguration, WebDriverStatus, WindowHandle,
};
use hyper::{client::connect, Method};
use serde_json::Value as Json;
use std::convert::{TryFrom, TryInto as _};
use std::future::Future;
use tokio::sync::{mpsc, oneshot};
use webdriver::command::{SendKeysParameters, WebDriverCommand};
use webdriver::common::{FrameId, ELEMENT_KEY};

// Used only under `native-tls`
#[cfg_attr(not(feature = "native-tls"), allow(unused_imports))]
use crate::ClientBuilder;

/// A WebDriver client tied to a single browser
/// [session](https://www.w3.org/TR/webdriver1/#sessions).
///
/// Use [`ClientBuilder`](crate::ClientBuilder) to create a new session.
///
/// Note that most callers should explicitly call `Client::close`, and wait for the returned
/// future before exiting. Not doing so may result in the WebDriver session not being cleanly
/// closed, which is particularly important for some drivers, such as geckodriver, where
/// multiple simulatenous sessions are not supported. If `close` is not explicitly called, a
/// session close request will be spawned on the given `handle` when the last instance of this
/// `Client` is dropped.
#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) tx: mpsc::UnboundedSender<Task>,
    pub(crate) is_legacy: bool,
}

impl Client {
    /// Connect to the WebDriver host running the given address.
    ///
    /// This connects using a platform-native TLS library, and is only available with the
    /// `native-tls` feature. To customize, use [`ClientBuilder`] instead.
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[deprecated(since = "0.17.1", note = "Prefer ClientBuilder::native")]
    pub async fn new(webdriver: &str) -> Result<Self, error::NewSessionError> {
        ClientBuilder::native().connect(webdriver).await
    }

    /// Connect to the WebDriver host running the given address.
    ///
    /// The provided `connector` is used to establish the connection to the WebDriver host, and
    /// should generally be one that supports HTTPS, as that is commonly required by WebDriver
    /// implementations.
    ///
    /// Calls `with_capabilities_and_connector` with an empty capabilities list.
    pub(crate) async fn new_with_connector<C>(
        webdriver: &str,
        connector: C,
    ) -> Result<Self, error::NewSessionError>
    where
        C: connect::Connect + Unpin + 'static + Clone + Send + Sync,
    {
        Self::with_capabilities_and_connector(
            webdriver,
            &webdriver::capabilities::Capabilities::new(),
            connector,
        )
        .await
    }

    /// Connect to the WebDriver host running the given address.
    ///
    /// Prefer using [`ClientBuilder`](crate::ClientBuilder) over calling this method directly.
    ///
    /// The given capabilities will be requested in `alwaysMatch` or `desiredCapabilities`
    /// depending on the protocol version supported by the server.
    ///
    /// Returns a future that resolves to a handle for issuing additional WebDriver tasks.
    pub async fn with_capabilities_and_connector<C>(
        webdriver: &str,
        cap: &Capabilities,
        connector: C,
    ) -> Result<Self, error::NewSessionError>
    where
        C: connect::Connect + Unpin + 'static + Clone + Send + Sync,
    {
        Session::with_capabilities_and_connector(webdriver, cap, connector).await
    }

    /// Get the unique session ID assigned by the WebDriver server to this client.
    pub async fn session_id(&self) -> Result<Option<String>, error::CmdError> {
        match self.issue(Cmd::GetSessionId).await? {
            Json::String(s) => Ok(Some(s)),
            Json::Null => Ok(None),
            v => unreachable!("response to GetSessionId was not a string: {:?}", v),
        }
    }

    /// Set the User Agent string to use for all subsequent requests.
    pub async fn set_ua<S: Into<String>>(&self, ua: S) -> Result<(), error::CmdError> {
        self.issue(Cmd::SetUa(ua.into())).await?;
        Ok(())
    }

    /// Get the current User Agent string.
    pub async fn get_ua(&self) -> Result<Option<String>, error::CmdError> {
        match self.issue(Cmd::GetUa).await? {
            Json::String(s) => Ok(Some(s)),
            Json::Null => Ok(None),
            v => unreachable!("response to GetSessionId was not a string: {:?}", v),
        }
    }

    /// Terminate the WebDriver session.
    ///
    /// Normally, a shutdown of the WebDriver connection will be initiated when the last clone of a
    /// `Client` is dropped. Specifically, the shutdown request will be issued using the tokio
    /// `Handle` given when creating this `Client`. This in turn means that any errors will be
    /// dropped.
    ///
    /// Once it has been called on one instance of a `Client`, all requests to other instances
    /// of that `Client` will fail.
    ///
    /// This function may be useful in conjunction with `raw_client_for`, as it allows you to close
    /// the automated browser window while doing e.g., a large download.
    pub async fn close(self) -> Result<(), error::CmdError> {
        self.issue(Cmd::Shutdown).await?;
        Ok(())
    }

    /// Mark this client's session as persistent.
    ///
    /// After all instances of a `Client` have been dropped, we normally shut down the WebDriver
    /// session, which also closes the associated browser window or tab. By calling this method,
    /// the shutdown command will _not_ be sent to this client's session, meaning its window or tab
    /// will remain open.
    ///
    /// Note that an explicit call to [`Client::close`] will still terminate the session.
    ///
    /// This function is safe to call multiple times.
    pub async fn persist(&self) -> Result<(), error::CmdError> {
        self.issue(Cmd::Persist).await?;
        Ok(())
    }
}

// NOTE: new impl block to keep related methods together.

/// [Sessions](https://www.w3.org/TR/webdriver1/#sessions)
impl Client {
    /// Get the WebDriver status.
    ///
    /// See [8.3 Status](https://www.w3.org/TR/webdriver1/#status) of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Status"))]
    pub async fn status(&self) -> Result<WebDriverStatus, error::CmdError> {
        let res = self.issue(WebDriverCommand::Status).await?;
        let status: WebDriverStatus = serde_json::from_value(res)?;
        Ok(status)
    }

    /// Get the timeouts for the current session.
    ///
    /// See [8.4 Get Timeouts](https://www.w3.org/TR/webdriver1/#get-timeouts) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Get Timeouts"))]
    pub async fn get_timeouts(&self) -> Result<TimeoutConfiguration, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetTimeouts).await?;
        let timeouts: TimeoutConfiguration = serde_json::from_value(res)?;
        Ok(timeouts)
    }

    /// Set the timeouts for the current session.
    ///
    /// See [8.5 Set Timeouts](https://www.w3.org/TR/webdriver1/#set-timeouts) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Set Timeouts"))]
    #[cfg_attr(docsrs, doc(alias = "Update Timeouts"))]
    pub async fn update_timeouts(
        &self,
        timeouts: TimeoutConfiguration,
    ) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::SetTimeouts(timeouts.into_params()))
            .await?;
        Ok(())
    }
}

/// [Navigation](https://www.w3.org/TR/webdriver1/#navigation)
impl Client {
    /// Navigate directly to the given URL.
    ///
    /// See [9.1 Navigate To](https://www.w3.org/TR/webdriver1/#dfn-navigate-to) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Navigate To"))]
    pub async fn goto(&self, url: &str) -> Result<(), error::CmdError> {
        let url = url.to_owned();
        let base = self.current_url_().await?;
        let url = base.join(&url)?;
        self.issue(WebDriverCommand::Get(webdriver::command::GetParameters {
            url: url.into(),
        }))
        .await?;
        Ok(())
    }

    /// Retrieve the currently active URL for this session.
    ///
    /// See [9.2 Get Current URL](https://www.w3.org/TR/webdriver1/#dfn-get-current-url) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Current URL"))]
    pub async fn current_url(&self) -> Result<url::Url, error::CmdError> {
        self.current_url_().await
    }

    pub(crate) async fn current_url_(&self) -> Result<url::Url, error::CmdError> {
        let url = self.issue(WebDriverCommand::GetCurrentUrl).await?;
        if let Some(url) = url.as_str() {
            let url = if url.is_empty() { "about:blank" } else { url };
            Ok(url.parse()?)
        } else {
            Err(error::CmdError::NotW3C(url))
        }
    }

    /// Go back to the previous page.
    ///
    /// See [9.3 Back](https://www.w3.org/TR/webdriver1/#dfn-back) of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Back"))]
    pub async fn back(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::GoBack).await?;
        Ok(())
    }

    /// Go forward to the next page.
    ///
    /// See [9.4 Forward](https://www.w3.org/TR/webdriver1/#dfn-forward) of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Forward"))]
    pub async fn forward(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::GoForward).await?;
        Ok(())
    }

    /// Refresh the current previous page.
    ///
    /// See [9.5 Refresh](https://www.w3.org/TR/webdriver1/#dfn-refresh) of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Refresh"))]
    pub async fn refresh(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::Refresh).await?;
        Ok(())
    }

    /// Get the current page title.
    ///
    /// See [9.6 Get Title](https://www.w3.org/TR/webdriver1/#dfn-get-title) of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Title"))]
    pub async fn title(&self) -> Result<String, error::CmdError> {
        let title = self.issue(WebDriverCommand::GetTitle).await?;
        if let Json::String(s) = title {
            Ok(s)
        } else {
            Err(error::CmdError::NotW3C(title))
        }
    }
}

/// [Command Contexts](https://www.w3.org/TR/webdriver1/#command-contexts)
impl Client {
    /// Gets the current window handle.
    ///
    /// See [10.1 Get Window Handle](https://www.w3.org/TR/webdriver1/#get-window-handle) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Window Handle"))]
    pub async fn window(&self) -> Result<WindowHandle, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetWindowHandle).await?;
        match res {
            Json::String(x) => Ok(x.try_into()?),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Closes the current window.
    ///
    /// Will close the session if no other windows exist.
    ///
    /// Closing a window will not switch the client to one of the remaining windows.
    /// The switching must be done by calling `switch_to_window` using a still live window
    /// after the current window has been closed.
    ///
    /// See [10.2 Close Window](https://www.w3.org/TR/webdriver1/#close-window) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Close Window"))]
    pub async fn close_window(&self) -> Result<(), error::CmdError> {
        let _res = self.issue(WebDriverCommand::CloseWindow).await?;
        Ok(())
    }

    /// Switches to the chosen window.
    ///
    /// See [10.3 Switch To Window](https://www.w3.org/TR/webdriver1/#switch-to-window) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Switch To Window"))]
    pub async fn switch_to_window(&self, window: WindowHandle) -> Result<(), error::CmdError> {
        let params = webdriver::command::SwitchToWindowParameters {
            handle: window.into(),
        };
        let _res = self.issue(WebDriverCommand::SwitchToWindow(params)).await?;
        Ok(())
    }

    /// Gets a list of all active windows (and tabs)
    ///
    /// See [10.4 Get Window Handles](https://www.w3.org/TR/webdriver1/#get-window-handles) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Window Handles"))]
    pub async fn windows(&self) -> Result<Vec<WindowHandle>, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetWindowHandles).await?;
        match res {
            Json::Array(handles) => handles
                .into_iter()
                .map(|handle| match handle {
                    Json::String(x) => Ok(x.try_into()?),
                    v => Err(error::CmdError::NotW3C(v)),
                })
                .collect::<Result<Vec<_>, _>>(),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Creates a new window. If `as_tab` is `true`, then a tab will be created instead.
    ///
    /// Windows are treated the same as tabs by the WebDriver protocol. The functions `new_window`,
    /// `switch_to_window`, `close_window`, `window` and `windows` all operate on both tabs and
    /// windows.
    ///
    /// This operation is only in the editor's draft of the next iteration of the WebDriver
    /// protocol, and may thus not be supported by all WebDriver implementations. For example, if
    /// you're using `geckodriver`, you will need `geckodriver > 0.24` and `firefox > 66` to use
    /// this feature.
    ///
    /// See [11.5 New Window](https://w3c.github.io/webdriver/#dfn-new-window) of the editor's
    /// draft standard.
    #[cfg_attr(docsrs, doc(alias = "New Window"))]
    pub async fn new_window(&self, as_tab: bool) -> Result<NewWindowResponse, error::CmdError> {
        let type_hint = if as_tab { "tab" } else { "window" }.to_string();
        let type_hint = Some(type_hint);
        let params = webdriver::command::NewWindowParameters { type_hint };
        match self.issue(WebDriverCommand::NewWindow(params)).await? {
            Json::Object(mut obj) => {
                let handle = match obj
                    .remove("handle")
                    .and_then(|x| x.as_str().map(WindowHandle::try_from))
                {
                    Some(Ok(handle)) => handle,
                    _ => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let typ = match obj.get("type").and_then(|x| x.as_str()) {
                    Some(typ) => match typ {
                        "tab" => NewWindowType::Tab,
                        "window" => NewWindowType::Window,
                        _ => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                    },
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                Ok(NewWindowResponse { handle, typ })
            }
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Switches to the frame specified at the index.
    ///
    /// See [10.5 Switch To Frame](https://www.w3.org/TR/webdriver1/#switch-to-frame) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Switch To Frame"))]
    pub async fn enter_frame(&self, index: Option<u16>) -> Result<(), error::CmdError> {
        let params = webdriver::command::SwitchToFrameParameters {
            id: index.map(FrameId::Short),
        };
        self.issue(WebDriverCommand::SwitchToFrame(params)).await?;
        Ok(())
    }

    /// Switches to the parent of the frame the client is currently contained within.
    ///
    /// See [10.6 Switch To Parent Frame](https://www.w3.org/TR/webdriver1/#switch-to-parent-frame)
    /// of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Switch To Parent Frame"))]
    pub async fn enter_parent_frame(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::SwitchToParentFrame).await?;
        Ok(())
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// See [10.7.2 Set Window Rect](https://www.w3.org/TR/webdriver1/#dfn-set-window-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Set Window Rect"))]
    pub async fn set_window_rect(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Some(x as i32),
            y: Some(y as i32),
            width: Some(width as i32),
            height: Some(height as i32),
        });

        self.issue(cmd).await?;
        Ok(())
    }

    /// Gets the x, y, width, and height properties of the current window.
    ///
    /// See [10.7.1 Get Window Rect](https://www.w3.org/TR/webdriver1/#dfn-get-window-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Window Rect"))]
    pub async fn get_window_rect(&self) -> Result<(u64, u64, u64, u64), error::CmdError> {
        match self.issue(WebDriverCommand::GetWindowRect).await? {
            Json::Object(mut obj) => {
                let x = match obj.remove("x").and_then(|x| x.as_u64()) {
                    Some(x) => x,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let y = match obj.remove("y").and_then(|y| y.as_u64()) {
                    Some(y) => y,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let width = match obj.remove("width").and_then(|width| width.as_u64()) {
                    Some(width) => width,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let height = match obj.remove("height").and_then(|height| height.as_u64()) {
                    Some(height) => height,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                Ok((x, y, width, height))
            }
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// See [10.7.2 Set Window Rect](https://www.w3.org/TR/webdriver1/#dfn-set-window-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Set Window Rect"))]
    pub async fn set_window_size(&self, width: u32, height: u32) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: None,
            y: None,
            width: Some(width as i32),
            height: Some(height as i32),
        });

        self.issue(cmd).await?;
        Ok(())
    }

    /// Gets the width and height of the current window.
    ///
    /// See [10.7.1 Get Window Rect](https://www.w3.org/TR/webdriver1/#dfn-get-window-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Window Rect"))]
    pub async fn get_window_size(&self) -> Result<(u64, u64), error::CmdError> {
        let (_, _, width, height) = self.get_window_rect().await?;
        Ok((width, height))
    }

    /// Sets the x, y, width, and height properties of the current window.
    ///
    /// See [10.7.2 Set Window Rect](https://www.w3.org/TR/webdriver1/#dfn-set-window-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Set Window Rect"))]
    pub async fn set_window_position(&self, x: u32, y: u32) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::SetWindowRect(webdriver::command::WindowRectParameters {
            x: Some(x as i32),
            y: Some(y as i32),
            width: None,
            height: None,
        });

        self.issue(cmd).await?;
        Ok(())
    }

    /// Gets the x and y top-left coordinate of the current window.
    ///
    /// See [10.7.1 Get Window Rect](https://www.w3.org/TR/webdriver1/#dfn-get-window-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Window Rect"))]
    pub async fn get_window_position(&self) -> Result<(u64, u64), error::CmdError> {
        let (x, y, _, _) = self.get_window_rect().await?;
        Ok((x, y))
    }

    /// Maximize the current window.
    ///
    /// See [10.7.3 Maximize Window](https://www.w3.org/TR/webdriver1/#dfn-maximize-window) of the
    /// WebDriver standard.
    pub async fn maximize_window(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::MaximizeWindow).await?;
        Ok(())
    }

    /// Minimize the current window.
    ///
    /// See [10.7.4 Minimize Window](https://www.w3.org/TR/webdriver1/#dfn-minimize-window) of the
    /// WebDriver standard.
    pub async fn minimize_window(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::MinimizeWindow).await?;
        Ok(())
    }

    /// Make the current window fullscreen.
    ///
    /// See [10.7.5 Fullscreen Window](https://www.w3.org/TR/webdriver1/#dfn-fullscreen-window) of the
    /// WebDriver standard.
    pub async fn fullscreen_window(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::FullscreenWindow).await?;
        Ok(())
    }
}

/// [Element Retrieval](https://www.w3.org/TR/webdriver1/#element-retrieval)
impl Client {
    /// Find an element on the page that matches the given [`Locator`].
    ///
    /// See [12.2 Find Element](https://www.w3.org/TR/webdriver1/#find-element) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Find Element"))]
    pub async fn find(&self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        self.by(search.into_parameters()).await
    }

    /// Find all elements on the page that match the given [`Locator`].
    ///
    /// See [12.3 Find Elements](https://www.w3.org/TR/webdriver1/#find-elements) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Find Elements"))]
    pub async fn find_all(&self, search: Locator<'_>) -> Result<Vec<Element>, error::CmdError> {
        let res = self
            .issue(WebDriverCommand::FindElements(search.into_parameters()))
            .await?;
        let array = self.parse_lookup_all(res)?;
        Ok(array
            .into_iter()
            .map(move |e| Element {
                client: self.clone(),
                element: e,
            })
            .collect())
    }

    /// Get the active element for this session.
    ///
    /// The "active" element is the `Element` within the DOM that currently has focus. This will
    /// often be an `<input>` or `<textarea>` element that currently has the text selection, or
    /// another input element such as a checkbox or radio button. Which elements are focusable
    /// depends on the platform and browser configuration.
    ///
    /// If no element has focus, the result may be the page body or a `NoSuchElement` error.
    ///
    /// See [12.6 Get Active Element](https://www.w3.org/TR/webdriver1/#dfn-get-active-element) of
    /// the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Active Element"))]
    pub async fn active_element(&self) -> Result<Element, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetActiveElement).await?;
        let e = self.parse_lookup(res)?;
        Ok(Element {
            client: self.clone(),
            element: e,
        })
    }

    /// Locate a form on the page.
    ///
    /// Through the returned `Form`, HTML forms can be filled out and submitted.
    pub async fn form(&self, search: Locator<'_>) -> Result<Form, error::CmdError> {
        let l = search.into_parameters();
        let res = self.issue(WebDriverCommand::FindElement(l)).await?;
        let f = self.parse_lookup(res)?;
        Ok(Form {
            client: self.clone(),
            form: f,
        })
    }
}

/// [Document Handling](https://www.w3.org/TR/webdriver1/#document-handling)
impl Client {
    /// Get the HTML source for the current page.
    ///
    /// See [15.1 Get Page Source](https://www.w3.org/TR/webdriver1/#dfn-get-page-source) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Page Source"))]
    pub async fn source(&self) -> Result<String, error::CmdError> {
        let src = self.issue(WebDriverCommand::GetPageSource).await?;
        if let Some(src) = src.as_str() {
            Ok(src.to_string())
        } else {
            Err(error::CmdError::NotW3C(src))
        }
    }

    /// Execute the given JavaScript `script` in the current browser session.
    ///
    /// `args` is available to the script inside the `arguments` array. Since `Element` implements
    /// `Serialize`, you can also provide serialized `Element`s as arguments, and they will
    /// correctly deserialize to DOM elements on the other side.
    ///
    /// To retrieve the value of a variable, `return` has to be used in the JavaScript code.
    ///
    /// See [15.2.1 Execute Script](https://www.w3.org/TR/webdriver1/#dfn-execute-script) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Execute Script"))]
    pub async fn execute(
        &self,
        script: &str,
        mut args: Vec<Json>,
    ) -> Result<Json, error::CmdError> {
        self.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: script.to_string(),
            args: Some(args),
        };

        self.issue(WebDriverCommand::ExecuteScript(cmd)).await
    }

    /// Execute the given async JavaScript `script` in the current browser session.
    ///
    /// The provided JavaScript has access to `args` through the JavaScript variable `arguments`.
    /// The `arguments` array also holds an additional element at the end that provides a completion callback
    /// for the asynchronous code.
    ///
    /// Since `Element` implements `Serialize`, you can also provide serialized `Element`s as arguments, and they will
    /// correctly deserialize to DOM elements on the other side.
    ///
    /// # Examples
    ///
    /// Call a web API from the browser and retrieve the value asynchronously
    ///
    /// ```ignore
    /// const JS: &'static str = r#"
    ///     const [date, callback] = arguments;
    ///
    ///     fetch(`http://weather.api/${date}/hourly`)
    ///     // whenever the HTTP Request completes,
    ///     // send the value back to the Rust context
    ///     .then(data => {
    ///         callback(data.json())
    ///     })
    /// "#;
    ///
    /// let weather = client.execute_async(JS, vec![date]).await?;
    /// ```
    ///
    /// See [15.2.2 Execute Async
    /// Script](https://www.w3.org/TR/webdriver1/#dfn-execute-async-script) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Execute Async Script"))]
    pub async fn execute_async(
        &self,
        script: &str,
        mut args: Vec<Json>,
    ) -> Result<Json, error::CmdError> {
        self.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: script.to_string(),
            args: Some(args),
        };

        self.issue(WebDriverCommand::ExecuteAsyncScript(cmd)).await
    }
}

/// [Actions](https://www.w3.org/TR/webdriver1/#actions)
impl Client {
    /// Create a new Actions chain.
    ///
    /// ```ignore
    /// let mouse_actions = MouseActions::new("mouse")
    ///     .then(PointerAction::Down {
    ///         button: MOUSE_BUTTON_LEFT,
    ///     })
    ///     .then(PointerAction::MoveBy {
    ///         duration: Some(Duration::from_secs(2)),
    ///         x: 100,
    ///         y: 0,
    ///     })
    ///     .then(PointerAction::Up {
    ///         button: MOUSE_BUTTON_LEFT,
    ///     });
    /// client.perform_actions(mouse_actions).await?;
    /// ```
    ///
    /// See the documentation for [`Actions`] for more information.
    /// Perform the specified input actions.
    ///
    /// See [17.5 Perform Actions](https://www.w3.org/TR/webdriver1/#perform-actions) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Perform Actions"))]
    pub async fn perform_actions(
        &self,
        actions: impl Into<Actions>,
    ) -> Result<(), error::CmdError> {
        let params = webdriver::command::ActionsParameters {
            actions: actions.into().sequences.into_iter().map(|x| x.0).collect(),
        };

        self.issue(WebDriverCommand::PerformActions(params)).await?;
        Ok(())
    }

    /// Release all input actions.
    ///
    /// See [17.6 Release Actions](https://www.w3.org/TR/webdriver1/#release-actions) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Release Actions"))]
    pub async fn release_actions(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::ReleaseActions).await?;
        Ok(())
    }
}

/// [User Prompts](https://www.w3.org/TR/webdriver1/#user-prompts)
impl Client {
    /// Dismiss the active alert, if there is one.
    ///
    /// See [18.1 Dismiss Alert](https://www.w3.org/TR/webdriver1/#dismiss-alert) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Dismiss Alert"))]
    pub async fn dismiss_alert(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::DismissAlert).await?;
        Ok(())
    }

    /// Accept the active alert, if there is one.
    ///
    /// See [18.2 Accept Alert](https://www.w3.org/TR/webdriver1/#accept-alert) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Accept Alert"))]
    pub async fn accept_alert(&self) -> Result<(), error::CmdError> {
        self.issue(WebDriverCommand::AcceptAlert).await?;
        Ok(())
    }

    /// Get the text of the active alert, if there is one.
    ///
    /// See [18.3 Get Alert Text](https://www.w3.org/TR/webdriver1/#get-alert-text) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Alert Text"))]
    pub async fn get_alert_text(&self) -> Result<String, error::CmdError> {
        let res = self.issue(WebDriverCommand::GetAlertText).await?;
        if let Json::String(s) = res {
            Ok(s)
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Send the specified text to the active alert, if there is one.
    ///
    /// See [18.4 Send Alert Text](https://www.w3.org/TR/webdriver1/#send-alert-text) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Send Alert Text"))]
    pub async fn send_alert_text(&self, text: &str) -> Result<(), error::CmdError> {
        let params = SendKeysParameters {
            text: text.to_string(),
        };
        self.issue(WebDriverCommand::SendAlertText(params)).await?;
        Ok(())
    }
}

/// [Screen Capture](https://www.w3.org/TR/webdriver1/#screen-capture)
impl Client {
    /// Get a PNG-encoded screenshot of the current page.
    ///
    /// See [19.1 Take Screenshot](https://www.w3.org/TR/webdriver1/#dfn-take-screenshot) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Take Screenshot"))]
    pub async fn screenshot(&self) -> Result<Vec<u8>, error::CmdError> {
        let src = self.issue(WebDriverCommand::TakeScreenshot).await?;
        if let Some(src) = src.as_str() {
            base64::decode(src).map_err(error::CmdError::ImageDecodeError)
        } else {
            Err(error::CmdError::NotW3C(src))
        }
    }
}

/// Operations that wait for a change on the page.
impl Client {
    /// Wait for the given function to return `true` before proceeding.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    #[deprecated(
        since = "0.17.5",
        note = "This method might block forever. Please use client.wait().on(...) instead. You can still wait forever using: client.wait().forever().on(...)"
    )]
    pub async fn wait_for<F, FF>(&self, mut is_ready: F) -> Result<(), error::CmdError>
    where
        F: FnMut(&Client) -> FF,
        FF: Future<Output = Result<bool, error::CmdError>>,
    {
        while !is_ready(self).await? {}
        Ok(())
    }

    /// Wait for the given element to be present on the page.
    ///
    /// This can be useful to wait for something to appear on the page before interacting with it.
    /// While this currently just spins and yields, it may be more efficient than this in the
    /// future. In particular, in time, it may only run `is_ready` again when an event occurs on
    /// the page.
    #[deprecated(
        since = "0.17.5",
        note = "This method might block forever. Please use client.wait().on(locator) instead. You can still wait forever using: client.wait().forever().on(locator)"
    )]
    pub async fn wait_for_find(&self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        self.wait().forever().for_element(search).await
    }

    /// Wait for the page to navigate to a new URL before proceeding.
    ///
    /// If the `current` URL is not provided, `self.current_url()` will be used. Note however that
    /// this introduces a race condition: the browser could finish navigating *before* we call
    /// `current_url()`, which would lead to an eternal wait.
    #[deprecated(
        since = "0.17.5",
        note = "This method might block forever and has a chance of randomly blocking. Please use client.wait().on(url) instead, to check if you navigated successfully to the new URL."
    )]
    pub async fn wait_for_navigation(
        &self,
        current: Option<url::Url>,
    ) -> Result<(), error::CmdError> {
        let current = match current {
            Some(current) => current,
            None => self.current_url_().await?,
        };

        #[allow(deprecated)]
        self.wait_for(move |c| {
            // TODO: get rid of this clone
            let current = current.clone();
            // TODO: and this one too
            let c = c.clone();
            async move { Ok(c.current_url().await? != current) }
        })
        .await
    }
}

/// Raw access to the WebDriver instance.
impl Client {
    /// Issue an HTTP request to the given `url` with all the same cookies as the current session.
    ///
    /// Calling this method is equivalent to calling `with_raw_client_for` with an empty closure.
    pub async fn raw_client_for(
        &self,
        method: Method,
        url: &str,
    ) -> Result<hyper::Response<hyper::Body>, error::CmdError> {
        self.with_raw_client_for(method, url, |req| req.body(hyper::Body::empty()).unwrap())
            .await
    }

    /// Build and issue an HTTP request to the given `url` with all the same cookies as the current
    /// session.
    ///
    /// Before the HTTP request is issued, the given `before` closure will be called with a handle
    /// to the `Request` about to be sent.
    pub async fn with_raw_client_for<F>(
        &self,
        method: Method,
        url: &str,
        before: F,
    ) -> Result<hyper::Response<hyper::Body>, error::CmdError>
    where
        F: FnOnce(http::request::Builder) -> hyper::Request<hyper::Body>,
    {
        let url = url.to_owned();
        // We need to do some trickiness here. GetCookies will only give us the cookies for the
        // *current* domain, whereas we want the cookies for `url`'s domain. So, we navigate to the
        // URL in question, fetch its cookies, and then navigate back. *Except* that we can't do
        // that either (what if `url` is some huge file?). So we *actually* navigate to some weird
        // url that's unlikely to exist on the target doamin, and which won't resolve into the
        // actual content, but will still give the same cookies.
        //
        // The fact that cookies can have /path and security constraints makes this even more of a
        // pain. /path in particular is tricky, because you could have a URL like:
        //
        //    example.com/download/some_identifier/ignored_filename_just_for_show
        //
        // Imagine if a cookie is set with path=/download/some_identifier. How do we get that
        // cookie without triggering a request for the (large) file? I don't know. Hence: TODO.
        let old_url = self.current_url_().await?;
        let url = old_url.clone().join(&url)?;
        let cookie_url = url.clone().join("/please_give_me_your_cookies")?;
        self.goto(cookie_url.as_str()).await?;

        // TODO: go back before we return if this call errors:
        let cookies = self.issue(WebDriverCommand::GetCookies).await?;
        if !cookies.is_array() {
            return Err(error::CmdError::NotW3C(cookies));
        }
        self.back().await?;
        let ua = self.get_ua().await?;

        // now add all the cookies
        let mut all_ok = true;
        let mut jar = Vec::new();
        for cookie in cookies.as_array().unwrap() {
            if !cookie.is_object() {
                all_ok = false;
                break;
            }

            // https://w3c.github.io/webdriver/webdriver-spec.html#cookies
            let cookie = cookie.as_object().unwrap();
            if !cookie.contains_key("name") || !cookie.contains_key("value") {
                all_ok = false;
                break;
            }

            if !cookie["name"].is_string() || !cookie["value"].is_string() {
                all_ok = false;
                break;
            }

            // Note that since we're sending these cookies, all that matters is the mapping
            // from name to value. The other fields only matter when deciding whether to
            // include a cookie or not, and the driver has already decided that for us
            // (GetCookies is for a particular URL).
            jar.push(
                cookie::Cookie::new(
                    cookie["name"].as_str().unwrap().to_owned(),
                    cookie["value"].as_str().unwrap().to_owned(),
                )
                .encoded()
                .to_string(),
            );
        }

        if !all_ok {
            return Err(error::CmdError::NotW3C(cookies));
        }

        let mut req = hyper::Request::builder();
        req = req
            .method(method)
            .uri(http::Uri::try_from(url.as_str()).unwrap());
        req = req.header(hyper::header::COOKIE, jar.join("; "));
        if let Some(s) = ua {
            req = req.header(hyper::header::USER_AGENT, s);
        }
        let req = before(req);
        let (tx, rx) = oneshot::channel();
        self.issue(Cmd::Raw { req, rsp: tx }).await?;
        match rx.await {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Err(e.into()),
            Err(e) => unreachable!("Session ended prematurely: {:?}", e),
        }
    }
}

/// Allow to wait for conditions.
impl Client {
    /// Starting building a new wait operation. This can be used to wait for a certain condition, by
    /// periodically checking the state and optionally returning a value:
    ///
    /// ```no_run
    /// # use fantoccini::{ClientBuilder, Locator};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fantoccini::error::CmdError> {
    /// # #[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
    /// # let client = ClientBuilder::native().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
    /// # #[cfg(feature = "rustls-tls")]
    /// # let client = ClientBuilder::rustls().connect("http://localhost:4444").await.expect("failed to connect to WebDriver");
    /// # #[cfg(all(not(feature = "native-tls"), not(feature = "rustls-tls")))]
    /// # let client: fantoccini::Client = unreachable!("no tls provider available");
    /// // -- snip wrapper code --
    /// let button = client.wait().for_element(Locator::Css(
    ///     r#"a.button-download[href="/learn/get-started"]"#,
    /// )).await?;
    /// // -- snip wrapper code --
    /// # client.close().await
    /// # }
    /// ```
    ///
    /// Also see: [`crate::wait`].
    pub fn wait(&self) -> Wait<'_> {
        Wait::new(self)
    }
}

/// Helper methods
impl Client {
    pub(crate) async fn by(
        &self,
        locator: webdriver::command::LocatorParameters,
    ) -> Result<Element, error::CmdError> {
        let res = self.issue(WebDriverCommand::FindElement(locator)).await?;
        let e = self.parse_lookup(res)?;
        Ok(Element {
            client: self.clone(),
            element: e,
        })
    }

    /// Extract the `WebElement` from a `FindElement` or `FindElementElement` command.
    pub(crate) fn parse_lookup(
        &self,
        res: Json,
    ) -> Result<webdriver::common::WebElement, error::CmdError> {
        let mut res = match res {
            Json::Object(o) => o,
            res => return Err(error::CmdError::NotW3C(res)),
        };

        // legacy protocol uses "ELEMENT" as identifier
        let key = if self.is_legacy() {
            "ELEMENT"
        } else {
            ELEMENT_KEY
        };

        if !res.contains_key(key) {
            return Err(error::CmdError::NotW3C(Json::Object(res)));
        }

        match res.remove(key) {
            Some(Json::String(wei)) => {
                return Ok(webdriver::common::WebElement(wei));
            }
            Some(v) => {
                res.insert(key.to_string(), v);
            }
            None => {}
        }

        Err(error::CmdError::NotW3C(Json::Object(res)))
    }

    /// Extract `WebElement`s from a `FindElements` or `FindElementElements` command.
    pub(crate) fn parse_lookup_all(
        &self,
        res: Json,
    ) -> Result<Vec<webdriver::common::WebElement>, error::CmdError> {
        let res = match res {
            Json::Array(a) => a,
            res => return Err(error::CmdError::NotW3C(res)),
        };

        let mut array = Vec::new();
        for json in res {
            let e = self.parse_lookup(json)?;
            array.push(e);
        }

        Ok(array)
    }

    pub(crate) fn fixup_elements(&self, args: &mut [Json]) {
        if self.is_legacy() {
            for arg in args {
                // the serialization of WebElement uses the W3C index,
                // but legacy implementations need us to use the "ELEMENT" index
                if let Json::Object(ref mut o) = *arg {
                    if let Some(wei) = o.remove(ELEMENT_KEY) {
                        o.insert("ELEMENT".to_string(), wei);
                    }
                }
            }
        }
    }
}

/// Response returned by [`Client::new_window()`] method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewWindowResponse {
    /// Handle to the created browser window.
    pub handle: WindowHandle,

    /// Type of the created browser window.
    pub typ: NewWindowType,
}
