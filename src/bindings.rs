//! Key bindings: which keyboard keys ride the bike, the editor behind the
//! "Tasten" settings view, and the text format of the bindings file.
//!
//! This module names physical keys, so it belongs to the input layer with
//! [`crate::input`]; the headless simulation never sees it (ADR 0009). Reading
//! and writing the file is done by [`crate::save`]; drawing the view by
//! [`crate::settings`].

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::flow::{MenuAction, MenuInput, Screen};

/// Each action has this many key slots.
pub const SLOTS: usize = 2;
/// Larger bindings files are not read.
pub const MAX_KEYS_BYTES: usize = 4 * 1024;
/// The first line of a bindings file is this word followed by the format version.
pub const KEYS_HEADER: &str = "moto-devil-keys";
pub const KEYS_VERSION: u32 = 1;

/// A riding action a key can be bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Gas,
    Brake,
    TiltLeft,
    TiltRight,
    Jump,
    Turn,
    Restart,
}

impl Action {
    /// All actions in the order the settings view lists them.
    pub const ALL: [Action; 7] = [
        Action::Gas,
        Action::Brake,
        Action::TiltLeft,
        Action::TiltRight,
        Action::Jump,
        Action::Turn,
        Action::Restart,
    ];

    /// The name shown in the settings view.
    pub fn name(self) -> &'static str {
        match self {
            Action::Gas => "Gas",
            Action::Brake => "Bremse",
            Action::TiltLeft => "Nach links kippen",
            Action::TiltRight => "Nach rechts kippen",
            Action::Jump => "Springen / Jet",
            Action::Turn => "Umdrehen",
            Action::Restart => "Level neu starten",
        }
    }

    /// The stable name in the bindings file.
    fn id(self) -> &'static str {
        match self {
            Action::Gas => "gas",
            Action::Brake => "brake",
            Action::TiltLeft => "tilt-left",
            Action::TiltRight => "tilt-right",
            Action::Jump => "jump",
            Action::Turn => "turn",
            Action::Restart => "restart",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|a| *a == self).unwrap_or(0)
    }
}

/// Every key that can be bound: the physical key, its stable name in the
/// bindings file, and the label shown when the layout does not tell us better.
/// Escape and Enter are missing on purpose: they always cancel, go back, and
/// confirm, so the menus stay reachable whatever is bound.
const KEYS: &[(KeyCode, &str, &str)] = {
    macro_rules! keys {
        ($($code:ident => $label:expr),* $(,)?) => {
            &[$((KeyCode::$code, stringify!($code), $label)),*]
        };
    }
    keys![
        KeyA => "A", KeyB => "B", KeyC => "C", KeyD => "D", KeyE => "E", KeyF => "F",
        KeyG => "G", KeyH => "H", KeyI => "I", KeyJ => "J", KeyK => "K", KeyL => "L",
        KeyM => "M", KeyN => "N", KeyO => "O", KeyP => "P", KeyQ => "Q", KeyR => "R",
        KeyS => "S", KeyT => "T", KeyU => "U", KeyV => "V", KeyW => "W", KeyX => "X",
        KeyY => "Y", KeyZ => "Z",
        Digit0 => "0", Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4",
        Digit5 => "5", Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9",
        Backquote => "`", Minus => "-", Equal => "=", BracketLeft => "[",
        BracketRight => "]", Backslash => "\\", Semicolon => ";", Quote => "'",
        Comma => ",", Period => ".", Slash => "/", IntlBackslash => "<",
        Space => "Leertaste", Tab => "Tab", Backspace => "Rück",
        ShiftLeft => "Shift links", ShiftRight => "Shift rechts",
        ControlLeft => "Strg links", ControlRight => "Strg rechts",
        AltLeft => "Alt", AltRight => "AltGr",
        ArrowUp => "↑", ArrowDown => "↓", ArrowLeft => "←", ArrowRight => "→",
        Insert => "Einfg", Delete => "Entf", Home => "Pos1", End => "Ende",
        PageUp => "Bild↑", PageDown => "Bild↓",
        Numpad0 => "Num 0", Numpad1 => "Num 1", Numpad2 => "Num 2", Numpad3 => "Num 3",
        Numpad4 => "Num 4", Numpad5 => "Num 5", Numpad6 => "Num 6", Numpad7 => "Num 7",
        Numpad8 => "Num 8", Numpad9 => "Num 9", NumpadAdd => "Num +",
        NumpadSubtract => "Num -", NumpadMultiply => "Num *", NumpadDivide => "Num /",
        NumpadDecimal => "Num ,",
        F1 => "F1", F2 => "F2", F3 => "F3", F4 => "F4", F5 => "F5", F6 => "F6",
        F7 => "F7", F8 => "F8", F9 => "F9", F10 => "F10", F11 => "F11", F12 => "F12",
    ]
};

fn key_entry(code: KeyCode) -> Option<&'static (KeyCode, &'static str, &'static str)> {
    KEYS.iter().find(|(known, ..)| *known == code)
}

/// Whether a label typed by the layout can be shown: one Latin character that
/// the UI font has.
fn is_label_char(c: char) -> bool {
    !c.is_control() && !c.is_whitespace() && (c.is_ascii() || ('\u{A0}'..='\u{17F}').contains(&c))
}

/// A bound key and the label to show for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundKey {
    pub code: KeyCode,
    pub label: String,
}

impl BoundKey {
    /// The key if it can be bound. Keys that type one character (letters,
    /// digits, punctuation) take `typed` as their label, so a German keyboard
    /// shows "Z" for the key that types Z, not the US name of its position.
    pub fn new(code: KeyCode, typed: Option<char>) -> Option<Self> {
        let (_, _, fallback) = key_entry(code)?;
        let types_one_char = fallback.chars().count() == 1;
        let label = match typed {
            Some(c) if types_one_char && is_label_char(c) => c.to_uppercase().collect(),
            _ => (*fallback).to_string(),
        };
        Some(Self { code, label })
    }

    fn of(code: KeyCode) -> Self {
        Self::new(code, None).unwrap_or(Self {
            code,
            label: String::new(),
        })
    }
}

/// The keys bound to each riding action, up to [`SLOTS`] each. A key belongs to
/// at most one slot.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct KeyBindings {
    slots: [[Option<BoundKey>; SLOTS]; Action::ALL.len()],
}

impl Default for KeyBindings {
    /// W/A/S/D and the arrows, Space, E, and R.
    fn default() -> Self {
        let mut bindings = Self::empty();
        let defaults = [
            (Action::Gas, [KeyCode::KeyW, KeyCode::ArrowUp]),
            (Action::Brake, [KeyCode::KeyS, KeyCode::ArrowDown]),
            (Action::TiltLeft, [KeyCode::KeyA, KeyCode::ArrowLeft]),
            (Action::TiltRight, [KeyCode::KeyD, KeyCode::ArrowRight]),
        ];
        for (action, codes) in defaults {
            for (slot, code) in codes.into_iter().enumerate() {
                bindings.bind(action, slot, BoundKey::of(code));
            }
        }
        bindings.bind(Action::Jump, 0, BoundKey::of(KeyCode::Space));
        bindings.bind(Action::Turn, 0, BoundKey::of(KeyCode::KeyE));
        bindings.bind(Action::Restart, 0, BoundKey::of(KeyCode::KeyR));
        bindings
    }
}

impl KeyBindings {
    pub fn empty() -> Self {
        Self {
            slots: Default::default(),
        }
    }

    pub fn slot(&self, action: Action, slot: usize) -> Option<&BoundKey> {
        self.slots[action.index()].get(slot)?.as_ref()
    }

    /// The physical keys bound to `action`.
    pub fn keys(&self, action: Action) -> impl Iterator<Item = KeyCode> + '_ {
        self.slots[action.index()]
            .iter()
            .flatten()
            .map(|key| key.code)
    }

    /// The keys of `action` as the HUD shows them, such as "W/↑", or "–".
    pub fn label(&self, action: Action) -> String {
        let labels: Vec<&str> = self.slots[action.index()]
            .iter()
            .flatten()
            .map(|key| key.label.as_str())
            .collect();
        if labels.is_empty() {
            "–".to_string()
        } else {
            labels.join("/")
        }
    }

    /// Where `code` is bound, if anywhere.
    fn find(&self, code: KeyCode) -> Option<(Action, usize)> {
        Action::ALL.into_iter().find_map(|action| {
            self.slots[action.index()]
                .iter()
                .position(|key| key.as_ref().is_some_and(|key| key.code == code))
                .map(|slot| (action, slot))
        })
    }

    /// Puts `key` into a slot of `action`. A key does one thing only, so it
    /// leaves any other slot it was in; returns the other action it was taken
    /// from.
    pub fn bind(&mut self, action: Action, slot: usize, key: BoundKey) -> Option<Action> {
        if slot >= SLOTS {
            return None;
        }
        let previous = self.find(key.code);
        if let Some((old_action, old_slot)) = previous {
            self.slots[old_action.index()][old_slot] = None;
        }
        self.slots[action.index()][slot] = Some(key);
        previous
            .map(|(old_action, _)| old_action)
            .filter(|old| *old != action)
    }

    pub fn clear(&mut self, action: Action, slot: usize) {
        if let Some(key) = self.slots[action.index()].get_mut(slot) {
            *key = None;
        }
    }
}

/// Writes the bindings in a line-based format like the progress file: a header
/// with the version, then one line per bound key with the action, the key, and
/// its label, separated by tabs:
///
/// ```text
/// moto-devil-keys 1
/// key<TAB>gas<TAB>KeyW<TAB>W
/// key<TAB>jump<TAB>Space<TAB>Leertaste
/// ```
///
/// An action without a line has no keys.
pub fn encode(bindings: &KeyBindings) -> String {
    let mut out = format!("{KEYS_HEADER} {KEYS_VERSION}\n");
    for action in Action::ALL {
        for key in bindings.slots[action.index()].iter().flatten() {
            if let Some((_, name, _)) = key_entry(key.code) {
                out.push_str(&format!("key\t{}\t{name}\t{}\n", action.id(), key.label));
            }
        }
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
pub enum KeysError {
    TooLarge,
    MissingHeader,
    UnsupportedVersion(String),
}

impl std::fmt::Display for KeysError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "the bindings file is larger than {MAX_KEYS_BYTES} bytes"),
            Self::MissingHeader => {
                write!(f, "the bindings file does not start with `{KEYS_HEADER}`")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "bindings format version {version} is not supported")
            }
        }
    }
}

/// Reads a bindings file. The header must match; after that, lines that break
/// a rule are skipped: unknown actions or keys, keys bound twice, more than
/// [`SLOTS`] keys for an action. A label that cannot be shown falls back to the
/// key's own name.
pub fn decode(text: &str) -> Result<KeyBindings, KeysError> {
    if text.len() > MAX_KEYS_BYTES {
        return Err(KeysError::TooLarge);
    }
    let mut lines = text.lines();
    let header = lines.next().ok_or(KeysError::MissingHeader)?;
    let version = header
        .strip_prefix(KEYS_HEADER)
        .and_then(|rest| rest.strip_prefix(' '))
        .ok_or(KeysError::MissingHeader)?;
    if version.trim() != KEYS_VERSION.to_string() {
        return Err(KeysError::UnsupportedVersion(
            version.trim().chars().take(16).collect(),
        ));
    }

    let mut bindings = KeyBindings::empty();
    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        let ["key", action, name, rest @ ..] = fields.as_slice() else {
            continue;
        };
        let Some(action) = Action::ALL.into_iter().find(|a| a.id() == *action) else {
            continue;
        };
        let Some((code, ..)) = KEYS.iter().find(|(_, known, _)| known == name) else {
            continue;
        };
        let mut label = rest.first().map_or("", |label| *label).chars();
        let typed = match (label.next(), label.next()) {
            (Some(c), None) => Some(c),
            _ => None,
        };
        let Some(key) = BoundKey::new(*code, typed) else {
            continue;
        };
        let free = bindings.slots[action.index()]
            .iter()
            .position(Option::is_none);
        if let Some(slot) = free
            && bindings.find(key.code).is_none()
        {
            bindings.slots[action.index()][slot] = Some(key);
        }
    }
    Ok(bindings)
}

/// The settings view's rows: one per action, then "restore defaults".
pub const RESET_ROW: usize = Action::ALL.len();

/// The cursor of the settings view and whether it waits for a key.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct BindingEditor {
    pub row: usize,
    pub slot: usize,
    /// The next key pressed goes into the selected slot.
    pub waiting: bool,
    /// What the last change did, shown under the list.
    pub notice: Option<String>,
}

impl BindingEditor {
    pub fn action(&self) -> Option<Action> {
        Action::ALL.get(self.row).copied()
    }

    /// Handles one key press in the settings view. Arrows move, Enter starts
    /// waiting for a key (or restores the defaults), Backspace/Delete clear a
    /// slot, and Escape goes back, which the caller hands to the flow. While
    /// waiting, Escape cancels and any bindable key is bound.
    pub fn press(
        &mut self,
        code: KeyCode,
        typed: Option<char>,
        bindings: &mut KeyBindings,
    ) -> Option<MenuAction> {
        if self.waiting {
            if code == KeyCode::Escape {
                self.waiting = false;
                self.notice = None;
                return None;
            }
            let Some(action) = self.action() else {
                self.waiting = false;
                return None;
            };
            let Some(key) = BoundKey::new(code, typed) else {
                self.notice = Some("Diese Taste gehört zum Menü.".into());
                return None;
            };
            let label = key.label.clone();
            self.waiting = false;
            self.notice = bindings.bind(action, self.slot, key).map(|old| {
                format!(
                    "{label} gehört jetzt zu {} statt zu {}.",
                    action.name(),
                    old.name()
                )
            });
            return None;
        }

        self.notice = None;
        match code {
            KeyCode::ArrowUp => self.row = self.row.saturating_sub(1),
            KeyCode::ArrowDown => self.row = (self.row + 1).min(RESET_ROW),
            KeyCode::ArrowLeft => self.slot = self.slot.saturating_sub(1),
            KeyCode::ArrowRight => self.slot = (self.slot + 1).min(SLOTS - 1),
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if self.action().is_some() {
                    self.waiting = true;
                } else {
                    *bindings = KeyBindings::default();
                    self.notice = Some("Standardtasten wiederhergestellt.".into());
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if let Some(action) = self.action() {
                    bindings.clear(action, self.slot);
                }
            }
            KeyCode::Escape => return Some(MenuAction::Back),
            _ => {}
        }
        None
    }
}

/// Starts the settings view at the top, not waiting for a key.
pub fn reset_editor(mut editor: ResMut<BindingEditor>) {
    *editor = BindingEditor::default();
}

/// Feeds key presses to the [`BindingEditor`] on the settings screen. It reads
/// every frame so it never sees presses from another screen.
pub fn edit_bindings(
    screen: Res<State<Screen>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut editor: ResMut<BindingEditor>,
    mut bindings: ResMut<KeyBindings>,
    mut menu: ResMut<MenuInput>,
) {
    let presses: Vec<&KeyboardInput> = keyboard
        .read()
        .filter(|event| event.state.is_pressed() && !event.repeat)
        .collect();
    if *screen.get() != Screen::Settings || presses.is_empty() {
        return;
    }
    // Change the resources only when something changed, so the view redraws and
    // the file is written only then.
    let mut next_editor = editor.clone();
    let mut next_bindings = bindings.clone();
    for event in presses {
        let typed = match &event.logical_key {
            Key::Character(text) => {
                let mut chars = text.chars();
                chars.next().filter(|_| chars.next().is_none())
            }
            _ => None,
        };
        if let Some(action) = next_editor.press(event.key_code, typed, &mut next_bindings) {
            menu.0.push(action);
        }
    }
    if next_editor != *editor {
        *editor = next_editor;
    }
    if next_bindings != *bindings {
        *bindings = next_bindings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_press(
        editor: &mut BindingEditor,
        bindings: &mut KeyBindings,
        code: KeyCode,
    ) -> Option<MenuAction> {
        editor.press(code, None, bindings)
    }

    #[test]
    fn defaults_are_wasd_and_the_arrows() {
        let bindings = KeyBindings::default();
        assert_eq!(bindings.label(Action::Gas), "W/↑");
        assert_eq!(bindings.label(Action::TiltLeft), "A/←");
        assert_eq!(bindings.label(Action::Jump), "Leertaste");
        assert_eq!(
            bindings.keys(Action::Restart).collect::<Vec<_>>(),
            [KeyCode::KeyR]
        );
    }

    #[test]
    fn a_key_moves_to_its_new_action() {
        let mut bindings = KeyBindings::default();
        let taken = bindings.bind(Action::Jump, 1, BoundKey::of(KeyCode::KeyW));
        assert_eq!(taken, Some(Action::Gas));
        assert_eq!(bindings.label(Action::Gas), "↑");
        assert_eq!(bindings.label(Action::Jump), "Leertaste/W");

        // Moving a key within its own action is not a conflict.
        assert_eq!(
            bindings.bind(Action::Jump, 0, BoundKey::of(KeyCode::KeyW)),
            None
        );
        assert_eq!(bindings.label(Action::Jump), "W");
    }

    #[test]
    fn menu_keys_cannot_be_bound_and_layouts_name_their_letters() {
        assert_eq!(BoundKey::new(KeyCode::Escape, None), None);
        assert_eq!(BoundKey::new(KeyCode::Enter, None), None);
        assert_eq!(BoundKey::new(KeyCode::KeyY, Some('z')).unwrap().label, "Z");
        assert_eq!(
            BoundKey::new(KeyCode::Semicolon, Some('ö')).unwrap().label,
            "Ö"
        );
        assert_eq!(
            BoundKey::new(KeyCode::Numpad1, Some('1')).unwrap().label,
            "Num 1",
            "keys with a longer name keep it"
        );
        assert_eq!(BoundKey::new(KeyCode::KeyQ, Some('日')).unwrap().label, "Q");
    }

    #[test]
    fn the_editor_rebinds_clears_and_restores() {
        let mut editor = BindingEditor::default();
        let mut bindings = KeyBindings::default();

        // Gas, second slot: Enter, then a key.
        editor_press(&mut editor, &mut bindings, KeyCode::ArrowRight);
        editor_press(&mut editor, &mut bindings, KeyCode::Enter);
        assert!(editor.waiting);
        assert_eq!(
            editor_press(&mut editor, &mut bindings, KeyCode::Escape),
            None,
            "Escape cancels waiting without leaving"
        );
        assert!(!editor.waiting);
        editor_press(&mut editor, &mut bindings, KeyCode::Enter);
        editor_press(&mut editor, &mut bindings, KeyCode::Enter);
        assert!(editor.waiting, "Enter cannot be bound");
        editor_press(&mut editor, &mut bindings, KeyCode::KeyK);
        assert!(!editor.waiting);
        assert_eq!(bindings.label(Action::Gas), "W/K");

        // Taking Space from the jump says so.
        editor_press(&mut editor, &mut bindings, KeyCode::Enter);
        editor_press(&mut editor, &mut bindings, KeyCode::Space);
        assert_eq!(bindings.label(Action::Gas), "W/Leertaste");
        assert_eq!(bindings.label(Action::Jump), "–");
        assert!(
            editor
                .notice
                .as_deref()
                .is_some_and(|n| n.contains("Springen"))
        );

        // Backspace clears the selected slot.
        editor_press(&mut editor, &mut bindings, KeyCode::ArrowLeft);
        editor_press(&mut editor, &mut bindings, KeyCode::Backspace);
        assert_eq!(bindings.label(Action::Gas), "Leertaste");

        // The last row restores the defaults; Escape leaves.
        for _ in 0..RESET_ROW + 3 {
            editor_press(&mut editor, &mut bindings, KeyCode::ArrowDown);
        }
        assert_eq!(editor.row, RESET_ROW);
        editor_press(&mut editor, &mut bindings, KeyCode::Enter);
        assert!(!editor.waiting);
        assert_eq!(bindings, KeyBindings::default());
        assert_eq!(
            editor_press(&mut editor, &mut bindings, KeyCode::Escape),
            Some(MenuAction::Back)
        );
    }

    #[test]
    fn the_bindings_format_round_trips() {
        let mut bindings = KeyBindings::default();
        bindings.bind(
            Action::Jump,
            1,
            BoundKey::new(KeyCode::KeyY, Some('z')).unwrap(),
        );
        bindings.clear(Action::Restart, 0);
        bindings.bind(Action::Turn, 1, BoundKey::of(KeyCode::Numpad5));
        let text = encode(&bindings);
        assert!(text.starts_with("moto-devil-keys 1\nkey\tgas\tKeyW\tW\n"));
        assert!(text.contains("key\tjump\tKeyY\tZ\n"));
        assert_eq!(decode(&text), Ok(bindings));
    }

    #[test]
    fn damaged_lines_are_skipped() {
        let text = "moto-devil-keys 1\n\
            key\tgas\tKeyW\tW\n\
            key\tgas\tKeyW\tW\n\
            key\tbrake\tKeyW\tW\n\
            key\tfly\tKeyF\tF\n\
            key\tbrake\tEscape\tEsc\n\
            key\tbrake\tKeyS\t\u{1b}\n\
            key\tjump\tSpace\tAnything\n\
            key\tturn\tKeyQ\n\
            key\tturn\tKeyE\tE\n\
            key\tturn\tKeyR\tR\n\
            nonsense\n";
        let bindings = decode(text).unwrap();
        assert_eq!(bindings.label(Action::Gas), "W");
        assert_eq!(bindings.label(Action::Brake), "S", "bad label falls back");
        assert_eq!(bindings.label(Action::Jump), "Leertaste");
        assert_eq!(bindings.label(Action::Turn), "Q/E", "at most two keys");
        assert_eq!(bindings.label(Action::Restart), "–");
    }

    #[test]
    fn foreign_or_oversized_files_are_rejected() {
        assert_eq!(decode(""), Err(KeysError::MissingHeader));
        assert_eq!(decode("moto-devil-save 1\n"), Err(KeysError::MissingHeader));
        assert_eq!(
            decode("moto-devil-keys 2\n"),
            Err(KeysError::UnsupportedVersion("2".into()))
        );
        let huge = format!("moto-devil-keys 1\n{}", "x".repeat(MAX_KEYS_BYTES));
        assert_eq!(decode(&huge), Err(KeysError::TooLarge));
    }

    #[test]
    fn the_settings_screen_reads_keyboard_presses() {
        use bevy::input::ButtonState;
        use bevy::state::app::StatesPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .insert_state(Screen::Settings)
            .add_message::<KeyboardInput>()
            .init_resource::<KeyBindings>()
            .init_resource::<BindingEditor>()
            .init_resource::<MenuInput>()
            .add_systems(Update, edit_bindings);
        let press = |app: &mut App, key_code, logical_key| {
            app.world_mut().write_message(KeyboardInput {
                key_code,
                logical_key,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            app.update();
        };
        press(&mut app, KeyCode::Enter, Key::Enter);
        press(&mut app, KeyCode::KeyY, Key::Character("z".into()));
        assert_eq!(
            app.world().resource::<KeyBindings>().label(Action::Gas),
            "Z/↑"
        );
        press(&mut app, KeyCode::Escape, Key::Escape);
        assert_eq!(app.world().resource::<MenuInput>().0, [MenuAction::Back]);
    }
}
