//! Types used to represent particular elements on a page.

use crate::{error, Client, Locator};
use serde::Serialize;
use serde_json::Value as Json;
use webdriver::command::{SendKeysParameters, SwitchToFrameParameters, WebDriverCommand};
use webdriver::common::FrameId;
use webdriver::error::WebDriverError;

/// A single DOM element on the current page.
///
/// Note that there is a lot of subtlety in how you can interact with an element through WebDriver,
/// which [the WebDriver standard goes into detail on](https://www.w3.org/TR/webdriver1/#elements).
/// The same goes for inspecting [element state](https://www.w3.org/TR/webdriver1/#element-state).
#[derive(Clone, Debug, Serialize)]
pub struct Element {
    #[serde(skip_serializing)]
    pub(crate) client: Client,
    #[serde(flatten)]
    pub(crate) element: webdriver::common::WebElement,
}

/// An HTML form on the current page.
#[derive(Clone, Debug)]
pub struct Form {
    pub(crate) client: Client,
    pub(crate) form: webdriver::common::WebElement,
}

impl Element {
    /// Get back the [`Client`] hosting this `Element`.
    pub fn client(self) -> Client {
        self.client
    }
}

/// [Command Contexts](https://www.w3.org/TR/webdriver1/#command-contexts)
impl Element {
    /// Switches to the frame contained within the element.
    ///
    /// See [10.5 Switch To Frame](https://www.w3.org/TR/webdriver1/#switch-to-frame) of the
    /// WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Switch To Frame"))]
    pub async fn enter_frame(self) -> Result<Client, error::CmdError> {
        let Self {
            mut client,
            element,
        } = self;
        let params = SwitchToFrameParameters {
            id: Some(FrameId::Element(element)),
        };
        client
            .issue(WebDriverCommand::SwitchToFrame(params))
            .await?;
        Ok(client)
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
    pub async fn find(&mut self, search: Locator<'_>) -> Result<Element, error::CmdError> {
        let res = self
            .client
            .issue(WebDriverCommand::FindElementElement(
                self.element.clone(),
                search.into(),
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
    pub async fn find_all(&mut self, search: Locator<'_>) -> Result<Vec<Element>, error::CmdError> {
        let res = self
            .client
            .issue(WebDriverCommand::FindElementElements(
                self.element.clone(),
                search.into(),
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
    /// Look up an [attribute] value for this element by name.
    ///
    /// `Ok(None)` is returned if the element does not have the given attribute.
    ///
    /// See [13.2 Get Element Attribute](https://www.w3.org/TR/webdriver1/#get-element-attribute)
    /// of the WebDriver standard.
    ///
    /// [attribute]: https://dom.spec.whatwg.org/#concept-attribute
    #[cfg_attr(docsrs, doc(alias = "Get Element Attribute"))]
    pub async fn attr(&mut self, attribute: &str) -> Result<Option<String>, error::CmdError> {
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
    /// See [13.3 Get Element Property](https://www.w3.org/TR/webdriver1/#get-element-property)
    /// of the WebDriver standard.
    ///
    /// [property]: https://www.ecma-international.org/ecma-262/5.1/#sec-8.12.1
    #[cfg_attr(docsrs, doc(alias = "Get Element Property"))]
    pub async fn prop(&mut self, prop: &str) -> Result<Option<String>, error::CmdError> {
        let cmd = WebDriverCommand::GetElementProperty(self.element.clone(), prop.to_string());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(Some(v)),
            Json::Null => Ok(None),
            v => Err(error::CmdError::NotW3C(v)),
        }
    }

    /// Retrieve the text contents of this elment.
    ///
    /// See [13.5 Get Element Text](https://www.w3.org/TR/webdriver1/#get-element-text)
    /// of the WebDriver standard.
    #[cfg_attr(docsrs, doc(alias = "Get Element Text"))]
    pub async fn text(&mut self) -> Result<String, error::CmdError> {
        let cmd = WebDriverCommand::GetElementText(self.element.clone());
        match self.client.issue(cmd).await? {
            Json::String(v) => Ok(v),
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
    pub async fn html(&mut self, inner: bool) -> Result<String, error::CmdError> {
        let prop = if inner { "innerHTML" } else { "outerHTML" };
        Ok(self.prop(prop).await?.unwrap())
    }
}

/// [Element Interaction](https://www.w3.org/TR/webdriver1/#element-interaction)
impl Element {
    /// Simulate the user clicking on this element.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    ///
    /// See [14.1 Element Click](https://www.w3.org/TR/webdriver1/#element-click) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Element Click"))]
    pub async fn click(mut self) -> Result<Client, error::CmdError> {
        let cmd = WebDriverCommand::ElementClick(self.element);
        let r = self.client.issue(cmd).await?;
        if r.is_null() || r.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(self.client)
        } else {
            Err(error::CmdError::NotW3C(r))
        }
    }

    /// Clear this element.
    ///
    /// See [14.2 Element Clear](https://www.w3.org/TR/webdriver1/#element-clear) of the WebDriver
    /// standard.
    #[cfg_attr(docsrs, doc(alias = "Element Clear"))]
    pub async fn clear(&mut self) -> Result<(), error::CmdError> {
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
    pub async fn send_keys(&mut self, text: &str) -> Result<(), error::CmdError> {
        let cmd = WebDriverCommand::ElementSendKeys(
            self.element.clone(),
            SendKeysParameters {
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

/// Higher-level operations.
impl Element {
    /// Follow the `href` target of the element matching the given CSS selector *without* causing a
    /// click interaction.
    ///
    /// Note that since this *may* result in navigation, we give up the handle to the element.
    pub async fn follow(mut self) -> Result<Client, error::CmdError> {
        let cmd = WebDriverCommand::GetElementAttribute(self.element, "href".to_string());
        let href = self.client.issue(cmd).await?;
        let href = match href {
            Json::String(v) => v,
            Json::Null => {
                let e = WebDriverError::new(
                    webdriver::error::ErrorStatus::InvalidArgument,
                    "cannot follow element without href attribute",
                );
                return Err(error::CmdError::Standard(e));
            }
            v => return Err(error::CmdError::NotW3C(v)),
        };

        let url = self.client.current_url_().await?;
        let href = url.join(&href)?;
        self.client.goto(href.as_str()).await?;
        Ok(self.client)
    }

    /// Find and click an `<option>` child element by a locator.
    ///
    /// This method clicks the first `<option>` element that is found.
    /// If the element wasn't found, [`CmdError::NoSuchElement`](error::CmdError::NoSuchElement) will be issued.
    pub async fn select_by(mut self, locator: Locator<'_>) -> Result<Client, error::CmdError> {
        self.find(locator).await?.click().await
    }

    /// Find and click an `option` child element by its `value` attribute.
    pub async fn select_by_value(self, value: &str) -> Result<Client, error::CmdError> {
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
    pub async fn select_by_index(self, index: usize) -> Result<Client, error::CmdError> {
        self.select_by(Locator::Css(&format!("option:nth-of-type({})", index + 1)))
            .await
    }

    /// Find and click an `<option>` element by its visible text.
    ///
    /// The method doesn't make any escaping for the argument like it is done in python webdriver client for [example].
    /// It also doesn't make any normalizations before match.
    ///
    /// [example]: https://github.com/SeleniumHQ/selenium/blob/941dc9c6b2e2aa4f701c1b72be8de03d4b7e996a/py/selenium/webdriver/support/select.py#L67
    pub async fn select_by_label(self, label: &str) -> Result<Client, error::CmdError> {
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
    pub async fn set(
        &mut self,
        locator: Locator<'_>,
        value: &str,
    ) -> Result<Self, error::CmdError> {
        let locator = WebDriverCommand::FindElementElement(self.form.clone(), locator.into());
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
    pub async fn set_by_name(&mut self, field: &str, value: &str) -> Result<Self, error::CmdError> {
        let locator = format!("[name='{}']", field);
        let locator = Locator::Css(&locator);
        self.set(locator, value).await
    }
}

impl Form {
    /// Submit this form using the first available submit button.
    ///
    /// `false` is returned if no submit button was not found.
    pub async fn submit(self) -> Result<Client, error::CmdError> {
        self.submit_with(Locator::Css("input[type=submit],button[type=submit]"))
            .await
    }

    /// Submit this form using the button matched by the given selector.
    ///
    /// `false` is returned if a matching button was not found.
    pub async fn submit_with(mut self, button: Locator<'_>) -> Result<Client, error::CmdError> {
        let locator = WebDriverCommand::FindElementElement(self.form, button.into());
        let res = self.client.issue(locator).await?;
        let submit = self.client.parse_lookup(res)?;
        let res = self
            .client
            .issue(WebDriverCommand::ElementClick(submit))
            .await?;
        if res.is_null() || res.as_object().map(|o| o.is_empty()).unwrap_or(false) {
            // geckodriver returns {} :(
            Ok(self.client)
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }

    /// Submit this form using the form submit button with the given label (case-insensitive).
    ///
    /// `false` is returned if a matching button was not found.
    pub async fn submit_using(self, button_label: &str) -> Result<Client, error::CmdError> {
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
    pub async fn submit_direct(mut self) -> Result<Client, error::CmdError> {
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
            Ok(self.client)
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
    pub async fn submit_sneaky(
        mut self,
        field: &str,
        value: &str,
    ) -> Result<Client, error::CmdError> {
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
            Form {
                form: self.form,
                client: self.client,
            }
            .submit_direct()
            .await
        } else {
            Err(error::CmdError::NotW3C(res))
        }
    }
}
