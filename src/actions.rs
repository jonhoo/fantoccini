//! Actions functionality for WebDriver.
#[cfg(doc)]
use crate::client::Client;
use crate::elements::Element;
#[cfg(doc)]
use crate::key::Key;
use std::fmt::Debug;
use std::time::Duration;
use webdriver::actions as WDActions;

/// An action not associated with an input device (e.g. pause).
///
/// See [17.4.1 General Actions](https://www.w3.org/TR/webdriver1/#general-actions) of the
/// WebDriver standard.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NullAction {
    /// Pause for the specified duration.
    Pause {
        /// The pause duration.
        duration: Duration,
    },
}

impl NullAction {
    fn into_item(self) -> WDActions::NullActionItem {
        match self {
            NullAction::Pause { duration } => WDActions::NullActionItem::General(
                WDActions::GeneralAction::Pause(WDActions::PauseAction {
                    duration: Some(duration.as_millis() as u64),
                }),
            ),
        }
    }
}

/// An action performed with a keyboard.
///
/// See [17.4.2 Keyboard Actions](https://www.w3.org/TR/webdriver1/#keyboard-actions) of the
/// WebDriver standard.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum KeyAction {
    /// Pause action.
    /// Useful for adding pauses between other key actions.
    Pause {
        /// The pause duration, given in milliseconds.
        duration: Duration,
    },
    /// Key up action.
    Up {
        /// The key code, e.g. `'a'`. See the [`Key`] enum for special key codes.
        value: char,
    },
    /// Key down action.
    Down {
        /// The key code, e.g. `'a'`. See the [`Key`] enum for special key codes.
        value: char,
    },
}

impl KeyAction {
    fn into_item(self) -> WDActions::KeyActionItem {
        use webdriver::actions::{KeyAction as WDKeyAction, KeyDownAction, KeyUpAction};
        match self {
            KeyAction::Pause { duration } => WDActions::KeyActionItem::General(
                WDActions::GeneralAction::Pause(WDActions::PauseAction {
                    duration: Some(duration.as_millis() as u64),
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

/// Left mouse button constant for use with `PointerAction`.
pub const MOUSE_BUTTON_LEFT: u64 = 0;

/// Middle mouse button constant for use with `PointerAction`.
pub const MOUSE_BUTTON_MIDDLE: u64 = 1;

/// Right mouse button constant for use with `PointerAction`.
pub const MOUSE_BUTTON_RIGHT: u64 = 2;

/// An action performed with a pointer device.
///
/// This can be a mouse, pen or touch device.
///
/// See [17.4.3 Pointer Actions](https://www.w3.org/TR/webdriver1/#pointer-actions) of the
/// WebDriver standard.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PointerAction {
    /// Pause action.
    /// Useful for adding pauses between other key actions.
    Pause {
        /// The pause duration, given in milliseconds.
        duration: Duration,
    },
    /// Pointer button down.
    Down {
        /// The mouse button index.
        ///
        /// The following constants are provided, but any mouse index can be used
        /// to represent the corresponding mouse button.
        /// - [`MOUSE_BUTTON_LEFT`]
        /// - [`MOUSE_BUTTON_MIDDLE`]
        /// - [`MOUSE_BUTTON_RIGHT`]
        button: u64,
    },
    /// Pointer button up.
    Up {
        /// The mouse button index.
        ///
        /// The following constants are provided, but any mouse index can be used
        /// to represent the corresponding mouse button.
        /// - [`MOUSE_BUTTON_LEFT`]
        /// - [`MOUSE_BUTTON_MIDDLE`]
        /// - [`MOUSE_BUTTON_RIGHT`]
        button: u64,
    },
    /// Move the pointer relative to the current position.
    ///
    /// The x and y offsets are relative to the current pointer position.
    MoveBy {
        /// The move duration.
        duration: Option<Duration>,
        /// `x` offset, in pixels.
        x: i64,
        /// `y` offset, in pixels.
        y: i64,
    },
    /// Move the pointer to a new position.
    ///
    /// The x and y offsets are relative to the top-left corner of the viewport.
    MoveTo {
        /// The move duration.
        duration: Option<Duration>,
        /// `x` offset, in pixels.
        x: i64,
        /// `y` offset, in pixels.
        y: i64,
    },
    /// Move the pointer to a position relative to the specified element.
    MoveToElement {
        /// The element to move the pointer in relation to. The `x` and `y` offsets are relative
        /// to this element's center position.
        element: Element,
        /// The move duration.
        duration: Option<Duration>,
        /// `x` offset, in pixels.
        x: i64,
        /// `y` offset, in pixels.
        y: i64,
    },
    /// Pointer cancel action. Used to cancel the current pointer action.
    Cancel,
}

impl PointerAction {
    fn into_item(self) -> WDActions::PointerActionItem {
        match self {
            PointerAction::Pause { duration } => WDActions::PointerActionItem::General(
                WDActions::GeneralAction::Pause(WDActions::PauseAction {
                    duration: Some(duration.as_millis() as u64),
                }),
            ),
            PointerAction::Down { button } => WDActions::PointerActionItem::Pointer(
                WDActions::PointerAction::Down(WDActions::PointerDownAction {
                    button,
                    ..Default::default()
                }),
            ),
            PointerAction::Up { button } => WDActions::PointerActionItem::Pointer(
                WDActions::PointerAction::Up(WDActions::PointerUpAction {
                    button,
                    ..Default::default()
                }),
            ),
            PointerAction::MoveBy { duration, x, y } => WDActions::PointerActionItem::Pointer(
                WDActions::PointerAction::Move(WDActions::PointerMoveAction {
                    duration: duration.map(|x| x.as_millis() as u64),
                    origin: WDActions::PointerOrigin::Pointer,
                    x: Some(x),
                    y: Some(y),
                    ..Default::default()
                }),
            ),
            PointerAction::MoveTo { duration, x, y } => WDActions::PointerActionItem::Pointer(
                WDActions::PointerAction::Move(WDActions::PointerMoveAction {
                    duration: duration.map(|x| x.as_millis() as u64),
                    origin: WDActions::PointerOrigin::Viewport,
                    x: Some(x),
                    y: Some(y),
                    ..Default::default()
                }),
            ),
            PointerAction::MoveToElement {
                element,
                duration,
                x,
                y,
            } => WDActions::PointerActionItem::Pointer(WDActions::PointerAction::Move(
                WDActions::PointerMoveAction {
                    duration: duration.map(|x| x.as_millis() as u64),
                    origin: WDActions::PointerOrigin::Element(element.element),
                    x: Some(x),
                    y: Some(y),
                    ..Default::default()
                },
            )),
            PointerAction::Cancel => {
                WDActions::PointerActionItem::Pointer(WDActions::PointerAction::Cancel)
            }
        }
    }
}

/// A sequence containing [`Null` actions](NullAction).
#[derive(Debug, Clone)]
pub struct NullActions {
    /// A unique identifier to distinguish this input source from others.
    ///
    /// Choose a meaningful string as it may be useful for debugging.
    id: String,
    /// The list of actions for this sequence.
    actions: Vec<NullAction>,
}

impl NullActions {
    /// Create a new NullActions sequence.
    ///
    /// The id can be any string but must uniquely identify this input source.
    pub fn new(id: String) -> Self {
        Self {
            id,
            actions: Vec::new(),
        }
    }
}

impl From<NullActions> for ActionSequence {
    fn from(na: NullActions) -> Self {
        ActionSequence(WDActions::ActionSequence {
            id: na.id,
            actions: WDActions::ActionsType::Null {
                actions: na.actions.into_iter().map(|x| x.into_item()).collect(),
            },
        })
    }
}

/// A sequence containing [`Key` actions](KeyAction).
#[derive(Debug, Clone)]
pub struct KeyActions {
    /// A unique identifier to distinguish this input source from others.
    ///
    /// Choose a meaningful string as it may be useful for debugging.
    id: String,
    /// The list of actions for this sequence.
    actions: Vec<KeyAction>,
}

impl KeyActions {
    /// Create a new KeyActions sequence.
    ///
    /// The id can be any string but must uniquely identify this input source.
    pub fn new(id: String) -> Self {
        Self {
            id,
            actions: Vec::new(),
        }
    }
}

impl From<KeyActions> for ActionSequence {
    fn from(ka: KeyActions) -> Self {
        ActionSequence(WDActions::ActionSequence {
            id: ka.id,
            actions: WDActions::ActionsType::Key {
                actions: ka.actions.into_iter().map(|x| x.into_item()).collect(),
            },
        })
    }
}

/// A sequence containing [`Pointer` actions](PointerAction) for a mouse.
#[derive(Debug, Clone)]
pub struct MouseActions {
    /// A unique identifier to distinguish this input source from others.
    ///
    /// Choose a meaningful string as it may be useful for debugging.
    id: String,
    /// The list of actions for this sequence.
    actions: Vec<PointerAction>,
}

impl MouseActions {
    /// Create a new `MouseActions` sequence.
    ///
    /// The id can be any string but must uniquely identify this input source.
    pub fn new(id: String) -> Self {
        Self {
            id,
            actions: Vec::new(),
        }
    }
}

impl From<MouseActions> for ActionSequence {
    fn from(ma: MouseActions) -> Self {
        ActionSequence(WDActions::ActionSequence {
            id: ma.id,
            actions: WDActions::ActionsType::Pointer {
                parameters: WDActions::PointerActionParameters {
                    pointer_type: WDActions::PointerType::Mouse,
                },
                actions: ma.actions.into_iter().map(|x| x.into_item()).collect(),
            },
        })
    }
}

/// A sequence containing [`Pointer` actions](PointerAction) for a pen device.
#[derive(Debug, Clone)]
pub struct PenActions {
    /// A unique identifier to distinguish this input source from others.
    ///
    /// Choose a meaningful string as it may be useful for debugging.
    id: String,
    /// The list of actions for this sequence.
    actions: Vec<PointerAction>,
}

impl PenActions {
    /// Create a new `PenActions` sequence.
    ///
    /// The id can be any string but must uniquely identify this input source.
    pub fn new(id: String) -> Self {
        Self {
            id,
            actions: Vec::new(),
        }
    }
}

impl From<PenActions> for ActionSequence {
    fn from(pa: PenActions) -> Self {
        ActionSequence(WDActions::ActionSequence {
            id: pa.id,
            actions: WDActions::ActionsType::Pointer {
                parameters: WDActions::PointerActionParameters {
                    pointer_type: WDActions::PointerType::Pen,
                },
                actions: pa.actions.into_iter().map(|x| x.into_item()).collect(),
            },
        })
    }
}

/// A sequence containing [`Pointer` actions](PointerAction) for a touch device.
#[derive(Debug, Clone)]
pub struct TouchActions {
    /// A unique identifier to distinguish this input source from others.
    ///
    /// Choose a meaningful string as it may be useful for debugging.
    id: String,
    /// The list of actions for this sequence.
    actions: Vec<PointerAction>,
}

impl TouchActions {
    /// Create a new `TouchActions` sequence.
    ///
    /// The id can be any string but must uniquely identify this input source.
    pub fn new(id: String) -> Self {
        Self {
            id,
            actions: Vec::new(),
        }
    }
}

impl From<TouchActions> for ActionSequence {
    fn from(ta: TouchActions) -> Self {
        ActionSequence(WDActions::ActionSequence {
            id: ta.id,
            actions: WDActions::ActionsType::Pointer {
                parameters: WDActions::PointerActionParameters {
                    pointer_type: WDActions::PointerType::Touch,
                },
                actions: ta.actions.into_iter().map(|x| x.into_item()).collect(),
            },
        })
    }
}

/// A sequence of actions to be performed.
///
/// See the documentation for [`Actions`] for more details.
#[derive(Debug)]
pub struct ActionSequence(pub(crate) WDActions::ActionSequence);

/// A source capable of providing inputs for a browser action chain.
///
/// See [input source](https://www.w3.org/TR/webdriver1/#dfn-input-sources) in the
/// WebDriver standard.
///
/// Each sequence type implements `InputSource` which provides a `pause()` and a `then()`
/// method. Each call to `pause()` or `then()` represents one tick for this sequence.
pub trait InputSource: Into<ActionSequence> {
    /// The action type associated with this `InputSource`.
    type Action;

    /// Add a pause action to the sequence for this input source.
    #[must_use]
    fn pause(self, duration: Duration) -> Self;

    /// Add the specified action to the sequence for this input source.
    #[must_use]
    fn then(self, action: Self::Action) -> Self;
}

impl InputSource for NullActions {
    type Action = NullAction;

    fn pause(self, duration: Duration) -> Self {
        self.then(NullAction::Pause { duration })
    }

    fn then(mut self, action: Self::Action) -> Self {
        self.actions.push(action);
        self
    }
}

impl InputSource for KeyActions {
    type Action = KeyAction;

    fn pause(self, duration: Duration) -> Self {
        self.then(KeyAction::Pause { duration })
    }

    fn then(mut self, action: Self::Action) -> Self {
        self.actions.push(action);
        self
    }
}

impl InputSource for MouseActions {
    type Action = PointerAction;

    fn pause(self, duration: Duration) -> Self {
        self.then(PointerAction::Pause { duration })
    }

    fn then(mut self, action: Self::Action) -> Self {
        self.actions.push(action);
        self
    }
}

impl InputSource for PenActions {
    type Action = PointerAction;

    fn pause(self, duration: Duration) -> Self {
        self.then(PointerAction::Pause { duration })
    }

    fn then(mut self, action: Self::Action) -> Self {
        self.actions.push(action);
        self
    }
}

impl InputSource for TouchActions {
    type Action = PointerAction;

    fn pause(self, duration: Duration) -> Self {
        self.then(PointerAction::Pause { duration })
    }

    fn then(mut self, action: Self::Action) -> Self {
        self.actions.push(action);
        self
    }
}

/// A list of action sequences to be performed via [`Client::perform_actions()`]
///
/// An [`ActionSequence`] is a sequence of actions of a specific type.
///
/// Multiple sequences can be represented as a table, where each row contains a
/// sequence and each column is 1 "tick" of time. The duration of any given tick
/// will be equal to the longest duration of any individual action in that column.
///
/// See the following example diagram:
///
/// ```ignore
///   Tick ->              1         2                     3
/// |===================================================================
/// | KeyActions        |  Down   |  Up                 |  Pause  |
/// |-------------------------------------------------------------------
/// | PointerActions    |  Down   |  Pause (2 seconds)  |  Up     |
/// |-------------------------------------------------------------------
/// | (other sequence)  |  Pause  |  Pause              |  Pause  |
/// |===================================================================
/// ```
///
/// At tick 1, both a `KeyAction::Down` and a `PointerAction::Down` are triggered
/// simultaneously.
///
/// At tick 2, only a `KeyAction::Up` is triggered. Meanwhile the pointer sequence
/// has a `PointerAction::Pause` for 2 seconds. Note that tick 3 does not start
/// until all of the actions from tick 2 have completed, including any pauses.
///
/// At tick 3, only a `PointerAction::Up` is triggered.
///
/// The bottom sequence is just to show that other sequences can be added. This could
/// be any of `NullActions`, `KeyActions` or `PointerActions`. There is no theoretical
/// limit to the number of sequences that can be specified.
#[derive(Debug, Default)]
pub struct Actions {
    pub(crate) sequences: Vec<ActionSequence>,
}

impl Actions {
    /// Append the specified sequence to the list of sequences.
    #[must_use]
    pub fn and(mut self, sequence: impl Into<ActionSequence>) -> Self {
        self.sequences.push(sequence.into());
        self
    }
}

impl<T> From<T> for Actions
where
    T: Into<ActionSequence>,
{
    fn from(sequence: T) -> Self {
        Self {
            sequences: vec![sequence.into()],
        }
    }
}

impl<T> From<Vec<T>> for Actions
where
    T: Into<ActionSequence>,
{
    fn from(sequences: Vec<T>) -> Self {
        Self {
            sequences: sequences.into_iter().map(|x| x.into()).collect(),
        }
    }
}
