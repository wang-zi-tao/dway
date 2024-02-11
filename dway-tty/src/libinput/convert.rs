use super::KeyLockState;
use bevy::{
    input::keyboard::{Key, NativeKey, NativeKeyCode},
    prelude::{ButtonInput, KeyCode},
};
use dway_util::keys::*;
use input::{event::tablet_pad::KeyState, Device};
use smol_str::SmolStr;
use tracing::warn;

#[allow(non_upper_case_globals)]
pub fn convert_keycode(
    code: u32,
    input_state: &ButtonInput<KeyCode>,
    state: KeyState,
    lock_state: &mut KeyLockState,
    device: &mut Device,
) -> (KeyCode, Key) {
    fn char_key(s: &str) -> Key {
        Key::Character(s.into())
    };
    fn undefined_key(code: u32) -> (KeyCode, Key) {
        (
            KeyCode::Unidentified(NativeKeyCode::Xkb(code)),
            Key::Unidentified(NativeKey::Xkb(code)),
        )
    }

    let (mut keycode, mut logicalkey) = match code {
        KEY_RESERVED => undefined_key(code),
        KEY_ESC => (KeyCode::Escape, Key::Escape),
        KEY_1 => (KeyCode::Digit1, Key::Character("1".into())),
        KEY_2 => (KeyCode::Digit2, Key::Character("2".into())),
        KEY_3 => (KeyCode::Digit3, Key::Character("3".into())),
        KEY_4 => (KeyCode::Digit4, Key::Character("4".into())),
        KEY_5 => (KeyCode::Digit5, Key::Character("5".into())),
        KEY_6 => (KeyCode::Digit6, Key::Character("6".into())),
        KEY_7 => (KeyCode::Digit7, Key::Character("7".into())),
        KEY_8 => (KeyCode::Digit8, Key::Character("8".into())),
        KEY_9 => (KeyCode::Digit9, Key::Character("9".into())),
        KEY_0 => (KeyCode::Digit0, Key::Character("0".into())),
        KEY_MINUS => (KeyCode::Minus, Key::Character("-".into())),
        KEY_EQUAL => (KeyCode::Equal, Key::Character("=".into())),
        KEY_BACKSPACE => (KeyCode::Backspace, Key::Backspace),
        KEY_TAB => (KeyCode::Tab, Key::Tab),
        KEY_Q => (KeyCode::KeyQ, Key::Character("q".into())),
        KEY_W => (KeyCode::KeyW, Key::Character("w".into())),
        KEY_E => (KeyCode::KeyE, Key::Character("e".into())),
        KEY_R => (KeyCode::KeyR, Key::Character("r".into())),
        KEY_T => (KeyCode::KeyT, Key::Character("t".into())),
        KEY_Y => (KeyCode::KeyY, Key::Character("y".into())),
        KEY_U => (KeyCode::KeyU, Key::Character("u".into())),
        KEY_I => (KeyCode::KeyI, Key::Character("i".into())),
        KEY_O => (KeyCode::KeyO, Key::Character("o".into())),
        KEY_P => (KeyCode::KeyP, Key::Character("p".into())),
        KEY_LEFTBRACE => (KeyCode::BracketLeft, Key::Character("[".into())),
        KEY_RIGHTBRACE => (KeyCode::BracketRight, Key::Character("]".into())),
        KEY_ENTER => (KeyCode::Enter, Key::Enter),
        KEY_LEFTCTRL => (KeyCode::ControlLeft, Key::Control),
        KEY_A => (KeyCode::KeyA, Key::Character("a".into())),
        KEY_S => (KeyCode::KeyS, Key::Character("s".into())),
        KEY_D => (KeyCode::KeyD, Key::Character("d".into())),
        KEY_F => (KeyCode::KeyF, Key::Character("f".into())),
        KEY_G => (KeyCode::KeyG, Key::Character("g".into())),
        KEY_H => (KeyCode::KeyH, Key::Character("h".into())),
        KEY_J => (KeyCode::KeyJ, Key::Character("j".into())),
        KEY_K => (KeyCode::KeyK, Key::Character("k".into())),
        KEY_L => (KeyCode::KeyL, Key::Character("l".into())),
        KEY_SEMICOLON => (KeyCode::Semicolon, Key::Character(";".into())),
        KEY_APOSTROPHE => (KeyCode::Quote, Key::Character("'".into())),
        KEY_GRAVE => (KeyCode::Backquote, Key::Unidentified(NativeKey::Xkb(code))),
        KEY_LEFTSHIFT => (KeyCode::ShiftLeft, Key::Shift),
        KEY_BACKSLASH => (KeyCode::Backslash, Key::Character("\\".into())),
        KEY_Z => (KeyCode::KeyZ, Key::Character("z".into())),
        KEY_X => (KeyCode::KeyX, Key::Character("x".into())),
        KEY_C => (KeyCode::KeyC, Key::Character("c".into())),
        KEY_V => (KeyCode::KeyV, Key::Character("v".into())),
        KEY_B => (KeyCode::KeyB, Key::Character("b".into())),
        KEY_N => (KeyCode::KeyN, Key::Character("n".into())),
        KEY_M => (KeyCode::KeyM, Key::Character("m".into())),
        KEY_COMMA => (KeyCode::Comma, Key::Character(",".into())),
        KEY_DOT => (KeyCode::Period, Key::Character(".".into())),
        KEY_SLASH => (KeyCode::Slash, Key::Character("/".into())),
        KEY_RIGHTSHIFT => (KeyCode::ShiftRight, Key::Shift),
        KEY_LEFTALT => (KeyCode::AltLeft, Key::Alt),
        KEY_SPACE => (KeyCode::Space, Key::Space),
        KEY_CAPSLOCK => (KeyCode::CapsLock, Key::CapsLock),
        KEY_F1 => (KeyCode::F1, Key::F1),
        KEY_F2 => (KeyCode::F2, Key::F2),
        KEY_F3 => (KeyCode::F3, Key::F3),
        KEY_F4 => (KeyCode::F4, Key::F4),
        KEY_F5 => (KeyCode::F5, Key::F5),
        KEY_F6 => (KeyCode::F6, Key::F6),
        KEY_F7 => (KeyCode::F7, Key::F7),
        KEY_F8 => (KeyCode::F8, Key::F8),
        KEY_F9 => (KeyCode::F9, Key::F9),
        KEY_F10 => (KeyCode::F10, Key::F10),
        KEY_NUMLOCK => (KeyCode::NumLock, Key::NumLock),
        KEY_SCROLLLOCK => (KeyCode::ScrollLock, Key::ScrollLock),
        KEY_KP7 => (KeyCode::Numpad7, Key::Character("7".into())),
        KEY_KP8 => (KeyCode::Numpad8, Key::Character("8".into())),
        KEY_KP9 => (KeyCode::Numpad9, Key::Character("9".into())),
        KEY_KPMINUS => (KeyCode::NumpadSubtract, Key::Character("-".into())),
        KEY_KP4 => (KeyCode::Numpad4, Key::Character("4".into())),
        KEY_KP5 => (KeyCode::Numpad5, Key::Character("5".into())),
        KEY_KP6 => (KeyCode::Numpad6, Key::Character("6".into())),
        KEY_KPPLUS => (KeyCode::NumpadAdd, Key::Character("+".into())),
        KEY_KP1 => (KeyCode::Numpad1, Key::Character("1".into())),
        KEY_KP2 => (KeyCode::Numpad2, Key::Character("2".into())),
        KEY_KP3 => (KeyCode::Numpad3, Key::Character("3".into())),
        KEY_KP0 => (KeyCode::Numpad0, Key::Character("0".into())),
        KEY_KPDOT => (KeyCode::NumpadDecimal, Key::Character(".".into())),
        // KEY_ZENKAKUHANKAKU => Key1,
        // KEY_102ND => Key1,
        KEY_F11 => (KeyCode::F11, Key::F11),
        KEY_F12 => (KeyCode::F12, Key::F12),
        // KEY_RO => Key1,
        // KEY_KATAKANA => Key1,
        // KEY_HIRAGANA => Key1,
        // KEY_HENKAN => Key1,
        // KEY_KATAKANAHIRAGANA => Key1,
        // KEY_MUHENKAN => Key1,
        KEY_KPJPCOMMA => (KeyCode::NumpadComma, Key::Character(",".into())),
        KEY_KPENTER => (KeyCode::NumpadEnter, Key::Enter),
        KEY_RIGHTCTRL => (KeyCode::ControlRight, Key::Control),
        KEY_KPSLASH => (KeyCode::NumpadDivide, Key::Character("/".into())),
        KEY_SYSRQ => undefined_key(code),
        KEY_RIGHTALT => (KeyCode::AltRight, Key::Alt),
        // KEY_LINEFEED => Key1,
        KEY_HOME => (KeyCode::Home, Key::Home),
        KEY_UP => (KeyCode::ArrowUp, Key::ArrowUp),
        KEY_PAGEUP => (KeyCode::PageUp, Key::PageUp),
        KEY_LEFT => (KeyCode::ArrowLeft, Key::ArrowLeft),
        KEY_RIGHT => (KeyCode::ArrowRight, Key::ArrowRight),
        KEY_END => (KeyCode::End, Key::End),
        KEY_DOWN => (KeyCode::ArrowDown, Key::ArrowDown),
        KEY_PAGEDOWN => (KeyCode::PageDown, Key::Character("".into())),
        KEY_INSERT => (KeyCode::Insert, Key::Character("".into())),
        KEY_DELETE => (KeyCode::Delete, Key::Character("".into())),
        // KEY_MACRO => Key1,
        KEY_MUTE => (KeyCode::AudioVolumeMute, Key::AudioVolumeMute),
        KEY_VOLUMEDOWN => (KeyCode::AudioVolumeDown, Key::AudioVolumeDown),
        KEY_VOLUMEUP => (KeyCode::AudioVolumeUp, Key::AudioVolumeUp),
        KEY_POWER => (KeyCode::Power, Key::Power),
        KEY_KPEQUAL => (KeyCode::NumpadEqual, Key::Character("=".into())),
        KEY_KPPLUSMINUS => (KeyCode::NumpadSubtract, Key::Character("-".into())),
        KEY_PAUSE => (KeyCode::Pause, Key::Pause),
        // KEY_SCALE => Key1, /* AL Compiz Scale (Expose) */
        KEY_KPCOMMA => (KeyCode::NumpadComma, Key::Character(",".into())),
        // KEY_HANGEUL => Key1,
        // KEY_HANGUEL => Key1,
        // KEY_HANJA => Key1,
        KEY_YEN => (KeyCode::IntlYen, undefined_key(code).1),
        KEY_LEFTMETA => (KeyCode::SuperLeft, Key::Super),
        KEY_RIGHTMETA => (KeyCode::SuperRight, Key::Super),
        KEY_COMPOSE => undefined_key(code),
        KEY_STOP => (KeyCode::MediaStop, Key::MediaStop),
        KEY_AGAIN => (KeyCode::Again, Key::Again),
        KEY_PROPS => (KeyCode::Props, Key::Props), /* AC Properties */
        KEY_UNDO => (KeyCode::Undo, Key::Undo),    /* AC Undo */
        // KEY_FRONT => WebForward,
        KEY_COPY => (KeyCode::Copy, Key::Copy), /* AC Copy */
        KEY_OPEN => (KeyCode::Open, Key::Open), /* AC Open */
        KEY_PASTE => (KeyCode::Paste, Key::Pause), /* AC Paste */
        KEY_FIND => (KeyCode::BrowserSearch, Key::BrowserSearch), /* AC Search */
        KEY_CUT => (KeyCode::Cut, Key::Cut),    /* AC Cut */
        KEY_HELP => (KeyCode::Help, Key::Help), /* AL Integrated Help Center */
        KEY_MENU => (KeyCode::ContextMenu, Key::ContextMenu), /* Menu (show menu) */
        KEY_CALC => undefined_key(code),        /* AL Calculator */
        // KEY_SETUP => Key1,
        KEY_SLEEP => (KeyCode::Sleep, undefined_key(code).1), /* SC System Sleep */
        KEY_WAKEUP => (KeyCode::WakeUp, Key::WakeUp),         /* System Wake Up */
        // KEY_FILE => Key1,   /* AL Local Machine Browser */
        // KEY_SENDFILE => Key1,
        // KEY_DELETEFILE => Key1,
        // KEY_XFER => Key1,
        // KEY_PROG1 => Key1,
        // KEY_PROG2 => Key1,
        // KEY_WWW => Key1, /* AL Internet Browser */
        // KEY_MSDOS => Key1,
        // KEY_COFFEE => Key1, /* AL Terminal Lock/Screensaver */
        // KEY_SCREENLOCK => Key1,
        // KEY_ROTATE_DISPLAY => Key1, /* Display orientation for e.g. tablets */
        // KEY_DIRECTION => Key1,
        // KEY_CYCLEWINDOWS => Key1,
        KEY_MAIL => (KeyCode::LaunchMail, Key::LaunchMail),
        KEY_BOOKMARKS => undefined_key(code), /* AC Bookmarks */
        KEY_COMPUTER => undefined_key(code),
        KEY_BACK => (KeyCode::BrowserBack, Key::BrowserBack), /* AC Back */
        KEY_FORWARD => (KeyCode::BrowserForward, Key::BrowserForward), /* AC Forward */
        // KEY_CLOSECD => Key1,
        // KEY_EJECTCD => Key1,
        // KEY_EJECTCLOSECD => Key1,
        KEY_NEXTSONG => (KeyCode::MediaTrackNext, Key::MediaTrackNext),
        KEY_PLAYPAUSE => (KeyCode::MediaPlayPause, Key::MediaPlayPause),
        KEY_PREVIOUSSONG => (KeyCode::MediaTrackPrevious, Key::MediaTrackPrevious),
        KEY_STOPCD => undefined_key(code),
        // KEY_RECORD => Key1,
        // KEY_REWIND => Key1,
        KEY_PHONE => undefined_key(code), /* Media Select Telephone */
        // KEY_ISO => Key1,
        // KEY_CONFIG => Key1, /* AL Consumer Control Configuration */
        KEY_HOMEPAGE => (KeyCode::BrowserHome, Key::BrowserHome), /* AC Home */
        KEY_REFRESH => (KeyCode::BrowserRefresh, Key::BrowserRefresh), /* AC Refresh */
        // KEY_EXIT => Key1,   /* AC Exit */
        // KEY_MOVE => Key1,
        // KEY_EDIT => Key1,
        // KEY_SCROLLUP => Key1,
        // KEY_SCROLLDOWN => Key1,
        // KEY_KPLEFTPAREN => Key1,
        // KEY_KPRIGHTPAREN => Key1,
        // KEY_NEW => Key1,  /* AC New */
        // KEY_REDO => Key1, /* AC Redo/Repeat */
        KEY_F13 => (KeyCode::F13, Key::F13),
        KEY_F14 => (KeyCode::F14, Key::F14),
        KEY_F15 => (KeyCode::F15, Key::F15),
        KEY_F16 => (KeyCode::F16, Key::F16),
        KEY_F17 => (KeyCode::F17, Key::F17),
        KEY_F18 => (KeyCode::F18, Key::F18),
        KEY_F19 => (KeyCode::F19, Key::F19),
        KEY_F20 => (KeyCode::F20, Key::F20),
        KEY_F21 => (KeyCode::F21, Key::F21),
        KEY_F22 => (KeyCode::F22, Key::F22),
        KEY_F23 => (KeyCode::F23, Key::F23),
        KEY_F24 => (KeyCode::F24, Key::F24),
        KEY_PRINT => (KeyCode::PrintScreen, Key::PrintScreen), /* AC Print */
        KEY_EMAIL => (KeyCode::LaunchMail, Key::LaunchMail),
        _ => undefined_key(code),
    };

    let shift = input_state.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    if shift {
        logicalkey = match keycode {
            KeyCode::Digit1 => Key::Character("!".into()),
            KeyCode::Digit2 => Key::Character("@".into()),
            KeyCode::Digit3 => Key::Character("#".into()),
            KeyCode::Digit4 => Key::Character("$".into()),
            KeyCode::Digit5 => Key::Character("%".into()),
            KeyCode::Digit6 => Key::Character("^".into()),
            KeyCode::Digit7 => Key::Character("&".into()),
            KeyCode::Digit8 => Key::Character("*".into()),
            KeyCode::Digit9 => Key::Character("(".into()),
            KeyCode::Digit0 => Key::Character(")".into()),
            KeyCode::Minus => Key::Character("_".into()),
            KeyCode::Equal => Key::Character("+".into()),
            KeyCode::KeyQ => Key::Character("Q".into()),
            KeyCode::KeyW => Key::Character("W".into()),
            KeyCode::KeyE => Key::Character("E".into()),
            KeyCode::KeyR => Key::Character("R".into()),
            KeyCode::KeyT => Key::Character("T".into()),
            KeyCode::KeyY => Key::Character("Y".into()),
            KeyCode::KeyU => Key::Character("U".into()),
            KeyCode::KeyI => Key::Character("I".into()),
            KeyCode::KeyO => Key::Character("O".into()),
            KeyCode::KeyP => Key::Character("P".into()),
            KeyCode::BracketLeft => Key::Character("{".into()),
            KeyCode::BracketRight => Key::Character("}".into()),
            KeyCode::Backslash => Key::Character("|".into()),
            KeyCode::KeyA => Key::Character("A".into()),
            KeyCode::KeyS => Key::Character("S".into()),
            KeyCode::KeyD => Key::Character("D".into()),
            KeyCode::KeyF => Key::Character("F".into()),
            KeyCode::KeyG => Key::Character("G".into()),
            KeyCode::KeyH => Key::Character("H".into()),
            KeyCode::KeyJ => Key::Character("J".into()),
            KeyCode::KeyK => Key::Character("K".into()),
            KeyCode::KeyL => Key::Character("L".into()),
            KeyCode::Semicolon => Key::Character(":".into()),
            KeyCode::Quote => Key::Character("\"".into()),
            KeyCode::KeyZ => Key::Character("Z".into()),
            KeyCode::KeyX => Key::Character("X".into()),
            KeyCode::KeyC => Key::Character("C".into()),
            KeyCode::KeyV => Key::Character("V".into()),
            KeyCode::KeyB => Key::Character("B".into()),
            KeyCode::KeyN => Key::Character("N".into()),
            KeyCode::KeyM => Key::Character("M".into()),
            KeyCode::Comma => Key::Character("<".into()),
            KeyCode::Period => Key::Character(">".into()),
            KeyCode::Slash => Key::Character("?".into()),
            _ => logicalkey,
        };
    }

    if !lock_state.number_lock {
        logicalkey = match keycode {
            KeyCode::Numpad0 => Key::Insert,
            KeyCode::NumpadDecimal => Key::Delete,
            KeyCode::Numpad1 => Key::End,
            KeyCode::Numpad2 => Key::ArrowDown,
            KeyCode::Numpad3 => Key::PageDown,
            KeyCode::Numpad4 => Key::ArrowLeft,
            KeyCode::Numpad6 => Key::ArrowRight,
            KeyCode::Numpad7 => Key::Home,
            KeyCode::Numpad8 => Key::ArrowUp,
            KeyCode::Numpad9 => Key::PageUp,
            _ => logicalkey,
        };
    }

    if state == KeyState::Pressed {
        match keycode {
            Numlock => {
                lock_state.number_lock = !lock_state.number_lock;
                device.led_update(lock_state.led());
            }
            Scroll => {
                lock_state.scoll_lock = !lock_state.scoll_lock;
                device.led_update(lock_state.led());
            }
            Capital => {
                lock_state.caps_lock = !lock_state.caps_lock;
                device.led_update(lock_state.led());
            }
            _ => {}
        }
    }

    (keycode, logicalkey)
}
