//! Types used to represent particular elements on a page.

use crate::wd::Locator;
use crate::{error, Client};
use serde::Serialize;
use serde_json::Value as Json;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use webdriver::command::WebDriverCommand;
use webdriver::common::FrameId;

/// Web element reference.
///
/// > Each element has an associated web element reference that uniquely identifies the element
/// > across all browsing contexts. The web element reference for every element representing the
/// > same element must be the same. It must be a string, and should be the result of generating
/// > a UUID.
///
/// See [11. Elements](https://www.w3.org/TR/webdriver1/#elements) of the WebDriver standard.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ElementRef(String);

impl Display for ElementRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ElementRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for ElementRef {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ElementRef> for String {
    fn from(id: ElementRef) -> Self {
        id.0
    }
}

impl From<String> for ElementRef {
    fn from(s: String) -> Self {
        ElementRef(s)
    }
}

/// A single DOM element on the current page.
///
/// Note that there is a lot of subtlety in how you can interact with an element through WebDriver,
/// which [the WebDriver standard goes into detail on](https://www.w3.org/TR/webdriver1/#elements).
/// The same goes for inspecting [element state](https://www.w3.org/TR/webdriver1/#element-state).
#[derive(Clone, Debug, Serialize)]
pub struct Element {
    /// The high-level WebDriver client, for sending commands.
    #[serde(skip_serializing)]
    pub(crate) client: Client,
    /// The encapsulated WebElement struct.
    #[serde(flatten)]
    pub(crate) element: webdriver::common::WebElement,
}

impl Element {
    /// Construct an `Element` with the specified element id.
    /// The element id is the id given by the webdriver.
    pub fn from_element_id(client: Client, element_id: ElementRef) -> Self {
        Self {
            client,
            element: webdriver::common::WebElement(element_id.0),
        }
    }

    /// Get back the [`Client`] hosting this `Element`.
    pub fn client(self) -> Client {
        self.client
    }

    /// Get the element id as given by the webdriver.
    pub fn element_id(&self) -> ElementRef {
        ElementRef(self.element.0.clone())
    }
}

/// An HTML form on the current page.
#[derive(Clone, Debug)]
pub struct Form {
    pub(crate) client: Client,
    pub(crate) form: webdriver::common::WebElement,
}

/// [Command Contexts](https://www.w3.org/TR/webdriver1/#command-contexts)
impl Element {
    /// Switches to the frame contained within the element.
    ///
    /// See [10.5 Switch To Frame](https://www.w3.org/TR/webdriver1/#switch-to-frame) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Switch To Frame"))]
    pub async fn enter_frame(&self) -> Result<(), error::CmdError> {
        let params = webdriver::command::SwitchToFrameParameters {
            id: Some(FrameId::Element(self.element.clone())),
        };
        self.client
            .issue(WebDriverCommand::SwitchToFrame(params))
            .await?;
        Ok(())
    }
}

/// [Element Retrieval](https://www.w3.org/TR/webdriver1/#element-retrieval)
impl Element {
    /// Find the first descendant element that matches the given [`Locator`].
    ///
    /// See [12.4 Find Element From
    /// Element](https://www.w3.org/TR/webdriver1/#find-element-from-element) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Find Element From Element"))]
    pub async fn find(&self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        let res = self
            .client
            .issue(WebDriverCommand::FindElementElement(
                self.element.clone(),
                search.into_parameters(),
            ))
            .await?;
        let e = self.client.parse_lookup(res)?;
        Ok(Element {
            client: self.client.clone(),
            element: e,
        })
    }

    /// Find all descendant elements that match the given [`Locator`].
    ///
    /// See [12.5 Find Elemente From
    /// Element](https://www.w3.org/TR/webdriver1/#find-elements-from-element) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Find Elements From Element"))]
    pub async fn find_all(&self, search: Locator<'_>) -> Result<Vec<Element>, error::CmdError> {
        let res = self
            .client
            .issue(WebDriverCommand::FindElementElements(
                self.element.clone(),
                search.into_parameters(),
            ))
            .await?;
        let array = self.client.parse_lookup_all(res)?;
        Ok(array
            .into_iter()
            .map(move |e| Element {
                client: self.client.clone(),
                element: e,
            })
            .collect())
    }
}

/// [Element State](https://www.w3.org/TR/webdriver1/#element-state)
impl Element {
    /// Return true if the element is currently selected.
    ///
    /// See [13.1 Is Element Selected](https://www.w3.org/TR/webdriver1/#is-element-selected)
    /// of the WebDriver standard.
    pub async fn is_selected(&self) -> Result<bool, error::CmdError> {
        let cmd = WebDriverCommand::IsSelected(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::Bool(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Return true if the element is currently enabled.
    ///
    /// See [13.8 Is Element Enabled](https://www.w3.org/TR/webdriver1/#is-element-enabled)
    /// of the WebDriver standard.
    pub async fn is_enabled(&self) -> Result<bool, error::CmdError> {
        let cmd = WebDriverCommand::IsEnabled(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::Bool(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Return true if the element is currently displayed.
    ///
    /// See [Element Displayedness](https://www.w3.org/TR/webdriver1/#element-displayedness)
    /// of the WebDriver standard.
    pub async fn is_displayed(&self) -> Result<bool, error::CmdError> {
        let cmd = WebDriverCommand::IsDisplayed(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::Bool(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up an [attribute] value for this element by name.
    ///
    /// `Ok(None)` is returned if the element does not have the given attribute.
    ///
    /// See [13.2 Get Element Attribute](https://www.w3.org/TR/webdriver1/#get-element-attribute)
    /// of the WebDriver standard.
    ///
    /// [attribute]: https://dom.spec.whatwg.org/#concept-attribute
    #[cfg_attr(docsrs, doc(alias = "Get Element Attribute"))]
    pub async fn attr(&self, attribute: &str) -> Result<Option<String>, error::CmdError> {
        let cmd =
            WebDriverCommand::GetElementAttribute(self.element.clone(), attribute.to_string());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up a DOM [property] for this element by name.
    ///
    /// `Ok(None)` is returned if the element does not have the given property.
    ///
    /// Boolean properties such as "checked" will be returned as the String "true" or "false".
    ///
    /// See [13.3 Get Element Property](https://www.w3.org/TR/webdriver1/#get-element-property)
    /// of the WebDriver standard.
    ///
    /// [property]: https://www.ecma-international.org/ecma-262/5.1/#sec-8.12.1
    #[cfg_attr(docsrs, doc(alias = "Get Element Property"))]
    pub async fn prop(&self, prop: &str) -> Result<Option<String>, error::CmdError> {
        let cmd = WebDriverCommand::GetElementProperty(self.element.clone(), prop.to_string());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(Some(v)),
            Json::Bool(b) => Ok(Some(b.to_string())),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Look up the [computed value] of a CSS property for this element by name.
    ///
    /// `Ok(String::new())` is returned if the the given CSS property is not found.
    ///
    /// See [13.4 Get Element CSS Value](https://www.w3.org/TR/webdriver1/#get-element-css-value)
    /// of the WebDriver standard.
    ///
    /// [computed value]: https://drafts.csswg.org/css-cascade-4/#computed-value
    #[cfg_attr(docsrs, doc(alias = "Get Element CSS Value"))]
    pub async fn css_value(&self, prop: &str) -> Result<String, error::CmdError> {
        let cmd = WebDriverCommand::GetCSSValue(self.element.clone(), prop.to_string());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Retrieve the text contents of this element.
    ///
    /// See [13.5 Get Element Text](https://www.w3.org/TR/webdriver1/#get-element-text)
    /// of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Element Text"))]
    pub async fn text(&self) -> Result<String, error::CmdError> {
        let cmd = WebDriverCommand::GetElementText(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Retrieve the tag name of this element.
    ///
    /// See [13.6 Get Element Tag Name](https://www.w3.org/TR/webdriver1/#get-element-tag-name)
    /// of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Element Tag Name"))]
    pub async fn tag_name(&self) -> Result<String, error::CmdError> {
        let cmd = WebDriverCommand::GetElementTagName(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(v),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Gets the x, y, width, and height properties of the current element.
    ///
    /// See [13.7 Get Element Rect](https://www.w3.org/TR/webdriver1/#dfn-get-element-rect) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Element Rect"))]
    pub async fn rectangle(&self) -> Result<(f64, f64, f64, f64), error::CmdError> {
        match self
            .client
            .issue(WebDriverCommand::GetElementRect(self.element.clone()))
            .await?
        {
            Json::Object(mut obj) => {
                let x = match obj.remove("x").and_then(|x| x.as_f64()) {
                    Some(x) => x,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let y = match obj.remove("y").and_then(|y| y.as_f64()) {
                    Some(y) => y,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let width = match obj.remove("width").and_then(|width| width.as_f64()) {
                    Some(width) => width,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                let height = match obj.remove("height").and_then(|height| height.as_f64()) {
                    Some(height) => height,
                    None => return Err(error::CmdError::NotW3C(Json::Object(obj))),
                };

                Ok((x, y, width, height))
            }
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Retrieve the HTML contents of this element.
    ///
    /// `inner` dictates whether the wrapping node's HTML is excluded or not. For example, take the
    /// HTML:
    ///
    /// ```html
    /// <div id="foo"><hr /></div>
    /// ```
    ///
    /// With `inner = true`, `<hr />` would be returned. With `inner = false`,
    /// `<div id="foo"><hr /></div>` would be returned instead.
    #[cfg_attr(docsrs, doc(alias = "innerHTML"))]
    #[cfg_attr(docsrs, doc(alias = "outerHTML"))]
    pub async fn html(&self, inner: bool) -> Result<String, error::CmdError> {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        Ok(self.prop(prop).await?.unwrap())
    }
}

/// [Element Interaction](https://www.w3.org/TR/webdriver1/#element-interaction)
impl Element {
    /// Simulate the user clicking on this element.
    ///
    /// See [14.1 Element Click](https://www.w3.org/TR/webdriver1/#element-click) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Element Click"))]
    pub async fn click(&self) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::ElementClick(self.element.clone());
        let r = self.client.issue(cmd).await?;
        if r.is_null() || r.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }

    /// Clear this element.
    ///
    /// See [14.2 Element Clear](https://www.w3.org/TR/webdriver1/#element-clear) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Element Clear"))]
    pub async fn clear(&self) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::ElementClear(self.element.clone());
        let r = self.client.issue(cmd).await?;
        if r.is_null() {
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }

    /// Simulate the user sending keys to this element.
    ///
    /// This operation scrolls into view the form control element and then sends the provided keys
    /// to the element. In case the element is not keyboard-interactable, an element not
    /// interactable error is returned.
    ///
    /// See [14.3 Element Send Keys](https://www.w3.org/TR/webdriver1/#element-send-keys) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Element Send Keys"))]
    pub async fn send_keys(&self, text: &str) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::ElementSendKeys(
            self.element.clone(),
            webdriver::command::SendKeysParameters {
                text: text.to_owned(),
            },
        );
        let r = self.client.issue(cmd).await?;
        if r.is_null() {
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }
}

/// [Screen Capture](https://www.w3.org/TR/webdriver1/#screen-capture)
impl Element {
    /// Get a PNG-encoded screenshot of this element.
    ///
    /// See [19.2 Take Element Screenshot](https://www.w3.org/TR/webdriver1/#dfn-take-element-screenshot) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Take Element Screenshot"))]
    pub async fn screenshot(&self) -> Result<Vec<u8>, error::CmdError> {
        let src = self
            .client
            .issue(WebDriverCommand::TakeElementScreenshot(
                self.element.clone(),
            ))
            .await?;
        if let Some(src) = src.as_str() {
            base64::decode(src).map_err(error::CmdError::ImageDecodeError)
        } else {
            Err(error::CmdError::NotW3C(src))
        }
    }
}

/// Higher-level operations.
impl Element {
    /// Follow the `href` target of the element matching the given CSS selector *without* causing a
    /// click interaction.
    pub async fn follow(&self) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::GetElementAttribute(self.element.clone(), "href".to_string());
        let href = self.client.issue(cmd).await?;
        let href = match href {
            Json::String(v) => v,
            Json::Null => {
                let e = error::WebDriver::new(
                    error::ErrorStatus::InvalidArgument,
                    "cannot follow element without href attribute",
                );
                return Err(error::CmdError::Standard(e));
            }
            v => return Err(error::CmdError::NotW3C(v)),
        };

        let url = self.client.current_url_().await?;
        let href = url.join(&href)?;
        self.client.goto(href.as_str()).await?;
        Ok(())
    }

    /// Find and click an `<option>` child element by a locator.
    ///
    /// This method clicks the first `<option>` element that is found.
    pub async fn select_by(&self, locator: Locator<'_>) -> Result<(), error::CmdError> {
        self.find(locator).await?.click().await
    }

    /// Find and click an `option` child element by its `value` attribute.
    pub async fn select_by_value(&self, value: &str) -> Result<(), error::CmdError> {
        self.select_by(Locator::Css(&format!("option[value='{}']", value)))
            .await
    }

    /// Find and click an `<option>` child element by its index.
    ///
    /// This method clicks the first `<option>` element that is an `index`th child
    /// (`option:nth-of-type(index+1)`). This will be the `index`th `<option>`
    /// element if the current element is a `<select>`. If you use this method on
    /// an `Element` that is _not_ a `<select>` (such as on a full `<form>`), it
    /// may not do what you expect if there are multiple `<select>` elements
    /// in the form, or if it there are stray `<option>` in the form.
    ///
    /// The indexing in this method is 0-based.
    pub async fn select_by_index(&self, index: usize) -> Result<(), error::CmdError> {
        self.select_by(Locator::Css(&format!("option:nth-of-type({})", index + 1)))
            .await
    }

    /// Find and click an `<option>` element by its visible text.
    ///
    /// The method doesn't make any escaping for the argument like it is done in python webdriver client for [example].
    /// It also doesn't make any normalizations before match.
    ///
    /// [example]: https://github.com/SeleniumHQ/selenium/blob/941dc9c6b2e2aa4f701c1b72be8de03d4b7e996a/py/selenium/webdriver/support/select.py#L67
    pub async fn select_by_label(&self, label: &str) -> Result<(), error::CmdError> {
        self.select_by(Locator::XPath(&format!(r".//option[.='{}']", label)))
            .await
    }
}

impl Form {
    /// Get back the [`Client`] hosting this `Form`.
    pub fn client(self) -> Client {
        self.client
    }
}

impl Form {
    /// Find a form input using the given `locator` and set its value to `value`.
    pub async fn set(&self, locator: Locator<'_>, value: &str) -> Result<Self, error::CmdError> {
        let locator =
            WebDriverCommand::FindElementElement(self.form.clone(), locator.into_parameters());
        let value = Json::from(value);

        let res = self.client.issue(locator).await?;
        let field = self.client.parse_lookup(res)?;
        let mut args = vec![via_json!(&field), value];
        self.client.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "arguments[0].value = arguments[1]".to_string(),
            args: Some(args),
        };

        let res = self
            .client
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .await?;
        if res.is_null() {
            Ok(Form {
                client: self.client.clone(),
                form: self.form.clone(),
            })
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Find a form input with the given `name` and set its value to `value`.
    pub async fn set_by_name(&self, field: &str, value: &str) -> Result<Self, error::CmdError> {
        let locator = format!("[name='{}']", field);
        let locator = Locator::Css(&locator);
        self.set(locator, value).await
    }
}

impl Form {
    /// Submit this form using the first available submit button.
    ///
    /// `false` is returned if no submit button was not found.
    pub async fn submit(&self) -> Result<(), error::CmdError> {
        self.submit_with(Locator::Css("input[type=submit],button[type=submit]"))
            .await
    }

    /// Submit this form using the button matched by the given selector.
    ///
    /// `false` is returned if a matching button was not found.
    pub async fn submit_with(&self, button: Locator<'_>) -> Result<(), error::CmdError> {
        let locator =
            WebDriverCommand::FindElementElement(self.form.clone(), button.into_parameters());
        let res = self.client.issue(locator).await?;
        let submit = self.client.parse_lookup(res)?;
        let res = self
            .client
            .issue(WebDriverCommand::ElementClick(submit))
            .await?;
        if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form using the form submit button with the given label (case-insensitive).
    ///
    /// `false` is returned if a matching button was not found.
    pub async fn submit_using(&self, button_label: &str) -> Result<(), error::CmdError> {
        let escaped = button_label.replace('\\', "\\\\").replace('"', "\\\"");
        let btn = format!(
            "input[type=submit][value=\"{}\" i],\
             button[type=submit][value=\"{}\" i]",
            escaped, escaped
        );
        self.submit_with(Locator::Css(&btn)).await
    }

    /// Submit this form directly, without clicking any buttons.
    ///
    /// This can be useful to bypass forms that perform various magic when the submit button is
    /// clicked, or that hijack click events altogether (yes, I'm looking at you online
    /// advertisement code).
    ///
    /// Note that since no button is actually clicked, the `name=value` pair for the submit button
    /// will not be submitted. This can be circumvented by using `submit_sneaky` instead.
    pub async fn submit_direct(&self) -> Result<(), error::CmdError> {
        let mut args = vec![via_json!(&self.form)];
        self.client.fixup_elements(&mut args);
        // some sites are silly, and name their submit button "submit". this ends up overwriting
        // the "submit" function of the form with a reference to the submit button itself, so we
        // can't call .submit(). we get around this by creating a *new* form, and using *its*
        // submit() handler but with this pointed to the real form. solution from here:
        // https://stackoverflow.com/q/833032/472927#comment23038712_834197
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "document.createElement('form').submit.call(arguments[0])".to_string(),
            args: Some(args),
        };

        let res = self
            .client
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .await?;
        if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(())
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form directly, without clicking any buttons, and with an extra field.
    ///
    /// Like `submit_direct`, this method will submit this form without clicking a submit button.
    /// However, it will *also* inject a hidden input element on the page that carries the given
    /// `field=value` mapping. This allows you to emulate the form data as it would have been *if*
    /// the submit button was indeed clicked.
    pub async fn submit_sneaky(&self, field: &str, value: &str) -> Result<(), error::CmdError> {
        let mut args = vec![via_json!(&self.form), Json::from(field), Json::from(value)];
        self.client.fixup_elements(&mut args);
        let cmd = webdriver::command::JavascriptCommandParameters {
            script: "\
                     var h = document.createElement('input');\
                     h.setAttribute('type', 'hidden');\
                     h.setAttribute('name', arguments[1]);\
                     h.value = arguments[2];\
                     arguments[0].appendChild(h)"
                .to_string(),
            args: Some(args),
        };

        let res = self
            .client
            .issue(WebDriverCommand::ExecuteScript(cmd))
            .await?;
        if res.is_null() | res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            self.submit_direct().await
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }
}
