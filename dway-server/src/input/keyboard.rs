use crate::{input::time, prelude::*, util::serial::next_serial, wl::surface::WlSurface};
use bevy::input::keyboard::KeyboardInput;
use std::{fs::File, io::Write, os::fd::AsFd, path::PathBuf};
use xkbcommon::xkb;

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
            input.scan_code + 8,
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
            input.scan_code,
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
