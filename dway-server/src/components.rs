use std::{cell::RefCell, collections::HashMap, rc::Rc};

use std::borrow::Cow;

use bevy::prelude::*;
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

#[derive(Component, Debug, Clone, Hash, PartialEq, Eq)]
pub enum SurfaceId {
    Wayland(ObjectId),
    X11(u32),
}
impl From<&WlSurfaceWrapper> for SurfaceId {
    fn from(value: &WlSurfaceWrapper) -> Self {
        Self::Wayland(value.0.id())
    }
}
impl From<WlSurface> for SurfaceId {
    fn from(value: WlSurface) -> Self {
        Self::Wayland(value.id())
    }
}
impl From<&WlSurface> for SurfaceId {
    fn from(value: &WlSurface) -> Self {
        Self::Wayland(value.id())
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
        Self::Wayland(value.wl_surface().id())
    }
}
impl From<PopupSurface> for SurfaceId {
    fn from(value: PopupSurface) -> Self {
        Self::Wayland(value.wl_surface().id())
    }
}
impl From<ToplevelSurface> for SurfaceId {
    fn from(value: ToplevelSurface) -> Self {
        Self::Wayland(value.wl_surface().id())
    }
}
impl From<&ToplevelSurface> for SurfaceId {
    fn from(value: &ToplevelSurface) -> Self {
        Self::Wayland(value.wl_surface().id())
    }
}

#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub struct WindowIndex(pub HashMap<SurfaceId, Entity>);

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
impl WlSurfaceWrapper {}

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
