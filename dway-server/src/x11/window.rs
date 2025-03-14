use std::sync::Arc;

use bevy::utils::HashSet;
use encoding::{types::DecoderTrap, Encoding};
use scopeguard::defer;
use x11rb::{
    connection::Connection,
    properties::{WmClass, WmHints, WmSizeHints},
    protocol::xproto::{Atom, AtomEnum, ConfigureWindowAux, ConnectionExt, PropMode},
    rust_connection::{ConnectionError, RustConnection},
    wrapper::ConnectionExt as RustConnectionExt,
};

use super::{atoms::Atoms, UnixStreamWrapper, XWaylandDisplay};
use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    util::rect::IRect,
};

const MWM_HINTS_FLAGS_FIELD: usize = 0;
const MWM_HINTS_DECORATIONS_FIELD: usize = 2;
const MWM_HINTS_DECORATIONS: u32 = 1 << 1;

#[derive(Component, Reflect)]
pub struct MappedXWindow;

#[derive(Bundle)]
pub struct XWindowBundle {
    pub xwindow: XWindow,
    pub geometry: Geometry,
    pub global_geometry: GlobalGeometry,
}

#[derive(Clone, Debug, Component, Reflect)]
pub struct XWindow {
    #[reflect(ignore, default = "unimplemented")]
    pub connection: Arc<(RustConnection<UnixStreamWrapper>, Atoms)>,
    pub window: x11rb::protocol::xproto::Window,
    pub parent_window: Option<x11rb::protocol::xproto::Window>,
    pub override_redirect: bool,
    pub title: Option<String>,
    pub class: Option<String>,
    pub instance: Option<String>,
    #[reflect(ignore)]
    pub hints: Option<WmHints>,
    #[reflect(ignore)]
    pub normal_hints: Option<WmSizeHints>,
    pub transient_for: Option<Entity>,
    pub net_state: HashSet<Atom>,
    pub motif_hints: Vec<u32>,
    pub window_type: Vec<Atom>,
    pub surface_id: Option<u32>,
    pub boarder_width: u32,
    pub is_toplevel: bool,
    pub surface_entity: Option<Entity>,
}
relationship!(XWindowAttachSurface=>XWindowSurfaceRef--XWindowRef);

impl XWindow {
    pub fn new(
        connection: Arc<(RustConnection<UnixStreamWrapper>, Atoms)>,
        window: x11rb::protocol::xproto::Window,
        parent_window: Option<x11rb::protocol::xproto::Window>,
        override_redirect: bool,
        is_toplevel: bool,
    ) -> Self {
        Self {
            connection,
            window,
            parent_window,
            override_redirect,
            title: None,
            class: None,
            instance: None,
            hints: None,
            normal_hints: None,
            transient_for: None,
            net_state: Default::default(),
            motif_hints: Vec::new(),
            window_type: Vec::new(),
            surface_id: None,
            boarder_width: 0,
            is_toplevel,
            surface_entity: None,
        }
    }

    pub fn atoms(&self) -> &Atoms {
        &self.connection.1
    }

    pub fn xwayland_connection(&self) -> &RustConnection<UnixStreamWrapper> {
        &self.connection.0
    }

    fn connection(&self) -> (&RustConnection<UnixStreamWrapper>, &Atoms) {
        (self.xwayland_connection(), self.atoms())
    }

    pub fn update_property(
        &mut self,
        x: &XWaylandDisplay,
        atom: Option<Atom>,
    ) -> Result<(), ConnectionError> {
        let atoms = self.atoms();
        match atom {
            Some(atom)
                if atom == atoms._NET_WM_NAME || atom == u8::from(AtomEnum::WM_NAME) as u32 =>
            {
                self.update_title()
            }
            Some(atom) if atom == u8::from(AtomEnum::WM_CLASS) as u32 => self.update_class(),
            Some(atom) if atom == atoms.WM_PROTOCOLS => self.update_protocols(),
            Some(atom) if atom == atoms.WM_HINTS => self.update_hints(),
            Some(atom) if atom == u8::from(AtomEnum::WM_NORMAL_HINTS) as u32 => {
                self.update_normal_hints()
            }
            Some(atom) if atom == u8::from(AtomEnum::WM_TRANSIENT_FOR) as u32 => {
                self.update_transient_for(x)
            }
            Some(atom) if atom == atoms._NET_WM_WINDOW_TYPE => self.update_net_window_type(),
            Some(atom) if atom == atoms._MOTIF_WM_HINTS => self.update_motif_hints(),
            Some(atom) => {
                debug!("ignore unknown atom: {atom}");
                Ok(())
            } // unknown
            None => {
                self.update_title()?;
                self.update_class()?;
                self.update_protocols()?;
                self.update_hints()?;
                self.update_normal_hints()?;
                self.update_transient_for(x)?;
                self.update_net_window_type()?;
                self.update_motif_hints()?;
                Ok(())
            }
        }
    }

    fn update_class(&mut self) -> Result<(), ConnectionError> {
        let conn = self.xwayland_connection();
        let (class, instance) = match WmClass::get(conn, self.window)?.reply_unchecked() {
            Ok(Some(wm_class)) => (
                encoding::all::ISO_8859_1
                    .decode(wm_class.class(), DecoderTrap::Replace)
                    .ok()
                    .unwrap_or_default(),
                encoding::all::ISO_8859_1
                    .decode(wm_class.instance(), DecoderTrap::Replace)
                    .ok()
                    .unwrap_or_default(),
            ),
            Ok(None) | Err(ConnectionError::ParseError(_)) => {
                (Default::default(), Default::default())
            } // Getting the property failed
            Err(err) => return Err(err),
        };

        debug!(window=%self.window,"set class to {:?}", class);
        debug!(window=%self.window,"set instance to {:?}", instance);
        self.class = Some(class);
        self.instance = Some(instance);

        Ok(())
    }

    fn update_hints(&mut self) -> Result<(), ConnectionError> {
        self.hints = match WmHints::get(&self.connection.0, self.window)?.reply_unchecked() {
            Ok(hints) => hints,
            Err(ConnectionError::ParseError(_)) => None,
            Err(err) => return Err(err),
        };
        debug!(window=%self.window,"set hint to {:?}", self.hints);
        Ok(())
    }

    fn update_normal_hints(&mut self) -> Result<(), ConnectionError> {
        self.normal_hints = match WmSizeHints::get_normal_hints(&self.connection.0, self.window)?
            .reply_unchecked()
        {
            Ok(hints) => hints,
            Err(ConnectionError::ParseError(_)) => None,
            Err(err) => return Err(err),
        };
        debug!(window=%self.window,"set normal hints to {:?}", self.normal_hints);
        Ok(())
    }

    fn update_motif_hints(&mut self) -> Result<(), ConnectionError> {
        let Some(hints) = (match self
            .connection
            .0
            .get_property(
                false,
                self.window,
                self.connection.1._MOTIF_WM_HINTS,
                AtomEnum::ANY,
                0,
                2048,
            )?
            .reply_unchecked()
        {
            Ok(Some(reply)) => reply.value32().map(|vals| vals.collect::<Vec<_>>()),
            Ok(None) | Err(ConnectionError::ParseError(_)) => return Ok(()),
            Err(err) => return Err(err),
        }) else {
            return Ok(());
        };

        if hints.len() < 5 {
            return Ok(());
        }

        self.motif_hints = hints;
        debug!(window=%self.window,"set motif hints to {:?}", self.motif_hints);
        Ok(())
    }

    fn update_protocols(&mut self) -> Result<(), ConnectionError> {
        let (conn, atoms) = self.connection();
        let Some(protocols) = (match conn
            .get_property(
                false,
                self.window,
                atoms.WM_PROTOCOLS,
                AtomEnum::ATOM,
                0,
                2048,
            )?
            .reply_unchecked()
        {
            Ok(Some(reply)) => reply.value32().map(|vals| vals.collect::<Vec<_>>()),
            Ok(None) | Err(ConnectionError::ParseError(_)) => return Ok(()),
            Err(err) => return Err(err),
        }) else {
            return Ok(());
        };
        dbg!(protocols);

        // self.protocols = protocols
        //     .into_iter()
        //     .filter_map(|atom| match atom {
        //         x if x == atoms.WM_TAKE_FOCUS => Some(WMProtocol::TakeFocus),
        //         x if x == atoms.WM_DELETE_WINDOW => Some(WMProtocol::DeleteWindow),
        //         _ => None,
        //     })
        //     .collect::<Vec<_>>();
        Ok(())
    }

    fn update_transient_for(&mut self, x: &XWaylandDisplay) -> Result<(), ConnectionError> {
        let conn = self.xwayland_connection();
        let reply = match conn
            .get_property(
                false,
                self.window,
                AtomEnum::WM_TRANSIENT_FOR,
                AtomEnum::WINDOW,
                0,
                2048,
            )?
            .reply_unchecked()
        {
            Ok(Some(reply)) => reply,
            Ok(None) | Err(ConnectionError::ParseError(_)) => return Ok(()),
            Err(err) => return Err(err),
        };
        let window = reply
            .value32()
            .and_then(|mut iter| iter.next())
            .filter(|w| *w != 0);

        self.transient_for = window.and_then(|window| x.find_window(window).ok());
        debug!(window=%self.window,"transient for {:?}", self.transient_for);
        Ok(())
    }

    pub fn read_window_property_string(
        &mut self,
        atom: impl Into<Atom>,
    ) -> Result<Option<String>, ConnectionError> {
        let (conn, atoms) = self.connection();
        let reply = match conn
            .get_property(false, self.window, atom, AtomEnum::ANY, 0, 2048)?
            .reply_unchecked()
        {
            Ok(Some(reply)) => reply,
            Ok(None) | Err(ConnectionError::ParseError(_)) => return Ok(None),
            Err(err) => return Err(err),
        };
        let Some(bytes) = reply.value8() else {
            return Ok(None);
        };
        let bytes = bytes.collect::<Vec<u8>>();

        match reply.type_ {
            x if x == u8::from(AtomEnum::STRING) as u32 => Ok(encoding::all::ISO_8859_1
                .decode(&bytes, DecoderTrap::Replace)
                .ok()),
            x if x == atoms.UTF8_STRING => Ok(String::from_utf8(bytes).ok()),
            _ => Ok(None),
        }
    }

    fn update_title(&mut self) -> Result<(), ConnectionError> {
        let title = self
            .read_window_property_string(self.atoms()._NET_WM_NAME)?
            .or(self.read_window_property_string(AtomEnum::WM_NAME)?)
            .unwrap_or_default();
        debug!(window=%self.window,"set title to {:?}", title);
        self.title = Some(title);
        Ok(())
    }

    fn update_net_window_type(&mut self) -> Result<(), ConnectionError> {
        let (conn, atoms) = self.connection();
        let atoms = match conn
            .get_property(
                false,
                self.window,
                atoms._NET_WM_WINDOW_TYPE,
                AtomEnum::ATOM,
                0,
                1024,
            )?
            .reply_unchecked()
        {
            Ok(atoms) => atoms,
            Err(ConnectionError::ParseError(_)) => return Ok(()),
            Err(err) => return Err(err),
        };

        self.window_type = atoms
            .and_then(|atoms| Some(atoms.value32()?.collect::<Vec<_>>()))
            .unwrap_or_default();
        debug!(window=%self.window,"set window type to {:?}", self.window_type);
        Ok(())
    }

    pub fn resize(&mut self, rect: IRect) -> Result<()> {
        self.set_rect(rect)
    }

    pub fn set_rect(&mut self, rect: IRect) -> Result<()> {
        let conn = self.xwayland_connection();
        let aux = ConfigureWindowAux::default()
            .x(rect.x())
            .y(rect.y())
            .width(Some(rect.width() as u32))
            .height(Some(rect.height() as u32));
        conn.configure_window(self.window, &aux)?;
        conn.flush()?;
        Ok(())
    }

    pub fn change_net_state(&mut self, atom: Atom, is_add: bool) -> Result<(), ConnectionError> {
        let mut changed = false;

        if is_add {
            changed |= self.net_state.insert(atom);
        } else {
            changed |= self.net_state.remove(&atom);
        }

        if changed {
            let new_props = Vec::from_iter(self.net_state.iter().copied());

            let (conn, atoms) = self.connection();

            conn.grab_server()?;
            defer! {
                let _ = conn.ungrab_server();
                let _ = conn.flush();
            }

            conn.change_property32(
                PropMode::REPLACE,
                self.window,
                atoms._NET_WM_STATE,
                AtomEnum::ATOM,
                &new_props,
            )?;
        }

        Ok(())
    }

    pub fn close(&mut self) -> Result<()> {
        let conn = self.xwayland_connection();
        conn.destroy_window(self.window)?;
        conn.flush()?;
        Ok(())
    }

    pub fn is_decorated(&self) -> bool {
        if self
            .motif_hints
            .get(MWM_HINTS_FLAGS_FIELD)
            .map(|v| v & MWM_HINTS_DECORATIONS != 0)
            .unwrap_or_default()
        {
            return self.motif_hints.get(MWM_HINTS_DECORATIONS_FIELD) == Some(&0);
        }
        false
    }
}
