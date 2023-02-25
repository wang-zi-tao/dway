use std::{collections::HashMap, cell::RefCell, rc::Rc};

use std::{ borrow::Cow};

use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::backend::ObjectId;
use smithay::utils::Physical;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::{reexports::{
    wayland_protocols::xdg::shell::server::xdg_popup::XdgPopup,
    wayland_server::protocol::wl_surface::WlSurface,
}, desktop::Window, xwayland::X11Surface};
use uuid::Uuid;

// use crate::wayland::shell::WindowElement;
use bevy_ecs::prelude::*;
use dway_protocol::window::WindowState;
use smithay::{
    backend::{drm::{DrmNode, DrmDevice, DrmDeviceFd}, renderer::{damage::DamageTrackedRenderer, multigpu::{GpuManager, egl::EglGlesBackend, MultiTexture}, gles2::Gles2Renderer, element::texture::TextureBuffer}, session::libseat::LibSeatSession},
    reexports::{wayland_server::{
        backend::{smallvec::SmallVec, GlobalId},
        DisplayHandle,
    }, calloop::{Dispatcher, RegistrationToken}, gbm, drm::control::crtc},
    utils::{Logical, Rectangle, Scale}, wayland::{dmabuf::{DmabufState, DmabufGlobal}, compositor::SurfaceData},
};

// use super::{backend::{},  cursor::{PointerElement, Cursor}, CalloopData};

// pub struct Element(pub WindowElement);
pub struct Id(Uuid);
pub struct Geometry(pub Rectangle<i32, Logical>);
pub struct ElementScale(pub Scale<i32>);
pub struct ElementCommit();
pub struct BBox(pub Scale<i32>);

pub struct ElementRemoteDisplay();
pub struct ElementState(pub WindowState);
pub struct ElementMouseGrab();
pub struct ElementKeyGrab();
pub struct ElementFullScreen;
pub struct ElementVisibility(pub bool);
pub struct PopupList {
    popups: SmallVec<[Entity; 1]>,
}
pub struct Popup {
    parent: Entity,
}

pub struct Screen {}
pub struct Gpu {}
// struct UDevSurface {
//     dh: DisplayHandle,
//     device_id: DrmNode,
//     render_node: DrmNode,
//     surface: RenderSurface,
//     global: Option<GlobalId>,
// }
// struct BackendData {
//     surfaces: Rc<RefCell<HashMap<crtc::Handle, Rc<RefCell<SurfaceData>>>>>,
//     gbm: gbm::Device<DrmDeviceFd>,
//     registration_token: RegistrationToken,
//     event_dispatcher: Dispatcher<'static, DrmDevice, CalloopData>,
// }
// pub struct UDevSession {
//     pub session: LibSeatSession,
//     dh: DisplayHandle,
//     dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
//     primary_gpu: DrmNode,
//     gpus: GpuManager<EglGlesBackend<Gles2Renderer>>,
//     backends: HashMap<DrmNode, BackendData>,
//     pointer_images: Vec<(xcursor::parser::Image, TextureBuffer<MultiTexture>)>,
//     pointer_element: PointerElement<MultiTexture>,
//     pointer_image: Cursor,
//     logger: slog::Logger,
// }
pub struct RenderState {
    damage_tracked_renderer: DamageTrackedRenderer,
}
pub struct ConnectionId(pub Uuid);
pub struct ConnectionIds(pub SmallVec<[ConnectionId; 1]>);
pub struct Connection();

#[derive(Component,Clone,Hash,PartialEq, Eq, Debug)]
pub enum WindowId{
    Wayland(ObjectId),
    X11(u32),
}
impl From<&WlSurface>for WindowId{
    fn from(value: &WlSurface) -> Self {
        Self::Wayland(value.id())
    }
}
impl From<&X11Surface>for WindowId{
    fn from(value: &X11Surface) -> Self {
        Self::X11(value.window_id())
    }
}
impl From<&ToplevelSurface>for WindowId{
    fn from(value: &ToplevelSurface) -> Self {
        Self::Wayland(value.wl_surface().id())
    }
}

#[derive(Resource)]
pub struct WindowIndex(pub HashMap<WindowId,Entity>);

#[derive(Component)]
pub struct WindowMark;

#[derive(Component)]
pub struct WaylandWindow(pub Window);

#[derive(Component)]
pub struct X11Window(pub X11Surface);

#[derive(Component)]
pub struct WlSurfaceWrapper(pub WlSurface);

#[derive(Component)]
pub struct UUID(pub Uuid);
impl UUID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Resource)]
pub struct UuidIndex(pub HashMap<Uuid,Entity>);

#[derive(Resource)]
pub struct NameIndex(pub HashMap<String,Entity>);

#[derive(Component)]
pub struct XdgPopupWrapper(pub XdgPopup);

#[derive(Component,Default)]
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
