use std::any::type_name;
use std::fmt::Write;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use std::borrow::Cow;

use bevy::ecs::query::{QueryEntityError, QueryItem, ROQueryItem, ReadOnlyWorldQuery, WorldQuery};
use bevy::prelude::*;
use failure::{format_err, Error};
use smithay::desktop::PopupKind;
use smithay::output::Output;
use smithay::reexports::wayland_server::backend::ObjectId;
use smithay::reexports::wayland_server::Resource;
use smithay::utils::Physical;
use smithay::wayland::shell::xdg::{PopupSurface, PositionerState, ToplevelSurface};
use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_popup::XdgPopup,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    xwayland::X11Surface,
};
use std::ops::Deref;
use uuid::Uuid;

// use crate::wayland::shell::WindowElement;
use dway_protocol::window::WindowState;
use smithay::{
    backend::{
        drm::{DrmDevice, DrmDeviceFd, DrmNode},
        renderer::{
            damage::DamageTrackedRenderer,
            element::texture::TextureBuffer,
            gles2::Gles2Renderer,
            multigpu::{egl::EglGlesBackend, GpuManager, MultiTexture},
        },
        session::libseat::LibSeatSession,
    },
    reexports::{
        calloop::{Dispatcher, RegistrationToken},
        drm::control::crtc,
        gbm,
        wayland_server::{
            backend::{smallvec::SmallVec, GlobalId},
            DisplayHandle,
        },
    },
    utils::{Logical, Rectangle, Scale},
    wayland::{
        compositor::SurfaceData,
        dmabuf::{DmabufGlobal, DmabufState},
    },
};

pub struct Id(Uuid);

#[derive(Component, Reflect, Clone, Hash, PartialEq, Eq)]
pub enum SurfaceId {
    Wayland(u32),
    X11(u32),
}

impl std::fmt::Debug for SurfaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wayland(arg0) => {
                f.write_str("Wl@")?;
                arg0.fmt(f)?;
                Ok(())
            }
            Self::X11(arg0) => {
                f.write_str("X@")?;
                arg0.fmt(f)?;
                Ok(())
            }
        }
    }
}

impl ToString for SurfaceId {
    fn to_string(&self) -> String {
        match self {
            Self::Wayland(arg0) => format!("Wl@{arg0}"),
            Self::X11(arg0) => format!("X@{arg0}"),
        }
    }
}
impl From<&WlSurfaceWrapper> for SurfaceId {
    fn from(value: &WlSurfaceWrapper) -> Self {
        Self::from(&value.0)
    }
}
impl From<WlSurface> for SurfaceId {
    fn from(value: WlSurface) -> Self {
        Self::from(&value)
    }
}
impl From<&WlSurface> for SurfaceId {
    fn from(value: &WlSurface) -> Self {
        Self::Wayland(value.id().to_string()[11..].parse().unwrap())
    }
}
impl From<X11Surface> for SurfaceId {
    fn from(value: X11Surface) -> Self {
        Self::X11(value.window_id())
    }
}
impl From<&X11Surface> for SurfaceId {
    fn from(value: &X11Surface) -> Self {
        Self::X11(value.window_id())
    }
}
impl From<&PopupSurface> for SurfaceId {
    fn from(value: &PopupSurface) -> Self {
        Self::from(value.wl_surface())
    }
}
impl From<PopupSurface> for SurfaceId {
    fn from(value: PopupSurface) -> Self {
        Self::from(value.wl_surface())
    }
}
impl From<ToplevelSurface> for SurfaceId {
    fn from(value: ToplevelSurface) -> Self {
        Self::from(value.wl_surface())
    }
}
impl From<&ToplevelSurface> for SurfaceId {
    fn from(value: &ToplevelSurface) -> Self {
        Self::from(value.wl_surface())
    }
}

#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct WindowIndex(pub HashMap<SurfaceId, Entity>);
impl WindowIndex {
    pub fn get(&self, surface: &SurfaceId) -> Option<&Entity> {
        if let Some(o) = self.0.get(surface) {
            Some(o)
        } else {
            error!(?surface, "surface entity not found");
            None
        }
    }
    pub fn query<'w, Q: WorldQuery, F: ReadOnlyWorldQuery>(
        &self,
        surface: &SurfaceId,
        query: &'w Query<Q, F>,
    ) -> Option<ROQueryItem<'w, Q>> {
        self.0
            .get(surface)
            .cloned()
            .or_else(|| {
                error!(
                    ?surface,
                    query = type_name::<Query<Q, F>>(),
                    "window index not found: {surface:?}"
                );
                None
            })
            .and_then(|e| {
                query
                    .get(e)
                    .map_err(|error| {
                        error!(entity=?e,surface=?surface,query=type_name::<Query<Q, F>>(),?error);
                    })
                    .ok()
            })
    }
    pub fn query_mut<'w, Q: WorldQuery, F: ReadOnlyWorldQuery>(
        &self,
        surface: &SurfaceId,
        query: &'w mut Query<Q, F>,
    ) -> Option<QueryItem<'w, Q>> {
        self.0
            .get(surface)
            .or_else(|| {
                error!(surface=?surface,query=type_name::<Query<Q, F>>(),"window index not found");
                None
            })
            .and_then(|&e| {
                query
                    .get_mut(e)
                    .map_err(|error| {
                        error!(entity=?e,surface=?surface,query=type_name::<Query<Q, F>>(),?error);
                    })
                    .ok()
            })
    }
}

#[derive(Component, Debug, Default)]
pub struct WindowMark;

#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct WindowZIndex(pub i32);

#[derive(Component, Debug, Deref, DerefMut)]
pub struct WaylandWindow(pub Window);

#[derive(Component, Debug, Deref, DerefMut)]
pub struct X11Window(pub X11Surface);

#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct WlSurfaceWrapper(pub WlSurface);
impl WlSurfaceWrapper {
    pub fn id(&self) -> SurfaceId {
        self.into()
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deref, DerefMut)]
pub struct UUID(pub Uuid);
impl UUID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Resource, Debug, Deref, DerefMut)]
pub struct UuidIndex(pub HashMap<Uuid, Entity>);

#[derive(Resource, Debug, Deref, DerefMut)]
pub struct NameIndex(pub HashMap<String, Entity>);

#[derive(Component, Debug, Deref, DerefMut)]
pub struct XdgPopupWrapper(pub XdgPopup);

#[derive(Component, Debug, Clone, Copy, Deref, DerefMut)]
pub struct WindowScale(pub Scale<f64>);

impl Default for WindowScale {
    fn default() -> Self {
        Self(Scale { x: 1.0, y: 1.0 })
    }
}
#[derive(Component, Debug, Default, Clone, Copy, Deref, DerefMut)]
pub struct NormalModeGlobalRect(pub Rectangle<i32, Physical>);

#[derive(Component, Debug, Default, Clone, Copy, Deref, DerefMut)]
pub struct GlobalPhysicalRect(pub Rectangle<i32, Physical>);

impl GlobalPhysicalRect {
    pub fn width(&self) -> u32 {
        self.0.size.w as u32
    }

    pub fn height(&self) -> u32 {
        self.0.size.h as u32
    }

    pub fn size_vec2(&self) -> Vec2 {
        Vec2::new(self.0.size.w as f32, self.0.size.h as f32)
    }
    pub fn to_rect(&self) -> Rect {
        Rect::new(
            self.0.loc.x as f32,
            self.0.loc.y as f32,
            (self.0.loc.x + self.0.size.w) as f32,
            (self.0.loc.y + self.0.size.h) as f32,
        )
    }
}

#[derive(Component, Debug, Default, Clone, Copy, Deref, DerefMut)]
pub struct PhysicalRect(pub Rectangle<i32, Physical>);
impl PhysicalRect {
    pub fn width(&self) -> u32 {
        self.0.size.w as u32
    }

    pub fn height(&self) -> u32 {
        self.0.size.h as u32
    }

    pub fn size_vec2(&self) -> Vec2 {
        Vec2::new(self.0.size.w as f32, self.0.size.h as f32)
    }
}
#[derive(Component, Debug, Default, Deref, DerefMut)]
pub struct LogicalRect(pub Rectangle<i32, Logical>);
impl LogicalRect {
    pub fn width(&self) -> u32 {
        self.0.size.w as u32
    }

    pub fn height(&self) -> u32 {
        self.0.size.h as u32
    }

    pub fn size_vec2(&self) -> Vec2 {
        Vec2::new(self.0.size.w as f32, self.0.size.h as f32)
    }
}

#[derive(Component, Debug)]
pub struct PopupWindow {
    pub kind: PopupKind,
    pub position: PositionerState,
}
impl PopupWindow {
    pub fn update_with_rect(
        &mut self,
        positioner: PositionerState,
        logical_rect: &mut LogicalRect,
        physical_rect: &mut PhysicalRect,
        scale: Option<&WindowScale>,
    ) {
        self.position = positioner;
        let geo = positioner.get_geometry();
        logical_rect.0 = geo;
        physical_rect.0 = geo.to_physical_precise_round(scale.cloned().unwrap_or_default().0);
    }
}

#[derive(Component, Debug, Clone, Deref, DerefMut)]
pub struct OutputWrapper(pub Output);

#[derive(Component, Debug, Clone, Copy, Default, Deref, DerefMut)]
pub struct SurfaceOffset(pub Rectangle<i32, Physical>);
