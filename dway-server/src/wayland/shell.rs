use std::{
    time::Duration, cell::RefCell,
};


use smithay::{

    wayland::{compositor::SurfaceData as WlSurfaceData, seat::WaylandFocus},
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
        utils::{send_frames_surface_tree, with_surfaces_surface_tree, OutputPresentationFeedback, take_presentation_feedback_surface_tree},
        Space, Window,
    },
    input::{keyboard::KeyboardTarget, pointer::PointerTarget},
    output::Output,
    reexports::{wayland_server::{protocol::wl_surface::WlSurface, Resource}, wayland_protocols::wp::presentation_time::server::wp_presentation_feedback},
    render_elements,
    utils::{user_data::UserDataMap, IsAlive, Logical, Point, Rectangle, Serial, Size},
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

    pub fn take_presentation_feedback<F1, F2>(
        &self,
        output_feedback: &mut OutputPresentationFeedback,
        primary_scan_out_output: F1,
        presentation_feedback_flags: F2,
    ) where
        F1: FnMut(&WlSurface, &WlSurfaceData) -> Option<Output> + Copy,
        F2: FnMut(&WlSurface, &WlSurfaceData) -> wp_presentation_feedback::Kind + Copy,
    {
        match self {
            WindowElement::Wayland(w) => w.take_presentation_feedback(
                output_feedback,
                primary_scan_out_output,
                presentation_feedback_flags,
            ),
            WindowElement::X11(w) => {
                if let Some(surface) = w.wl_surface() {
                    take_presentation_feedback_surface_tree(
                        &surface,
                        output_feedback,
                        primary_scan_out_output,
                        presentation_feedback_flags,
                    );
                }
            }
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
    pos:Point<i32,Logical>,
    activate: bool,
)  {
    let output = space.outputs().next().cloned();
    let _output_geometry = output
        .and_then(|o| {
            let geo = space.output_geometry(&o)?;
            let map = layer_map_for_output(&o);
            let zone = map.non_exclusive_zone();
            Some(Rectangle::from_loc_and_size(geo.loc + zone.loc, zone.size))
        })
        .unwrap_or_else(|| Rectangle::from_loc_and_size((0, 0), (800, 800)));

    space.map_element(window.clone(), (pos.x, pos.y), activate);
    
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

#[derive(Default)]
pub struct FullscreenSurface(RefCell<Option<WindowElement>>);

impl FullscreenSurface {
    pub fn set(&self, window: WindowElement) {
        *self.0.borrow_mut() = Some(window);
    }

    pub fn get(&self) -> Option<WindowElement> {
        self.0.borrow().clone()
    }

    pub fn clear(&self) -> Option<WindowElement> {
        self.0.borrow_mut().take()
    }
}

pub fn fixup_positions(space: &mut Space<WindowElement>) {
    // fixup outputs
    let mut offset = Point::<i32, Logical>::from((0, 0));
    for output in space.outputs().cloned().collect::<Vec<_>>().into_iter() {
        let size = space
            .output_geometry(&output)
            .map(|geo| geo.size)
            .unwrap_or_else(|| Size::from((0, 0)));
        space.map_output(&output, offset);
        layer_map_for_output(&output).arrange();
        offset.x += size.w;
    }

    // fixup windows
    let mut orphaned_windows = Vec::new();
    let outputs = space
        .outputs()
        .flat_map(|o| {
            let geo = space.output_geometry(o)?;
            let map = layer_map_for_output(o);
            let zone = map.non_exclusive_zone();
            Some(Rectangle::from_loc_and_size(geo.loc + zone.loc, zone.size))
        })
        .collect::<Vec<_>>();
    for window in space.elements() {
        let window_location = match space.element_location(window) {
            Some(loc) => loc,
            None => continue,
        };
        let geo_loc = window.bbox().loc + window_location;

        if !outputs.iter().any(|o_geo| o_geo.contains(geo_loc)) {
            orphaned_windows.push(window.clone());
        }
    }
    for window in orphaned_windows.into_iter() {
        place_new_window(space, &window,(0,0).into(), false);
    }
}
