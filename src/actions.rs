//! Actions functionality for WebDriver.
use crate::elements::Element;
use std::fmt::Debug;
use std::time::Duration;
use webdriver::actions as WDActions;

/// An action not associated with an input device (i.e. pause).
#[derive(Debug, Clone)]
pub enum NullAction {
    /// Pause for the specified duration.
    Pause {
        /// The pause duration, given in milliseconds.
        duration: u64,
    },
}

impl From<NullAction> for WDActions::NullActionItem {
    fn from(na: NullAction) -> Self {
        match na {
            NullAction::Pause { duration } => WDActions::NullActionItem::General(
                WDActions::GeneralAction::Pause(WDActions::PauseAction {
                    duration: Some(duration),
                }),
            ),
        }
    }
}

/// An action performed with a keyboard.
#[derive(Debug, Clone)]
pub enum KeyAction {
    /// Pause action.
    /// Useful for adding pauses between other key actions.
    Pause {
        /// The pause duration, given in milliseconds.
        duration: u64,
    },
    /// Key up action.
    Up {
        /// The key code, e.g. `'a'`. See the [`Keys`] module for special key codes.
        ///
        /// [`Keys`]: crate::keys::Keys
        value: char,
    },
    /// Key down action.
    Down {
        /// The key code, e.g. `'a'`. See the [`Keys`] module for special key codes.
        ///
        /// [`Keys`]: crate::keys::Keys
        value: char,
    },
}

impl From<KeyAction> for WDActions::KeyActionItem {
    fn from(ka: KeyAction) -> Self {
        use webdriver::actions::{KeyAction as WDKeyAction, KeyDownAction, KeyUpAction};
        match ka {
            KeyAction::Pause { duration } => WDActions::KeyActionItem::General(
                WDActions::GeneralAction::Pause(WDActions::PauseAction {
                    duration: Some(duration),
                }),
            ),
            KeyAction::Up { value } => {
                WDActions::KeyActionItem::Key(WDKeyAction::Up(KeyUpAction {
                    value: value.to_string(),
                }))
            }
            KeyAction::Down { value } => {
                WDActions::KeyActionItem::Key(WDKeyAction::Down(KeyDownAction {
                    value: value.to_string(),
                }))
            }
        }
    }
}

/// An action performed with a pointer device. See `PointerActionType` for
/// the availabledevice types.
#[derive(Debug, Clone)]
pub enum PointerAction {
    /// Pause action.
    /// Useful for adding pauses between other key actions.
    Pause {
        /// The pause duration, given in milliseconds.
        duration: u64,
    },
    /// Pointer button down.
    Down {
        /// The mouse button index. Button values are as follows:
        /// - Left = 0
        /// - Middle = 1
        /// - Right = 2
        button: u64,
    },
    /// Pointer button up.
    Up {
        /// The mouse button index. Button values are as follows:
        /// - Left = 0
        /// - Middle = 1
        /// - Right = 2
        button: u64,
    },
    /// Pointer move action. Duration is specified in milliseconds (can be 0).
    /// The x and y offsets are relative to the origin.
    Move {
        /// The move duration, given in milliseconds.
        duration: u64,
        /// The origin that the `x` and `y` coordinates are relative to.
        origin: PointerOrigin,
        /// `x` offset, in pixels.
        x: i64,
        /// `y` offset, in pixels.
        y: i64,
    },
    /// Pointer cancel action. Used to cancel the current pointer action.
    Cancel,
}

/// The pointer origin to use for the relative x,y offset.
#[derive(Debug, Clone)]
pub enum PointerOrigin {
    /// Coordinates are relative to the top-left corner of the browser window.
    Viewport,
    /// Coordinates are relative to the pointer's current position.
    Pointer,
    /// Coordinates are relative to the specified element's center position.
    WebElement(Element),
}

impl From<PointerOrigin> for WDActions::PointerOrigin {
    fn from(po: PointerOrigin) -> Self {
        match po {
            PointerOrigin::Viewport => WDActions::PointerOrigin::Viewport,
            PointerOrigin::Pointer => WDActions::PointerOrigin::Pointer,
            PointerOrigin::WebElement(e) => WDActions::PointerOrigin::Element(
                webdriver::common::WebElement(e.element_id().to_string()),
            ),
        }
    }
}

/// The type of pointer.
#[derive(Debug, Clone)]
pub enum PointerActionType {
    /// A mouse pointer device.
    Mouse,
    /// A pen device.
    Pen,
    /// A touch device.
    Touch,
}

impl From<PointerActionType> for WDActions::PointerType {
    fn from(pat: PointerActionType) -> Self {
        match pat {
            PointerActionType::Mouse => WDActions::PointerType::Mouse,
            PointerActionType::Pen => WDActions::PointerType::Pen,
            PointerActionType::Touch => WDActions::PointerType::Touch,
        }
    }
}

impl From<PointerAction> for WDActions::PointerActionItem {
    fn from(pa: PointerAction) -> Self {
        match pa {
            PointerAction::Pause { duration } => WDActions::PointerActionItem::General(
                WDActions::GeneralAction::Pause(WDActions::PauseAction {
                    duration: Some(duration),
                }),
            ),
            PointerAction::Down { button } => WDActions::PointerActionItem::Pointer(
                WDActions::PointerAction::Down(WDActions::PointerDownAction { button }),
            ),
            PointerAction::Up { button } => WDActions::PointerActionItem::Pointer(
                WDActions::PointerAction::Up(WDActions::PointerUpAction { button }),
            ),
            PointerAction::Move {
                duration,
                origin,
                x,
                y,
            } => WDActions::PointerActionItem::Pointer(WDActions::PointerAction::Move(
                WDActions::PointerMoveAction {
                    duration: Some(duration),
                    origin: origin.into(),
                    x: Some(x),
                    y: Some(y),
                },
            )),
            PointerAction::Cancel => {
                WDActions::PointerActionItem::Pointer(WDActions::PointerAction::Cancel)
            }
        }
    }
}

/// A channel containing `Null` actions.
#[derive(Debug, Clone)]
pub struct NullActionChannel {
    /// An identifier to distinguish this channel from others. Choose a meaningful string.
    /// May be useful for debugging.
    id: String,
    /// The list of actions for this channel.
    actions: Vec<NullAction>,
}

impl NullActionChannel {
    /// Create a new NullActionChannel.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            actions: Vec::new(),
        }
    }

    /// Add a pause action to this channel.
    pub fn add_pause(&mut self, duration: Duration) {
        self.add_action(NullAction::Pause {
            duration: duration.as_millis() as u64,
        });
    }

    /// Add the specified action to this channel.
    pub fn add_action(&mut self, action: NullAction) {
        self.actions.push(action);
    }
}

/// A channel containing `Key` actions.
#[derive(Debug, Clone)]
pub struct KeyActionChannel {
    /// An identifier to distinguish this channel from others. Choose a meaningful string.
    /// May be useful for debugging.
    id: String,
    /// The list of actions for this channel.
    actions: Vec<KeyAction>,
}

impl KeyActionChannel {
    /// Create a new Key channel.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            actions: Vec::new(),
        }
    }

    /// Add a pause action to this channel.
    pub fn add_pause(&mut self, duration: Duration) {
        self.add_action(KeyAction::Pause {
            duration: duration.as_millis() as u64,
        });
    }

    /// Add the specified action to this channel.
    pub fn add_action(&mut self, action: KeyAction) {
        self.actions.push(action);
    }
}

/// A channel containing `Key` actions.
#[derive(Debug, Clone)]
pub struct PointerActionChannel {
    /// An identifier to distinguish this channel from others. Choose a meaningful string.
    /// May be useful for debugging.
    id: String,
    /// The pointer type. Can be `Mouse`, `Pen` or `Touch`.
    pointer_type: PointerActionType,
    /// The list of actions for this channel.
    actions: Vec<PointerAction>,
}

impl PointerActionChannel {
    /// Create a new Pointer channel.
    pub fn new(id: &str, pointer_type: PointerActionType) -> Self {
        Self {
            id: id.to_string(),
            pointer_type,
            actions: Vec::new(),
        }
    }

    /// Add a pause action to this channel.
    pub fn add_pause(&mut self, duration: Duration) {
        self.add_action(PointerAction::Pause {
            duration: duration.as_millis() as u64,
        });
    }

    /// Add the specified action to this channel.
    pub fn add_action(&mut self, action: PointerAction) {
        self.actions.push(action);
    }
}

/// An ActionChannel is a sequence of actions of a specific type.
/// When combined with other channels, you can think of them like a grid, with actions on the
/// horizontal axis and channels on the vertical axis. All of the actions in the first column
/// will be executed simultaneously, then all of the second actions, and so on.
/// The second column will not be executed until all actions in the first column have been
/// completed, including any pauses.
///
/// Thus, the duration of each column will be equal to the longest duration of any individual
/// action in that column.
#[derive(Debug, Clone)]
pub enum ActionChannel {
    /// A channel containing null actions.
    Null(NullActionChannel),
    /// A channel containing key actions.
    Key(KeyActionChannel),
    /// A channel containing pointer actions. All actions in the channel are for a single
    /// pointer type.
    Pointer(PointerActionChannel),
}

impl ActionChannel {
    /// Add a pause action for this channel.
    pub fn add_pause(&mut self, duration: Duration) {
        match self {
            ActionChannel::Null(channel) => channel.add_pause(duration),
            ActionChannel::Key(channel) => channel.add_pause(duration),
            ActionChannel::Pointer(channel) => channel.add_pause(duration),
        }
    }

    fn into_sequence(self) -> WDActions::ActionSequence {
        match self {
            ActionChannel::Null(channel) => WDActions::ActionSequence {
                id: channel.id,
                actions: WDActions::ActionsType::Null {
                    actions: channel.actions.into_iter().map(|x| x.into()).collect(),
                },
            },
            ActionChannel::Key(channel) => WDActions::ActionSequence {
                id: channel.id,
                actions: WDActions::ActionsType::Key {
                    actions: channel.actions.into_iter().map(|x| x.into()).collect(),
                },
            },
            ActionChannel::Pointer(channel) => WDActions::ActionSequence {
                id: channel.id,
                actions: WDActions::ActionsType::Pointer {
                    parameters: WDActions::PointerActionParameters {
                        pointer_type: channel.pointer_type.into(),
                    },
                    actions: channel.actions.into_iter().map(|x| x.into()).collect(),
                },
            },
        }
    }
}

/// An Action Chain is simply a list of channels. See the documentation for `ActionChannel`
/// for more details.
///
/// Also see `ActionChainBuilder` for a convenient, high-level way to create an `ActionChain`.
#[derive(Debug, Clone, Default)]
pub struct ActionChain {
    channels: Vec<ActionChannel>,
}

impl ActionChain {
    /// Create a new ActionChain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an ActionChainBuilder.
    pub fn builder() -> ActionChainBuilder {
        ActionChainBuilder::default()
    }

    /// Add a complete channel to this ActionChain. This allows more flexibility if you want
    /// to create your own channels and then add them here.
    pub fn add_channel(&mut self, channel: ActionChannel) {
        self.channels.push(channel);
    }
}

impl From<ActionChain> for webdriver::command::ActionsParameters {
    fn from(ac: ActionChain) -> Self {
        let mut sequences = Vec::new();

        for channel in ac.channels {
            sequences.push(channel.into_sequence());
        }

        webdriver::command::ActionsParameters { actions: sequences }
    }
}

/// Builder for an ActionChain. Use `ActionChain::builder()` to create one.
#[derive(Debug)]
pub struct ActionChainBuilder {
    default_duration: Duration,
    key_channel: KeyActionChannel,
    mouse_channel: PointerActionChannel,
}

impl Default for ActionChainBuilder {
    fn default() -> Self {
        Self {
            default_duration: Duration::default(),
            key_channel: KeyActionChannel::new("key"),
            mouse_channel: PointerActionChannel::new("pointer", PointerActionType::Mouse),
        }
    }
}

impl ActionChainBuilder {
    /// Update the default pause interval. This will only affect actions added after this point.
    pub fn change_tick_interval(mut self, duration: Duration) -> Self {
        self.default_duration = duration;
        self
    }

    /// Construct an ActionChain from this ActionChainBuilder.
    pub fn build(self) -> ActionChain {
        let mut chain = ActionChain::new();
        if !self.key_channel.actions.is_empty() {
            chain.add_channel(ActionChannel::Key(self.key_channel));
        }

        if !self.mouse_channel.actions.is_empty() {
            chain.add_channel(ActionChannel::Pointer(self.mouse_channel));
        }

        chain
    }

    fn add_key_pause(&mut self) {
        self.key_channel.add_pause(self.default_duration);
    }

    fn add_mouse_pause(&mut self) {
        self.mouse_channel.add_pause(self.default_duration);
    }

    /// Add a pause for the specified duration.
    pub fn pause(mut self, duration: Duration) -> Self {
        self.key_channel.add_pause(duration);
        self.mouse_channel.add_pause(duration);
        self
    }

    /// Add a mouse button down action. Button values are as follows:
    /// - Left = 0
    /// - Middle = 1
    /// - Right = 2
    pub fn button_down(mut self, button: u64) -> Self {
        self.mouse_channel
            .add_action(PointerAction::Down { button });
        self.add_key_pause();
        self
    }

    /// Add a mouse button up action. Button values are as follows:
    /// - Left = 0
    /// - Middle = 1
    /// - Right = 2
    pub fn button_up(mut self, button: u64) -> Self {
        self.mouse_channel.add_action(PointerAction::Up { button });
        self.add_key_pause();
        self
    }

    /// Add a mouse move action. The `x_offset` and `y_offset` values are relative
    /// to the current mouse position.
    pub fn move_by(mut self, x_offset: i64, y_offset: i64) -> Self {
        self.mouse_channel.add_action(PointerAction::Move {
            duration: self.default_duration.as_millis() as u64,
            origin: PointerOrigin::Pointer,
            x: x_offset,
            y: y_offset,
        });
        self.add_key_pause();
        self
    }

    /// Add a mouse move action. The `x` and `y` values are relative to the
    /// top left corner of the browser window.
    pub fn move_to(mut self, x: i64, y: i64) -> Self {
        self.mouse_channel.add_action(PointerAction::Move {
            duration: self.default_duration.as_millis() as u64,
            origin: PointerOrigin::Viewport,
            x,
            y,
        });
        self.add_key_pause();
        self
    }

    /// Add an action to move the mouse to the specified element. The `x` and `y` values are
    /// relative to the element's center position.
    pub fn move_to_element_with_offset(mut self, element: Element, x: i64, y: i64) -> Self {
        self.mouse_channel.add_action(PointerAction::Move {
            duration: self.default_duration.as_millis() as u64,
            origin: PointerOrigin::WebElement(element),
            x,
            y,
        });
        self.add_key_pause();
        self
    }

    /// Add an action to move the mouse cursor to the center of the specified element.
    pub fn move_to_element(self, element: Element) -> Self {
        self.move_to_element_with_offset(element, 0, 0)
    }

    /// Add an action to click the specified mouse button. Button values are as follows:
    /// - Left = 0
    /// - Middle = 1
    /// - Right = 2
    pub fn click_button(self, button: u64) -> Self {
        self.button_down(button).button_up(button)
    }

    /// Add an action to double-click the specified mouse button. Button values are as follows:
    /// - Left = 0
    /// - Middle = 1
    /// - Right = 2
    pub fn double_click_button(self, button: u64) -> Self {
        self.click_button(button).click_button(button)
    }

    /// Add an action to click the left mouse button.
    pub fn click(self) -> Self {
        self.click_button(0)
    }

    /// Add an action to double-click the left mouse button.
    pub fn double_click(self) -> Self {
        self.double_click_button(0)
    }

    /// Add an action to click the left mouse button on the center point of the
    /// specified element.
    pub fn click_element(self, element: Element) -> Self {
        self.move_to_element(element).click()
    }

    /// Add an action to click the specified mouse button on the center point of the
    /// specified element.
    pub fn click_element_with_button(self, element: Element, button: u64) -> Self {
        self.move_to_element(element).click_button(button)
    }

    /// Add an action to double-click the left mouse button on the center point of the
    /// specified element.
    pub fn double_click_element(self, element: Element) -> Self {
        self.move_to_element(element).double_click()
    }

    /// Add an action to double-click the specified mouse button on the center point of the
    /// specified element.
    pub fn double_click_element_with_button(self, element: Element, button: u64) -> Self {
        self.move_to_element(element).double_click_button(button)
    }

    /// Add an action to click the specified mouse button on the center point of the
    /// source element, then drag to the center point of the target element, and then
    /// release the same mouse button.
    /// Button values are as follows:
    /// - Left = 0
    /// - Middle = 1
    /// - Right = 2
    pub fn drag_and_drop_element_with_button(
        self,
        source: Element,
        target: Element,
        button: u64,
    ) -> Self {
        self.move_to_element(source)
            .button_down(button)
            .move_to_element(target)
            .button_up(button)
    }

    /// Add an action to click the left mouse button on the center point of the source
    /// element, then drag to the center point of the target element, and then release
    /// the left mouse button.
    pub fn drag_and_drop_element(self, source: Element, target: Element) -> Self {
        self.drag_and_drop_element_with_button(source, target, 0)
    }

    /// Add a key down action for the specified character.
    /// See [`Keys`] for the codes for special keyboard buttons.
    ///
    /// [`Keys`]: crate::keys::Keys
    pub fn key_down(mut self, value: char) -> Self {
        self.key_channel.add_action(KeyAction::Down { value });
        self.add_mouse_pause();
        self
    }

    /// Add a key up action for the specified character.
    /// See [`Keys`] for the codes for special keyboard buttons.
    ///
    /// [`Keys`]: crate::keys::Keys
    pub fn key_up(mut self, value: char) -> Self {
        self.key_channel.add_action(KeyAction::Up { value });
        self.add_mouse_pause();
        self
    }

    /// Add a key down + key up action for each character in the specified string.
    /// See [`Keys`] for the codes for special keyboard buttons.
    ///
    /// [`Keys`]: crate::keys::Keys
    pub fn send_keys(mut self, text: &str) -> Self {
        for c in text.chars() {
            self = self.key_down(c).key_up(c)
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::Client;
    use serde_json::json;
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    fn compare_null_action(action: NullAction, value: serde_json::Value) {
        let channel = ActionChannel::Null(NullActionChannel {
            id: "null".to_string(),
            actions: vec![action],
        });

        let value_got = serde_json::to_value(channel.into_sequence());
        assert!(value_got.is_ok());
        assert_eq!(
            value_got.unwrap(),
            json!({
                "id": "null",
                "type": "none",
                "actions": [ value ]
            })
        );
    }

    #[test]
    fn test_null_action() {
        compare_null_action(
            NullAction::Pause { duration: 0 },
            json!({"type": "pause", "duration": 0}),
        );

        compare_null_action(
            NullAction::Pause { duration: 4 },
            json!({"type": "pause", "duration": 4}),
        );
    }

    fn compare_key_action(action: KeyAction, value: serde_json::Value) {
        let channel = ActionChannel::Key(KeyActionChannel {
            id: "key".to_string(),
            actions: vec![action],
        });

        let value_got = serde_json::to_value(channel.into_sequence());
        assert!(value_got.is_ok());
        assert_eq!(
            value_got.unwrap(),
            json!({
                "id": "key",
                "type": "key",
                "actions": [ value ]
            })
        );
    }

    #[test]
    fn test_key_action_pause() {
        compare_key_action(
            KeyAction::Pause { duration: 0 },
            json!({"type": "pause", "duration": 0}),
        );

        compare_key_action(
            KeyAction::Pause { duration: 3 },
            json!({"type": "pause", "duration": 3}),
        );
    }

    #[test]
    fn test_key_action_updown() {
        compare_key_action(
            KeyAction::Down { value: 'a' },
            json!({"type": "keyDown", "value": 'a'}),
        );

        compare_key_action(
            KeyAction::Down { value: '\u{e004}' },
            json!({
            "type": "keyDown", "value": '\u{e004}'
            }),
        );

        compare_key_action(
            KeyAction::Up { value: 'a' },
            json!({"type": "keyUp", "value": 'a'}),
        );

        compare_key_action(
            KeyAction::Up { value: '\u{e004}' },
            json!({
            "type": "keyUp", "value": '\u{e004}'
            }),
        );
    }

    fn compare_pointer_action(action: PointerAction, value: serde_json::Value) {
        let channel = ActionChannel::Pointer(PointerActionChannel {
            id: "mouse".to_string(),
            pointer_type: PointerActionType::Mouse,
            actions: vec![action],
        });

        let value_got = serde_json::to_value(channel.into_sequence());
        assert!(value_got.is_ok());
        assert_eq!(
            value_got.unwrap(),
            json!({
                "id": "mouse",
                "type": "pointer",
                "parameters": {
                    "pointerType": "mouse"
                },
                "actions": [ value ]
            })
        );
    }

    #[test]
    fn test_pointer_action_pause() {
        compare_pointer_action(
            PointerAction::Pause { duration: 0 },
            json!({"type": "pause", "duration": 0}),
        );

        compare_pointer_action(
            PointerAction::Pause { duration: 2 },
            json!({"type": "pause", "duration": 2}),
        );
    }

    #[test]
    fn test_pointer_action_button() {
        compare_pointer_action(
            PointerAction::Down { button: 0 },
            json!({"type": "pointerDown", "button": 0}),
        );

        compare_pointer_action(
            PointerAction::Down { button: 1 },
            json!({"type": "pointerDown", "button": 1}),
        );

        compare_pointer_action(
            PointerAction::Down { button: 2 },
            json!({"type": "pointerDown", "button": 2}),
        );

        compare_pointer_action(
            PointerAction::Up { button: 0 },
            json!({"type": "pointerUp", "button": 0}),
        );

        compare_pointer_action(
            PointerAction::Up { button: 1 },
            json!({"type": "pointerUp", "button": 1}),
        );

        compare_pointer_action(
            PointerAction::Up { button: 2 },
            json!({"type": "pointerUp", "button": 2}),
        );
    }

    #[test]
    fn test_pointer_action_pointermove() {
        compare_pointer_action(
            PointerAction::Move {
                duration: 0,
                x: 0,
                y: 0,
                origin: PointerOrigin::Viewport,
            },
            json!({
            "type": "pointerMove", "origin": "viewport", "x": 0, "y": 0, "duration": 0
            }),
        );

        compare_pointer_action(
            PointerAction::Move {
                duration: 0,
                x: 0,
                y: 0,
                origin: PointerOrigin::Pointer,
            },
            json!({
            "type": "pointerMove", "origin": "pointer", "x": 0, "y": 0, "duration": 0
            }),
        );

        let (tx, _) = unbounded_channel();
        let fake_client = Client {
            tx,
            is_legacy: false,
        };
        compare_pointer_action(
            PointerAction::Move {
                duration: 0,
                x: 0,
                y: 0,
                origin: PointerOrigin::WebElement(Element::from_element_id(
                    fake_client.clone(),
                    "id1234".to_string(),
                )),
            },
            json!({
            "type": "pointerMove", "origin": {"element-6066-11e4-a52e-4f735466cecf": "id1234"}, "x": 0, "y": 0, "duration": 0
            }),
        );

        compare_pointer_action(
            PointerAction::Move {
                duration: 1,
                x: 100,
                y: 200,
                origin: PointerOrigin::Viewport,
            },
            json!({
                "type": "pointerMove",
                "x": 100,
                "y": 200,
                "duration": 1,
                "origin": "viewport"
            }),
        );

        compare_pointer_action(
            PointerAction::Move {
                duration: 1,
                x: 100,
                y: 200,
                origin: PointerOrigin::Pointer,
            },
            json!({
                "type": "pointerMove",
                "x": 100,
                "y": 200,
                "duration": 1,
                "origin": "pointer"
            }),
        );

        compare_pointer_action(
            PointerAction::Move {
                duration: 1,
                x: 100,
                y: 200,
                origin: PointerOrigin::WebElement(Element::from_element_id(
                    fake_client,
                    "someid".to_string(),
                )),
            },
            json!({
                "type": "pointerMove",
                "x": 100,
                "y": 200,
                "duration": 1,
                "origin": {"element-6066-11e4-a52e-4f735466cecf": "someid"}
            }),
        );
    }

    #[test]
    fn test_pointer_action_cancel() {
        compare_pointer_action(PointerAction::Cancel, json!({"type": "pointerCancel"}));
    }
}
