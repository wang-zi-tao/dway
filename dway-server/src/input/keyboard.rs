use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::{path::PathBuf, sync::Arc, time::SystemTime};

use bevy::input::keyboard::KeyboardInput;
use failure::{format_err, Fallible};
use xkbcommon::xkb;

use crate::{prelude::*, util::serial::next_serial, wl::surface::WlSurface};

#[derive(Resource, Reflect, Default)]
pub struct Keymap {
    pub rate: i32,
    pub delay: i32,
    pub rules: String,
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: Option<String>,
}

#[derive(Component)]
pub struct WlKeyboard {
    pub raw: wl_keyboard::WlKeyboard,
    pub focus: Option<wl_surface::WlSurface>,
}

impl WlKeyboard {
    pub fn new(kbd: wl_keyboard::WlKeyboard, keymap: &Keymap) -> Fallible<Self> {
        let xkb_config = xkbcommon::xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &xkb_config,
            &keymap.rules,
            &keymap.model,
            &keymap.layout,
            &keymap.variant,
            keymap.options.clone(),
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or_else(|| format_err!("failed to encode keymap"))?;
        let keymap_string = keymap.get_as_string(xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1);

        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        let mut file = tempfile::tempfile_in(dir)?;
        file.write_all(keymap_string.as_bytes())?;
        file.flush()?;

        kbd.keymap(
            wl_keyboard::KeymapFormat::XkbV1,
            file.as_raw_fd(),
            keymap_string.bytes().len().try_into().unwrap(),
        );
        if kbd.version() >= 4 {
            kbd.repeat_info(25, 200);
        }
        Ok(Self {
            raw: kbd,
            focus: None,
        })
    }
    pub fn set_focus(&mut self, surface: &WlSurface) {
        if let Some(focus) = &self.focus {
            if &surface.raw != focus {
                self.raw.leave(next_serial(), &focus);
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
    pub fn key(&self, surface: &WlSurface, input: &KeyboardInput) {
        trace!(surface=?surface.raw.id(),"key evnet : {input:?}");
        self.raw.key(
            next_serial(),
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            input.scan_code,
            match input.state {
                bevy::input::ButtonState::Pressed => wl_keyboard::KeyState::Pressed,
                bevy::input::ButtonState::Released => wl_keyboard::KeyState::Released,
            },
        );
    }
}

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
        client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_keyboard::WlKeyboard,
        request: <wayland_server::protocol::wl_keyboard::WlKeyboard as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_keyboard::Request::Release => state.destroy_object::<WlKeyboard>(resource),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

pub struct WlKeyboardPlugin;
impl Plugin for WlKeyboardPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Keymap>();
        app.insert_resource(Keymap::default());
    }
}
