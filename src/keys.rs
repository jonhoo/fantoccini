//! Key codes for use with Actions.

/// Key codes for use with Actions.
#[derive(Debug)]
pub enum Keys {
    /// Null
    Null,
    /// Cancel
    Cancel,
    /// Help
    Help,
    /// Backspace key
    Backspace,
    /// Tab key
    Tab,
    /// Clear
    Clear,
    /// Return key
    Return,
    /// Enter key
    Enter,
    /// Shift key
    Shift,
    /// Control key
    Control,
    /// Alt key
    Alt,
    /// Pause key
    Pause,
    /// Escape key
    Escape,
    /// Space bar
    Space,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,
    /// End key
    End,
    /// Home key
    Home,
    /// Left arrow key
    Left,
    /// Up arrow key
    Up,
    /// Right arrow key
    Right,
    /// Down arrow key
    Down,
    /// Insert key
    Insert,
    /// Delete key
    Delete,
    /// Semicolon key
    Semicolon,
    /// Equals key
    Equals,
    /// Numpad 0 key
    NumPad0,
    /// Numpad 1 key
    NumPad1,
    /// Numpad 2 key
    NumPad2,
    /// Numpad 3 key
    NumPad3,
    /// Numpad 4 key
    NumPad4,
    /// Numpad 5 key
    NumPad5,
    /// Numpad 6 key
    NumPad6,
    /// Numpad 7 key
    NumPad7,
    /// Numpad 8 key
    NumPad8,
    /// Numpad 9 key
    NumPad9,
    /// Multiply key
    Multiply,
    /// Add key
    Add,
    /// Separator key
    Separator,
    /// Subtract key
    Subtract,
    /// Decimal key
    Decimal,
    /// Divide key
    Divide,
    /// F1 key
    F1,
    /// F2 key
    F2,
    /// F3 key
    F3,
    /// F4 key
    F4,
    /// F5 key
    F5,
    /// F6 key
    F6,
    /// F7 key
    F7,
    /// F8 key
    F8,
    /// F9 key
    F9,
    /// F10 key
    F10,
    /// F11 key
    F11,
    /// F12 key
    F12,
    /// Meta key
    Meta,
    /// Command key
    Command,
}

impl From<Keys> for char {
    fn from(k: Keys) -> char {
        match k {
            Keys::Null => '\u{e000}',
            Keys::Cancel => '\u{e001}',
            Keys::Help => '\u{e002}',
            Keys::Backspace => '\u{e003}',
            Keys::Tab => '\u{e004}',
            Keys::Clear => '\u{e005}',
            Keys::Return => '\u{e006}',
            Keys::Enter => '\u{e007}',
            Keys::Shift => '\u{e008}',
            Keys::Control => '\u{e009}',
            Keys::Alt => '\u{e00a}',
            Keys::Pause => '\u{e00b}',
            Keys::Escape => '\u{e00c}',
            Keys::Space => '\u{e00d}',
            Keys::PageUp => '\u{e00e}',
            Keys::PageDown => '\u{e00f}',
            Keys::End => '\u{e010}',
            Keys::Home => '\u{e011}',
            Keys::Left => '\u{e012}',
            Keys::Up => '\u{e013}',
            Keys::Right => '\u{e014}',
            Keys::Down => '\u{e015}',
            Keys::Insert => '\u{e016}',
            Keys::Delete => '\u{e017}',
            Keys::Semicolon => '\u{e018}',
            Keys::Equals => '\u{e019}',
            Keys::NumPad0 => '\u{e01a}',
            Keys::NumPad1 => '\u{e01b}',
            Keys::NumPad2 => '\u{e01c}',
            Keys::NumPad3 => '\u{e01d}',
            Keys::NumPad4 => '\u{e01e}',
            Keys::NumPad5 => '\u{e01f}',
            Keys::NumPad6 => '\u{e020}',
            Keys::NumPad7 => '\u{e021}',
            Keys::NumPad8 => '\u{e022}',
            Keys::NumPad9 => '\u{e023}',
            Keys::Multiply => '\u{e024}',
            Keys::Add => '\u{e025}',
            Keys::Separator => '\u{e026}',
            Keys::Subtract => '\u{e027}',
            Keys::Decimal => '\u{e028}',
            Keys::Divide => '\u{e029}',
            Keys::F1 => '\u{e031}',
            Keys::F2 => '\u{e032}',
            Keys::F3 => '\u{e033}',
            Keys::F4 => '\u{e034}',
            Keys::F5 => '\u{e035}',
            Keys::F6 => '\u{e036}',
            Keys::F7 => '\u{e037}',
            Keys::F8 => '\u{e038}',
            Keys::F9 => '\u{e039}',
            Keys::F10 => '\u{e03a}',
            Keys::F11 => '\u{e03b}',
            Keys::F12 => '\u{e03c}',
            Keys::Meta => '\u{e03d}',
            Keys::Command => '\u{e03d}',
        }
    }
}
