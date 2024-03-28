use crate::{input::time, prelude::*, util::serial::next_serial, wl::surface::WlSurface};
use bevy::input::keyboard::{KeyboardInput, NativeKeyCode};
use dway_util::keys::*;
use std::{fs::File, io::Write, os::fd::AsFd, path::PathBuf};
use xkbcommon::xkb::{self};

fn get_key_code(key: &KeyCode) -> u32 {
    // TODO: check all unwupported key
    match key {
        KeyCode::Unidentified(n) => match n {
            NativeKeyCode::Unidentified => KEY_RESERVED,
            NativeKeyCode::Android(s) => *s,
            NativeKeyCode::MacOS(s) => *s as u32,
            NativeKeyCode::Windows(s) => *s as u32,
            NativeKeyCode::Xkb(s) => *s,
        },
        KeyCode::Backquote => KEY_GRAVE,
        KeyCode::Backslash => KEY_BACKSLASH,
        KeyCode::BracketLeft => KEY_LEFTBRACE,
        KeyCode::BracketRight => KEY_RIGHTBRACE,
        KeyCode::Comma => KEY_COMMA,
        KeyCode::Digit0 => KEY_0,
        KeyCode::Digit1 => KEY_1,
        KeyCode::Digit2 => KEY_2,
        KeyCode::Digit3 => KEY_3,
        KeyCode::Digit4 => KEY_4,
        KeyCode::Digit5 => KEY_5,
        KeyCode::Digit6 => KEY_6,
        KeyCode::Digit7 => KEY_7,
        KeyCode::Digit8 => KEY_8,
        KeyCode::Digit9 => KEY_9,
        KeyCode::Equal => KEY_EQUAL,
        KeyCode::IntlBackslash => KEY_BACKSLASH,
        KeyCode::IntlRo => KEY_RESERVED,
        KeyCode::IntlYen => KEY_YEN,
        KeyCode::KeyA => KEY_A,
        KeyCode::KeyB => KEY_B,
        KeyCode::KeyC => KEY_C,
        KeyCode::KeyD => KEY_D,
        KeyCode::KeyE => KEY_E,
        KeyCode::KeyF => KEY_F,
        KeyCode::KeyG => KEY_G,
        KeyCode::KeyH => KEY_H,
        KeyCode::KeyI => KEY_I,
        KeyCode::KeyJ => KEY_J,
        KeyCode::KeyK => KEY_K,
        KeyCode::KeyL => KEY_L,
        KeyCode::KeyM => KEY_M,
        KeyCode::KeyN => KEY_N,
        KeyCode::KeyO => KEY_O,
        KeyCode::KeyP => KEY_P,
        KeyCode::KeyQ => KEY_Q,
        KeyCode::KeyR => KEY_R,
        KeyCode::KeyS => KEY_S,
        KeyCode::KeyT => KEY_T,
        KeyCode::KeyU => KEY_U,
        KeyCode::KeyV => KEY_V,
        KeyCode::KeyW => KEY_W,
        KeyCode::KeyX => KEY_X,
        KeyCode::KeyY => KEY_Y,
        KeyCode::KeyZ => KEY_Z,
        KeyCode::Minus => KEY_MINUS,
        KeyCode::Period => KEY_DOT,
        KeyCode::Quote => KEY_APOSTROPHE,
        KeyCode::Semicolon => KEY_SEMICOLON,
        KeyCode::Slash => KEY_SLASH,
        KeyCode::AltLeft => KEY_LEFTALT,
        KeyCode::AltRight => KEY_RIGHTALT,
        KeyCode::Backspace => KEY_BACKSPACE,
        KeyCode::CapsLock => KEY_CAPSLOCK,
        KeyCode::ContextMenu => KEY_MENU,
        KeyCode::ControlLeft => KEY_LEFTCTRL,
        KeyCode::ControlRight => KEY_RIGHTCTRL,
        KeyCode::Enter => KEY_ENTER,
        KeyCode::SuperLeft => KEY_LEFTMETA,
        KeyCode::SuperRight => KEY_RIGHTMETA,
        KeyCode::ShiftLeft => KEY_LEFTSHIFT,
        KeyCode::ShiftRight => KEY_RIGHTSHIFT,
        KeyCode::Space => KEY_SPACE,
        KeyCode::Tab => KEY_TAB,
        KeyCode::Convert => KEY_RESERVED,
        KeyCode::KanaMode => KEY_RESERVED,
        KeyCode::Lang1 => KEY_RESERVED,
        KeyCode::Lang2 => KEY_RESERVED,
        KeyCode::Lang3 => KEY_RESERVED,
        KeyCode::Lang4 => KEY_RESERVED,
        KeyCode::Lang5 => KEY_RESERVED,
        KeyCode::NonConvert => KEY_RESERVED,
        KeyCode::Delete => KEY_DELETE,
        KeyCode::End => KEY_END,
        KeyCode::Help => KEY_HELP,
        KeyCode::Home => KEY_HOME,
        KeyCode::Insert => KEY_INSERT,
        KeyCode::PageDown => KEY_PAGEDOWN,
        KeyCode::PageUp => KEY_PAGEUP,
        KeyCode::ArrowDown => KEY_DOWN,
        KeyCode::ArrowLeft => KEY_LEFT,
        KeyCode::ArrowRight => KEY_RIGHT,
        KeyCode::ArrowUp => KEY_UP,
        KeyCode::NumLock => KEY_NUMLOCK,
        KeyCode::Numpad0 => KEY_KP0,
        KeyCode::Numpad1 => KEY_KP1,
        KeyCode::Numpad2 => KEY_KP2,
        KeyCode::Numpad3 => KEY_KP3,
        KeyCode::Numpad4 => KEY_KP4,
        KeyCode::Numpad5 => KEY_KP5,
        KeyCode::Numpad6 => KEY_KP6,
        KeyCode::Numpad7 => KEY_KP7,
        KeyCode::Numpad8 => KEY_KP8,
        KeyCode::Numpad9 => KEY_KP9,
        KeyCode::NumpadAdd => KEY_KPPLUS,
        KeyCode::NumpadBackspace => KEY_RESERVED,
        KeyCode::NumpadClear => KEY_RESERVED,
        KeyCode::NumpadClearEntry => KEY_RESERVED,
        KeyCode::NumpadComma => KEY_KPCOMMA,
        KeyCode::NumpadDecimal => KEY_KPDOT,
        KeyCode::NumpadDivide => KEY_KPSLASH,
        KeyCode::NumpadEnter => KEY_KPENTER,
        KeyCode::NumpadEqual => KEY_KPEQUAL,
        KeyCode::NumpadHash => KEY_RESERVED,
        KeyCode::NumpadMemoryAdd => KEY_RESERVED,
        KeyCode::NumpadMemoryClear => KEY_RESERVED,
        KeyCode::NumpadMemoryRecall => KEY_RESERVED,
        KeyCode::NumpadMemoryStore => KEY_RESERVED,
        KeyCode::NumpadMemorySubtract => KEY_RESERVED,
        KeyCode::NumpadMultiply => KEY_RESERVED,
        KeyCode::NumpadParenLeft => KEY_RESERVED,
        KeyCode::NumpadParenRight => KEY_RESERVED,
        KeyCode::NumpadStar => KEY_RESERVED,
        KeyCode::NumpadSubtract => KEY_KPMINUS,
        KeyCode::Escape => KEY_ESC,
        KeyCode::Fn => KEY_FN,
        KeyCode::FnLock => KEY_RESERVED,
        KeyCode::PrintScreen => KEY_PRINT,
        KeyCode::ScrollLock => KEY_SCROLLLOCK,
        KeyCode::Pause => KEY_PAUSE,
        KeyCode::BrowserBack => KEY_BACK,
        KeyCode::BrowserFavorites => KEY_RESERVED,
        KeyCode::BrowserForward => KEY_FORWARD,
        KeyCode::BrowserHome => KEY_HOMEPAGE,
        KeyCode::BrowserRefresh => KEY_REFRESH,
        KeyCode::BrowserSearch => KEY_FIND,
        KeyCode::BrowserStop => KEY_RESERVED,
        KeyCode::Eject => KEY_RESERVED,
        KeyCode::LaunchApp1 => KEY_RESERVED,
        KeyCode::LaunchApp2 => KEY_RESERVED,
        KeyCode::LaunchMail => KEY_MAIL,
        KeyCode::MediaPlayPause => KEY_PLAYPAUSE,
        KeyCode::MediaSelect => KEY_RESERVED,
        KeyCode::MediaStop => KEY_STOP,
        KeyCode::MediaTrackNext => KEY_NEXTSONG,
        KeyCode::MediaTrackPrevious => KEY_PREVIOUSSONG,
        KeyCode::Power => KEY_POWER,
        KeyCode::Sleep => KEY_SLEEP,
        KeyCode::AudioVolumeDown => KEY_VOLUMEDOWN,
        KeyCode::AudioVolumeMute => KEY_MUTE,
        KeyCode::AudioVolumeUp => KEY_VOLUMEUP,
        KeyCode::WakeUp => KEY_WAKEUP,
        KeyCode::Meta => KEY_RESERVED,
        KeyCode::Hyper => KEY_RESERVED,
        KeyCode::Turbo => KEY_RESERVED,
        KeyCode::Abort => KEY_RESERVED,
        KeyCode::Resume => KEY_RESERVED,
        KeyCode::Suspend => KEY_RESERVED,
        KeyCode::Again => KEY_RESERVED,
        KeyCode::Copy => KEY_COPY,
        KeyCode::Cut => KEY_CUT,
        KeyCode::Find => KEY_RESERVED,
        KeyCode::Open => KEY_OPEN,
        KeyCode::Paste => KEY_PASTE,
        KeyCode::Props => KEY_PROPS,
        KeyCode::Select => KEY_RESERVED,
        KeyCode::Undo => KEY_UNDO,
        KeyCode::Hiragana => KEY_RESERVED,
        KeyCode::Katakana => KEY_RESERVED,
        KeyCode::F1 => KEY_F1,
        KeyCode::F2 => KEY_F2,
        KeyCode::F3 => KEY_F3,
        KeyCode::F4 => KEY_F4,
        KeyCode::F5 => KEY_F5,
        KeyCode::F6 => KEY_F6,
        KeyCode::F7 => KEY_F7,
        KeyCode::F8 => KEY_F8,
        KeyCode::F9 => KEY_F9,
        KeyCode::F10 => KEY_F10,
        KeyCode::F11 => KEY_F11,
        KeyCode::F12 => KEY_F12,
        KeyCode::F13 => KEY_F13,
        KeyCode::F14 => KEY_F14,
        KeyCode::F15 => KEY_F15,
        KeyCode::F16 => KEY_F16,
        KeyCode::F17 => KEY_F17,
        KeyCode::F18 => KEY_F18,
        KeyCode::F19 => KEY_F19,
        KeyCode::F20 => KEY_F20,
        KeyCode::F21 => KEY_F21,
        KeyCode::F22 => KEY_F22,
        KeyCode::F23 => KEY_F23,
        KeyCode::F24 => KEY_F24,
        KeyCode::F25 => KEY_RESERVED,
        KeyCode::F26 => KEY_RESERVED,
        KeyCode::F27 => KEY_RESERVED,
        KeyCode::F28 => KEY_RESERVED,
        KeyCode::F29 => KEY_RESERVED,
        KeyCode::F30 => KEY_RESERVED,
        KeyCode::F31 => KEY_RESERVED,
        KeyCode::F32 => KEY_RESERVED,
        KeyCode::F33 => KEY_RESERVED,
        KeyCode::F34 => KEY_RESERVED,
        KeyCode::F35 => KEY_RESERVED,
    }
}

#[derive(Resource, Reflect)]
pub struct Keymap {
    pub rate: i32,
    pub delay: i32,
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: Option<String>,
}
impl Default for Keymap {
    fn default() -> Self {
        Self {
            rate: 25,
            delay: 200,
            rules: "evdev".to_string(),
            model: "pc104".to_string(),
            layout: "us".to_string(),
            variant: Default::default(),
            options: Default::default(),
        }
    }
}

pub struct XkbState {
    pub state: xkb::State,
    pub file: File,
    pub keymap_string: String,
}
impl XkbState {
    pub fn new(config: &Keymap) -> Result<Self> {
        let context = xkbcommon::xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &context,
            &config.rules,
            &config.model,
            &config.layout,
            &config.variant,
            config.options.clone(),
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or_else(|| anyhow!("failed to encode keymap"))?;
        let keymap_string = keymap.get_as_string(xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1);

        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        let mut file = tempfile::tempfile_in(dir)?;
        file.write_all(keymap_string.as_bytes())?;
        file.flush()?;

        Ok(Self {
            state: xkb::State::new(&keymap),
            file,
            keymap_string,
        })
    }

    pub fn key(&mut self, input: &KeyboardInput) {
        self.state.update_key(
            get_key_code(&input.key_code) + 8,
            match input.state {
                bevy::input::ButtonState::Pressed => xkb::KeyDirection::Down,
                bevy::input::ButtonState::Released => xkb::KeyDirection::Up,
            },
        );
    }

    pub fn serialize(&self) -> [u32; 4] {
        let depressed = self.state.serialize_mods(xkb::STATE_MODS_DEPRESSED);
        let latched = self.state.serialize_mods(xkb::STATE_MODS_LATCHED);
        let locked = self.state.serialize_mods(xkb::STATE_MODS_LOCKED);
        let layout_effective = self.state.serialize_layout(xkb::STATE_LAYOUT_EFFECTIVE);
        [depressed, latched, locked, layout_effective]
    }
}

#[derive(Component)]
pub struct WlKeyboard {
    pub raw: wl_keyboard::WlKeyboard,
    pub focus: Option<wl_surface::WlSurface>,
}
#[derive(Bundle)]
pub struct WlKeyboardBundle {
    resource: WlKeyboard,
}

impl WlKeyboardBundle {
    pub fn new(resource: WlKeyboard) -> Self {
        Self { resource }
    }
}

impl WlKeyboard {
    pub fn new(kbd: wl_keyboard::WlKeyboard, keymap: &Keymap, keystate: &XkbState) -> Result<Self> {
        kbd.keymap(
            wl_keyboard::KeymapFormat::XkbV1,
            keystate.file.as_fd(),
            keystate.keymap_string.bytes().len().try_into().unwrap(),
        );
        if kbd.version() >= 4 {
            kbd.repeat_info(keymap.rate, keymap.delay);
        }

        Ok(Self {
            raw: kbd,
            focus: None,
        })
    }

    pub fn set_focus(&mut self, surface: &WlSurface) {
        if let Some(focus) = &self.focus {
            if &surface.raw != focus {
                if focus.is_alive() {
                    self.raw.leave(next_serial(), focus);
                }
                trace!("{} leave {}", self.raw.id(), focus.id());
                self.raw.enter(next_serial(), &surface.raw, Vec::new());
                trace!("{} enter {}", self.raw.id(), surface.raw.id());
                self.focus = Some(surface.raw.clone());
            }
        } else {
            self.raw.enter(next_serial(), &surface.raw, Vec::new());
            self.focus = Some(surface.raw.clone());
            trace!("{} enter {}", self.raw.id(), surface.raw.id());
        }
    }

    pub fn key(&mut self, surface: &WlSurface, input: &KeyboardInput, serialize: [u32; 4]) {
        trace!(surface=?surface.raw.id(),"key evnet : {input:?}");
        let serial = next_serial();
        self.set_focus(surface);
        self.raw.key(
            serial,
            time(),
            get_key_code(&input.key_code),
            match input.state {
                bevy::input::ButtonState::Pressed => wl_keyboard::KeyState::Pressed,
                bevy::input::ButtonState::Released => wl_keyboard::KeyState::Released,
            },
        );

        self.raw.modifiers(
            serial,
            serialize[0],
            serialize[1],
            serialize[2],
            serialize[3],
        );
    }
}

pub fn update_keymap() {} // TODO

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_keyboard::WlKeyboard: Entity] => SeatDelegate);

impl
    wayland_server::Dispatch<
        wayland_server::protocol::wl_keyboard::WlKeyboard,
        bevy::prelude::Entity,
        DWay,
    > for SeatDelegate
{
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_keyboard::WlKeyboard,
        request: <wayland_server::protocol::wl_keyboard::WlKeyboard as WlResource>::Request,
        _data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_keyboard::Request::Release => state.destroy_object(resource),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_keyboard::WlKeyboard,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

pub struct WlKeyboardPlugin;
impl Plugin for WlKeyboardPlugin {
    fn build(&self, app: &mut App) {
        let keymap = Keymap::default();
        app.insert_non_send_resource(XkbState::new(&keymap).unwrap());
        app.insert_resource(keymap);
        app.register_type::<Keymap>();
        app.add_systems(PostUpdate, update_keymap.in_set(UpdateKeymap));
    }
}
