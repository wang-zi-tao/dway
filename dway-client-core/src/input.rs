use super::desktop::{CursorOnOutput, FocusedWindow};
use crate::{desktop::CursorOnWindow, DWayClientSystem};
use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::*,
};
use bevy_relationship::{graph_query, ControlFlow};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    input::seat::SeatHasKeyboard,
    input::{
        grab::{SurfaceGrabKind, WlSurfacePointerState},
        keyboard::XkbState,
        seat::{SeatHasPointer, WlSeat},
    },
    input::{keyboard::WlKeyboard, pointer::WlPointer},
    schedule::DWayServerSet,
    wl::surface::ClientHasSurface,
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, DWayWindow},
};
use dway_server::{input::grab::ResizeEdges, macros::WindowAction, util::rect::IRect};

#[derive(Default)]
pub struct DWayInputPlugin {
    pub debug: bool,
}
impl Plugin for DWayInputPlugin {
    fn build(&self, app: &mut App) {
        // app.add_system(print_pick_events.label(WindowLabel::Input));
        use DWayClientSystem::*;
        app.add_systems(
            PreUpdate,
            (
                mouse_move_on_window.run_if(on_event::<CursorMoved>()),
                keyboard_input_system.run_if(on_event::<KeyboardInput>()),
                on_input_event,
            )
                .in_set(DWayServerSet::Input),
        );
        app.register_type::<SurfaceUiNode>();
    }
}

graph_query!(KeyboardInputGraph=>[
    surface=(&'static WlSurface, &'static GlobalGeometry, Option<&'static XdgToplevel>, Option<&'static XdgPopup>),
    client=Entity,
    keyboard=&'static mut WlKeyboard,
]=>{
    path=surface<-[ClientHasSurface]-client-[SeatHasKeyboard]->keyboard
});
// #[tracing::instrument(skip_all)]
pub fn keyboard_input_system(
    mut graph: KeyboardInputGraph,
    mut keyboard_evens: EventReader<KeyboardInput>,
    output_focus: Res<FocusedWindow>,
    mut keystate: NonSendMut<XkbState>,
) {
    if keyboard_evens.is_empty() {
        return;
    }
    for event in keyboard_evens.read() {
        keystate.key(event);
        if let Some(window) = output_focus.window_entity {
            graph.for_each_path_mut_from::<()>(
                window,
                |(surface, _rect, _toplevel, popup), _, keyboard| {
                    if popup.is_none() {
                        keyboard.key(surface, event, keystate.serialize());
                    }
                    ControlFlow::Continue
                },
            );
        }
        // TODO: keyboard on popup
    }
}

pub fn mouse_move_on_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut focus: ResMut<CursorOnOutput>,
) {
    for event in cursor_moved_events.read() {
        focus.0 = Some((event.window, event.position.as_ivec2()));
    }
}

#[derive(Component, Debug, Reflect)]
pub struct SurfaceUiNode {
    pub surface_entity: Entity,
    pub widget: Entity,
    pub grab: bool,
}

impl SurfaceUiNode {
    pub fn new(surface_entity: Entity, widget: Entity) -> Self {
        Self {
            surface_entity,
            widget,
            grab: false,
        }
    }

    pub fn with_grab(mut self, grab: bool) -> Self {
        self.grab = grab;
        self
    }
}

enum MouseEvent<'l> {
    Move(&'l CursorMoved),
    Button(&'l MouseButtonInput),
    Wheel(&'l MouseWheel),
}

graph_query!(InputGraph=>[
    surface=< (Entity, &'static WlSurface,&'static mut WlSurfacePointerState, &'static mut Geometry, &'static GlobalGeometry, Option<&'static XdgPopup>),With<DWayWindow>>,
    client=&'static mut WlSeat,
    pointer=&'static mut WlPointer,
]=>{
    pointer=surface<-[ClientHasSurface]-client-[SeatHasPointer]->pointer
});

pub fn on_input_event(
    mut graph: InputGraph,
    mut ui_query: Query<(&Interaction, &mut SurfaceUiNode, &mut Style)>,
    window_root_ui_query: Query<(&Node, &GlobalTransform)>,
    cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    mut cursor_on_window: ResMut<CursorOnWindow>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut button_events: EventReader<MouseButtonInput>,
    mut wheel_events: EventReader<MouseWheel>,
    mut window_action: EventWriter<WindowAction>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        return;
    };
    for event in cursor_moved_events
        .read()
        .map(MouseEvent::Move)
        .chain(button_events.read().map(MouseEvent::Button))
        .chain(wheel_events.read().map(MouseEvent::Wheel))
    {
        for (interaction, content, mut style) in ui_query.iter_mut() {
            if *interaction == Interaction::None {
                continue;
            }
            let Ok((content_node, content_geo)) = window_root_ui_query.get(content.widget) else {
                error!("cannot get window widget");
                continue;
            };
            let content_rect =
                Rect::from_center_size(content_geo.translation().xy(), content_node.size());
            graph.for_each_pointer_mut_from::<()>(
                content.surface_entity,
                |(
                    surface_entity,
                    surface,
                    window_pointer,
                    window_geometry,
                    window_global_geometry,
                    popup,
                ),
                 ref mut seat,
                 pointer| {
                    let relative_pos =
                        pos.as_vec2() - content_rect.min - surface.image_rect().pos().as_vec2();
                    match event {
                        MouseEvent::Move(e) => {
                            let relative_pos = e.position
                                - content_rect.min
                                - surface.image_rect().pos().as_vec2();
                            if window_pointer.enabled() {
                                pointer.move_cursor(seat, surface, relative_pos);
                                window_pointer.mouse_pos = relative_pos.as_ivec2();
                            }
                            cursor_on_window.0 = Some((*surface_entity, relative_pos.as_ivec2()));
                            if let Some(grab) = &window_pointer.grab {
                                match &**grab {
                                    SurfaceGrabKind::Move { mouse_pos, .. } => {
                                        window_action.send(WindowAction::SetRect(
                                            *surface_entity,
                                            IRect::from_pos_size(
                                                e.position.as_ivec2()
                                                    - *mouse_pos
                                                    - surface.image_rect().pos()
                                                    - (window_geometry.pos()
                                                        - window_global_geometry.pos()),
                                                window_geometry.size(),
                                            ),
                                        ));
                                    }
                                    SurfaceGrabKind::Resizing { edges, .. } => {
                                        let mut geo = window_geometry.geometry;
                                        if edges.contains(ResizeEdges::LEFT) {
                                            geo.min.x += relative_pos.x as i32;
                                            geo.max.x = geo.max.x;
                                        }
                                        if edges.contains(ResizeEdges::TOP) {
                                            geo.min.y += relative_pos.y as i32;
                                            geo.max.y = geo.max.y;
                                        }
                                        if edges.contains(ResizeEdges::RIGHT) {
                                            geo.max.x = geo.min.x + relative_pos.x as i32;
                                            geo.min.x = geo.min.x;
                                        }
                                        if edges.contains(ResizeEdges::BUTTOM) {
                                            geo.max.y = geo.min.y + relative_pos.y as i32;
                                            geo.min.y = geo.min.y;
                                        }
                                        window_action
                                            .send(WindowAction::SetRect(*surface_entity, geo));
                                    }
                                }
                            }
                        }
                        MouseEvent::Button(e) => {
                            if window_pointer.enabled() {
                                pointer.button(seat, e, surface, relative_pos.as_dvec2());
                                if !content_rect.contains(pos.as_vec2()) {
                                    if let Some(popup) = popup {
                                        popup.raw.popup_done();
                                    }
                                }
                            }
                            window_pointer.is_clicked = content_rect.contains(pos.as_vec2())
                                && e.state == ButtonState::Pressed;
                            let distant = if content.grab || e.state == ButtonState::Pressed {
                                16384.0
                            } else {
                                4.0
                            };
                            *style = Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(-distant),
                                top: Val::Px(-distant),
                                right: Val::Px(-distant),
                                bottom: Val::Px(-distant),
                                ..default()
                            };
                            if e.state == ButtonState::Released {
                                window_pointer.grab = None;
                            }
                            output_focus.window_entity = Some(*surface_entity);
                        }
                        MouseEvent::Wheel(e) => {
                            let acc = |x: f64| x * 20.0;
                            if window_pointer.enabled() {
                                pointer.asix(
                                    seat,
                                    DVec2::new(-acc(e.x as f64), -acc(e.y as f64)),
                                    surface,
                                    relative_pos.as_dvec2(),
                                );
                            }
                            output_focus.window_entity = Some(*surface_entity);
                        }
                    }
                    ControlFlow::Continue
                },
            );
        }
    }
}
