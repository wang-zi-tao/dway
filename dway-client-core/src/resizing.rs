use std::time::SystemTime;

use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    utils::tracing,
    winit::WinitWindows,
};
use dway_protocol::window::{WindowMessage, WindowMessageKind};
use dway_server::{
    events::ResizeWindow,
    geometry::GlobalGeometry,
    wl::surface::WlSurface,
    xdg::{toplevel::XdgToplevel, XdgSurface},
};

use crate::{
    desktop::{CursorOnOutput, FocusedWindow},
    protocol::WindowMessageSender,
    window::Backend,
    DWayClientState,
};

#[derive(Resource, Default)]
pub struct ResizingMethod {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Default)]
pub struct DWayResizingPlugin {}
impl Plugin for DWayResizingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ResizingMethod::default());
        app.add_system(
            resize_window
                .run_if(in_state(DWayClientState::Resizing).and_then(on_event::<CursorMoved>()))
                .in_set(OnUpdate(DWayClientState::Resizing)),
        );
        app.add_system(
            stop_resizing
                .run_if(
                    in_state(DWayClientState::Resizing).and_then(on_event::<MouseButtonInput>()),
                )
                .in_set(OnUpdate(DWayClientState::Resizing)),
        );
    }
}
// #[tracing::instrument(skip_all)]
pub fn resize_window(
    focused_window: Res<FocusedWindow>,
    mut cursor_move_events: EventReader<CursorMoved>,
    mut window_query: Query<(&mut Backend, &mut Style)>,
    mut surface_query: Query<(&mut XdgToplevel, &GlobalGeometry), With<WlSurface>>,
    physical_windows: NonSend<WinitWindows>,
    resize_method: Res<ResizingMethod>,
    mut output_focus: ResMut<CursorOnOutput>,
    mut events: EventWriter<ResizeWindow>,
) {
    let Some(focus_window)=&focused_window.0 else{
        return;
    };
    let Ok((  backend,style  ))=window_query.get_mut(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    let Ok((mut toplevel,geometry))=surface_query.get_mut(backend.get())else{
        error!("window backend entity {focus_window:?} not found");
        return;
    };
    let mut rect = geometry.geometry;
    for event in cursor_move_events.iter() {
        let Some( window )=physical_windows.get_window(event.window)else{
            error!("failed to get window {:?}",event.window);
            continue;
        };
        let pos: Vec2 = Vec2::new(
            event.position.x,
            window.outer_size().height as f32 - event.position.y,
        );
        output_focus.0 = Some((event.window, pos.as_ivec2()));
        if resize_method.top {
            rect.set_y(pos.y as i32);
        }
        if resize_method.bottom {
            rect.set_height(pos.y as i32 - rect.pos().y);
        }
        if resize_method.left {
            rect.set_x(pos.x as i32);
        }
        if resize_method.right {
            rect.set_width(pos.x as i32 - rect.pos().x);
        }
    }
}
pub fn stop_resizing(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut state: ResMut<State<DWayClientState>>,
    mut resize_method: ResMut<ResizingMethod>,
    mut commands: Commands,
) {
    for event in cursor_button_events.iter() {
        if event.state == ButtonState::Released {
            commands.insert_resource(NextState(Some(DWayClientState::Desktop)));
            *resize_method = ResizingMethod::default();
            return;
        }
    }
}
