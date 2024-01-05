use super::{keys::*, KeyLockState};
use bevy::prelude::{Input, KeyCode};
use input::{event::tablet_pad::KeyState, Device};
use tracing::warn;

#[allow(non_upper_case_globals)]
pub fn convert_keycode(
    code: u32,
    input_state: &Input<KeyCode>,
    state: KeyState,
    lock_state: &mut KeyLockState,
    device: &mut Device,
) -> Option<KeyCode> {
    use KeyCode::*;

    let mut keycode = match code {
        KEY_ESC => Escape,
        KEY_1 => Key1,
        KEY_2 => Key2,
        KEY_3 => Key3,
        KEY_4 => Key4,
        KEY_5 => Key5,
        KEY_6 => Key6,
        KEY_7 => Key7,
        KEY_8 => Key8,
        KEY_9 => Key9,
        KEY_0 => Key0,
        KEY_MINUS => Minus,
        KEY_EQUAL => Equals,
        KEY_BACKSPACE => Back,
        KEY_TAB => Tab,
        KEY_Q => Q,
        KEY_W => W,
        KEY_E => E,
        KEY_R => R,
        KEY_T => T,
        KEY_Y => Y,
        KEY_U => U,
        KEY_I => I,
        KEY_O => O,
        KEY_P => P,
        KEY_LEFTBRACE => BracketLeft,
        KEY_RIGHTBRACE => BracketRight,
        KEY_ENTER => Return,
        KEY_LEFTCTRL => ControlLeft,
        KEY_A => A,
        KEY_S => S,
        KEY_D => D,
        KEY_F => F,
        KEY_G => G,
        KEY_H => H,
        KEY_J => J,
        KEY_K => K,
        KEY_L => L,
        KEY_SEMICOLON => Semicolon,
        KEY_APOSTROPHE => Apostrophe,
        KEY_GRAVE => Grave,
        KEY_LEFTSHIFT => ShiftLeft,
        KEY_BACKSLASH => Backslash,
        KEY_Z => Z,
        KEY_X => X,
        KEY_C => C,
        KEY_V => V,
        KEY_B => B,
        KEY_N => N,
        KEY_M => M,
        KEY_COMMA => Comma,
        KEY_DOT => Period,
        KEY_SLASH => Slash,
        KEY_RIGHTSHIFT => ShiftRight,
        KEY_LEFTALT => AltLeft,
        KEY_SPACE => Space,
        KEY_CAPSLOCK => Capital,
        KEY_F1 => F1,
        KEY_F2 => F2,
        KEY_F3 => F3,
        KEY_F4 => F4,
        KEY_F5 => F5,
        KEY_F6 => F6,
        KEY_F7 => F7,
        KEY_F8 => F8,
        KEY_F9 => F9,
        KEY_F10 => F10,
        KEY_NUMLOCK => Numlock,
        KEY_SCROLLLOCK => Scroll,
        KEY_KP7 => Numpad7,
        KEY_KP8 => Numpad8,
        KEY_KP9 => Numpad9,
        KEY_KPMINUS => NumpadSubtract,
        KEY_KP4 => Numpad4,
        KEY_KP5 => Numpad5,
        KEY_KP6 => Numpad6,
        KEY_KPPLUS => NumpadAdd,
        KEY_KP1 => Numpad1,
        KEY_KP2 => Numpad2,
        KEY_KP3 => Numpad3,
        KEY_KP0 => Numpad0,
        KEY_KPDOT => NumpadDecimal,
        // KEY_ZENKAKUHANKAKU => Key1,
        // KEY_102ND => Key1,
        KEY_F11 => F11,
        KEY_F12 => F12,
        // KEY_RO => Key1,
        // KEY_KATAKANA => Key1,
        // KEY_HIRAGANA => Key1,
        // KEY_HENKAN => Key1,
        // KEY_KATAKANAHIRAGANA => Key1,
        // KEY_MUHENKAN => Key1,
        KEY_KPJPCOMMA => NumpadComma,
        KEY_KPENTER => NumpadEnter,
        KEY_RIGHTCTRL => ControlRight,
        KEY_KPSLASH => NumpadDivide,
        KEY_SYSRQ => Sysrq,
        KEY_RIGHTALT => AltRight,
        // KEY_LINEFEED => Key1,
        KEY_HOME => Home,
        KEY_UP => Up,
        KEY_PAGEUP => PageUp,
        KEY_LEFT => Left,
        KEY_RIGHT => Right,
        KEY_END => End,
        KEY_DOWN => Down,
        KEY_PAGEDOWN => PageDown,
        KEY_INSERT => Insert,
        KEY_DELETE => Delete,
        // KEY_MACRO => Key1,
        KEY_MUTE => Mute,
        KEY_VOLUMEDOWN => VolumeDown,
        KEY_VOLUMEUP => VolumeUp,
        KEY_POWER => Power,
        KEY_KPEQUAL => NumpadEquals,
        KEY_KPPLUSMINUS => NumpadSubtract,
        KEY_PAUSE => Pause,
        // KEY_SCALE => Key1, /* AL Compiz Scale (Expose) */
        KEY_KPCOMMA => NumpadComma,
        // KEY_HANGEUL => Key1,
        // KEY_HANGUEL => Key1,
        // KEY_HANJA => Key1,
        KEY_YEN => Yen,
        KEY_LEFTMETA => SuperLeft,
        KEY_RIGHTMETA => SuperRight,
        KEY_COMPOSE => Compose,
        KEY_STOP => Stop,
        // KEY_AGAIN => Key1,
        // KEY_PROPS => Key1, /* AC Properties */
        // KEY_UNDO => Key1,  /* AC Undo */
        // KEY_FRONT => WebForward,
        KEY_COPY => Copy, /* AC Copy */
        // KEY_OPEN => Open,  /* AC Open */
        KEY_PASTE => Paste,    /* AC Paste */
        KEY_FIND => WebSearch, /* AC Search */
        KEY_CUT => Cut,        /* AC Cut */
        // KEY_HELP => Key1,  /* AL Integrated Help Center */
        // KEY_MENU => Key1,  /* Menu (show menu) */
        KEY_CALC => Calculator, /* AL Calculator */
        // KEY_SETUP => Key1,
        KEY_SLEEP => Sleep, /* SC System Sleep */
        KEY_WAKEUP => Wake, /* System Wake Up */
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
        KEY_MAIL => Mail,
        KEY_BOOKMARKS => WebFavorites, /* AC Bookmarks */
        KEY_COMPUTER => MyComputer,
        KEY_BACK => WebBack,       /* AC Back */
        KEY_FORWARD => WebForward, /* AC Forward */
        // KEY_CLOSECD => Key1,
        // KEY_EJECTCD => Key1,
        // KEY_EJECTCLOSECD => Key1,
        KEY_NEXTSONG => NextTrack,
        KEY_PLAYPAUSE => PlayPause,
        KEY_PREVIOUSSONG => PrevTrack,
        KEY_STOPCD => MediaStop,
        // KEY_RECORD => Key1,
        // KEY_REWIND => Key1,
        KEY_PHONE => MediaSelect, /* Media Select Telephone */
        // KEY_ISO => Key1,
        // KEY_CONFIG => Key1, /* AL Consumer Control Configuration */
        KEY_HOMEPAGE => WebHome,   /* AC Home */
        KEY_REFRESH => WebRefresh, /* AC Refresh */
        // KEY_EXIT => Key1,   /* AC Exit */
        // KEY_MOVE => Key1,
        // KEY_EDIT => Key1,
        // KEY_SCROLLUP => Key1,
        // KEY_SCROLLDOWN => Key1,
        // KEY_KPLEFTPAREN => Key1,
        // KEY_KPRIGHTPAREN => Key1,
        // KEY_NEW => Key1,  /* AC New */
        // KEY_REDO => Key1, /* AC Redo/Repeat */
        KEY_F13 => F13,
        KEY_F14 => F14,
        KEY_F15 => F15,
        KEY_F16 => F16,
        KEY_F17 => F17,
        KEY_F18 => F18,
        KEY_F19 => F19,
        KEY_F20 => F20,
        KEY_F21 => F21,
        KEY_F22 => F22,
        KEY_F23 => F23,
        KEY_F24 => F24,
        KEY_PRINT => Snapshot, /* AC Print */
        KEY_EMAIL => Mail,

        o => {
            warn!("unknown key code: {o:X}");
            return None;
        }
    };

    let shift = input_state.any_pressed([ShiftLeft, ShiftRight]);

    if shift {
        keycode = match keycode {
            Grave | Key1 => return None,
            Key2 => At,
            Key3 | Key4 | Key5 | Key6 | Key7 => return None,
            Key8 => Asterisk,
            Key9 | Key0 | Minus => return None,
            Equals => Plus,
            BracketLeft | BracketRight | Backslash => return None,
            Semicolon => Colon,
            Apostrophe => return None,
            Comma | Period | Slash => return None,
            o => o,
        };
    }

    if !lock_state.number_lock {
        keycode = match keycode {
            Numpad0 => Insert,
            NumpadDecimal => Delete,
            Numpad1 => End,
            Numpad2 => Down,
            Numpad3 => PageDown,
            Numpad4 => Left,
            Numpad5 => return None,
            Numpad6 => Right,
            Numpad7 => Home,
            Numpad8 => Up,
            Numpad9 => PageUp,
            o => o,
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

    Some(keycode)
}
