use std::{
    time::Duration,
};


use smithay::{
    backend::renderer::{
        element::{
            memory::MemoryRenderBufferRenderElement, surface::WaylandSurfaceRenderElement,
            AsRenderElements, 
        },
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{
        layer_map_for_output,
        space::SpaceElement,
        utils::{send_frames_surface_tree, with_surfaces_surface_tree},
        Space, Window,
    },
    input::{keyboard::KeyboardTarget, pointer::PointerTarget},
    output::Output,
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    render_elements,
    utils::{user_data::UserDataMap, IsAlive, Logical, Point, Rectangle, Serial, Size},
    wayland::seat::WaylandFocus,
    xwayland::{
        xwm::ResizeEdge,
        X11Surface,
    },
};

use super::DWayState;

render_elements!(
    pub WindowRenderElement<R> where R: ImportAll + ImportMem;
    Window=WaylandSurfaceRenderElement<R>,
    Decoration=MemoryRenderBufferRenderElement<R>,
);

#[derive(Clone, Debug, PartialEq)]
pub enum WindowElement {
    Wayland(Window),
    X11(X11Surface),
}

impl WindowElement {
    pub fn id(&self) -> String {
        match self {
            WindowElement::Wayland(w) => w
                .wl_surface()
                .map_or_else(|| "unknown".to_string(), |s| format!("{:?}", s.id())),
            WindowElement::X11(w) => format!("X11({})", w.window_id()),
        }
    }

    pub fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            WindowElement::Wayland(w) => w.wl_surface(),
            WindowElement::X11(w) => w.wl_surface(),
        }
    }

    pub fn with_surfaces<F>(&self, processor: F)
    where
        F: FnMut(&WlSurface, &smithay::wayland::compositor::SurfaceData) + Copy,
    {
        match self {
            WindowElement::Wayland(w) => w.with_surfaces(processor),
            WindowElement::X11(w) => {
                if let Some(surface) = w.wl_surface() {
                    with_surfaces_surface_tree(&surface, processor);
                }
            }
        }
    }

    pub fn send_frame<T, F>(
        &self,
        output: &Output,
        time: T,
        throttle: Option<Duration>,
        primary_scan_out_output: F,
    ) where
        T: Into<Duration>,
        F: FnMut(&WlSurface, &smithay::wayland::compositor::SurfaceData) -> Option<Output> + Copy,
    {
        match self {
            WindowElement::Wayland(w) => {
                w.send_frame(output, time, throttle, primary_scan_out_output)
            }
            WindowElement::X11(w) => {
                if let Some(surface) = w.wl_surface() {
                    send_frames_surface_tree(
                        &surface,
                        output,
                        time,
                        throttle,
                        primary_scan_out_output,
                    );
                }
            }
        }
    }

    pub fn user_data(&self) -> &UserDataMap {
        match self {
            WindowElement::Wayland(w) => w.user_data(),
            WindowElement::X11(w) => w.user_data(),
        }
    }
}
impl SpaceElement for WindowElement {
    fn geometry(&self) -> Rectangle<i32, Logical> {
        
        match self {
            WindowElement::Wayland(w) => w.geometry(),
            WindowElement::X11(w) => SpaceElement::geometry(w),
        }
    }
    fn bbox(&self) -> Rectangle<i32, Logical> {
        
        match self {
            WindowElement::Wayland(w) => SpaceElement::bbox(w),
            WindowElement::X11(w) => SpaceElement::bbox(w),
        }
    }
    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        match self {
            WindowElement::Wayland(w) => SpaceElement::is_in_input_region(w, point),
            WindowElement::X11(w) => SpaceElement::is_in_input_region(w, point),
        }
    }
    fn z_index(&self) -> u8 {
        match self {
            WindowElement::Wayland(w) => SpaceElement::z_index(w),
            WindowElement::X11(w) => SpaceElement::z_index(w),
        }
    }

    fn set_activate(&self, activated: bool) {
        match self {
            WindowElement::Wayland(w) => SpaceElement::set_activate(w, activated),
            WindowElement::X11(w) => SpaceElement::set_activate(w, activated),
        }
    }
    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        match self {
            WindowElement::Wayland(w) => SpaceElement::output_enter(w, output, overlap),
            WindowElement::X11(w) => SpaceElement::output_enter(w, output, overlap),
        }
    }
    fn output_leave(&self, output: &Output) {
        match self {
            WindowElement::Wayland(w) => SpaceElement::output_leave(w, output),
            WindowElement::X11(w) => SpaceElement::output_leave(w, output),
        }
    }
    fn refresh(&self) {
        match self {
            WindowElement::Wayland(w) => SpaceElement::refresh(w),
            WindowElement::X11(w) => SpaceElement::refresh(w),
        }
    }
}
impl<R> AsRenderElements<R> for WindowElement
where
    R: Renderer + ImportAll + ImportMem,
    <R as Renderer>::TextureId: Texture + 'static,
{
    type RenderElement = WindowRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, smithay::utils::Physical>,
        scale: smithay::utils::Scale<f64>,
    ) -> Vec<C> {
        match self {
            WindowElement::Wayland(w) => AsRenderElements::<R>::render_elements::<
                WindowRenderElement<R>,
            >(w, renderer, location, scale),
            WindowElement::X11(w) => {
                AsRenderElements::<R>::render_elements::<WindowRenderElement<R>>(
                    w, renderer, location, scale,
                )
            }
        }
        .into_iter()
        .map(C::from)
        .collect()
    }
}
impl IsAlive for WindowElement {
    fn alive(&self) -> bool {
        match self {
            WindowElement::Wayland(w) => w.alive(),
            WindowElement::X11(w) => w.alive(),
        }
    }
}
impl KeyboardTarget<DWayState> for WindowElement {
    fn enter(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        keys: Vec<smithay::input::keyboard::KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            WindowElement::Wayland(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
            WindowElement::X11(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
        }
    }

    fn leave(&self, seat: &smithay::input::Seat<DWayState>, data: &mut DWayState, serial: Serial) {
        match self {
            WindowElement::Wayland(w) => KeyboardTarget::leave(w, seat, data, serial),
            WindowElement::X11(w) => KeyboardTarget::leave(w, seat, data, serial),
        }
    }

    fn key(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        key: smithay::input::keyboard::KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self {
            WindowElement::Wayland(w) => {
                KeyboardTarget::key(w, seat, data, key, state, serial, time)
            }
            WindowElement::X11(w) => KeyboardTarget::key(w, seat, data, key, state, serial, time),
        }
    }

    fn modifiers(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        modifiers: smithay::input::keyboard::ModifiersState,
        serial: Serial,
    ) {
        match self {
            WindowElement::Wayland(w) => {
                KeyboardTarget::modifiers(w, seat, data, modifiers, serial)
            }
            WindowElement::X11(w) => KeyboardTarget::modifiers(w, seat, data, modifiers, serial),
        }
    }
}
impl PointerTarget<DWayState> for WindowElement {
    fn enter(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => PointerTarget::enter(w, seat, data, event),
            WindowElement::X11(w) => PointerTarget::enter(w, seat, data, event),
        }
    }

    fn motion(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.motion(seat, data, event),
            WindowElement::X11(w) => w.motion(seat, data, event),
        }
    }

    fn button(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => w.button(seat, data, event),
            WindowElement::X11(w) => w.button(seat, data, event),
        }
    }

    fn axis(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        frame: smithay::input::pointer::AxisFrame,
    ) {
        match self {
            WindowElement::Wayland(w) => w.axis(seat, data, frame),
            WindowElement::X11(w) => w.axis(seat, data, frame),
        }
    }

    fn leave(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        serial: Serial,
        time: u32,
    ) {
        match self {
            WindowElement::Wayland(w) => PointerTarget::leave(w, seat, data, serial, time),
            WindowElement::X11(w) => PointerTarget::leave(w, seat, data, serial, time),
        }
    }

    fn relative_motion(
        &self,
        seat: &smithay::input::Seat<DWayState>,
        data: &mut DWayState,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        match self {
            WindowElement::Wayland(w) => PointerTarget::relative_motion(w, seat, data, event),
            WindowElement::X11(w) => PointerTarget::relative_motion(w, seat, data, event),
        }
    }
}

pub fn place_new_window(
    space: &mut Space<WindowElement>,
    window: &WindowElement,
    activate: bool,
) -> Rectangle<i32, Logical> {
    let output = space.outputs().next().cloned();
    let _output_geometry = output
        .and_then(|o| {
            let geo = space.output_geometry(&o)?;
            let map = layer_map_for_output(&o);
            let zone = map.non_exclusive_zone();
            Some(Rectangle::from_loc_and_size(geo.loc + zone.loc, zone.size))
        })
        .unwrap_or_else(|| Rectangle::from_loc_and_size((0, 0), (800, 800)));
    // let x= output_geometry.loc.x + output_geometry.size.w/2;
    // let y= output_geometry.loc.y + output_geometry.size.h/2;
    let x = 0;
    let y = 75;

    space.map_element(window.clone(), (x, y), activate);
    
    Rectangle::from_loc_and_size((x, y), (800, 600))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ResizeData {
    /// The edges the surface is being resized with.
    pub edges: ResizeEdge,
    /// The initial window location.
    pub initial_window_location: Point<i32, Logical>,
    /// The initial window size (geometry width and height).
    pub initial_window_size: Size<i32, Logical>,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ResizeState {
    /// The surface is not being resized.
    NotResizing,
    /// The surface is currently being resized.
    Resizing(ResizeData),
    /// The resize has finished, and the surface needs to ack the final configure.
    WaitingForFinalAck(ResizeData, Serial),
    /// The resize has finished, and the surface needs to commit its final state.
    WaitingForCommit(ResizeData),
}
impl Default for ResizeState {
    fn default() -> Self {
        Self::NotResizing
    }
}
