use std::{mem::replace, time::SystemTime};

use bevy::{
    app::AppExit,
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat},
    window::WindowMode,
    winit::WinitWindows,
};

use crossbeam_channel::TryRecvError;
use dway_protocol::window::WindowState;
use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use dway_server::{
    components::WindowScale,
    events::{CreateWindow, MouseButtonOnWindow, MouseMoveOnWindow},
    math::{ivec2_to_point, rectangle_i32_center, vec2_to_point},
    DWayServerLabel, DWayServerStage,
};

use dway_server::{
    components::{
        GlobalPhysicalRect, LogicalRect, NormalModeGlobalRect, PhysicalRect, SurfaceId,
        WindowIndex, WindowMark, UUID,
    },
    events::{DestroyWlSurface, WindowSetGeometryEvent},
    math::{rect_to_rectangle, rectangle_i32_to_rect},
    surface::ImportedSurface,
};
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    components::{AttachToOutput, OutputMark},
    desktop::{CursorOnOutput, FocusedWindow, WindowSet},
    protocol::{WindowMessageReceiver, WindowMessageSender},
    resizing::ResizingMethod,
    stages::DWayStage,
};

#[derive(SystemLabel)]
pub enum WindowLabel {
    Input,
    Receive,
    UpdateLogic,
    UpdateUi,
}

pub struct DWayWindowPlugin;
impl Plugin for DWayWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            CoreStage::Update,
            focus_on_new_window.label(WindowLabel::UpdateLogic),
        );
        app.add_system_to_stage(
            CoreStage::Update,
            create_window_ui.label(WindowLabel::UpdateLogic),
        );
        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(on_window_state_changed.label(WindowLabel::UpdateUi))
                .with_system(update_window_ui.label(WindowLabel::UpdateUi)),
        );
        app.add_system_to_stage(
            DWayServerStage::Send,
            destroy_window_ui.label(DWayServerLabel::Destroy),
        );
        // app.add_system_to_stage(
        //     CoreStage::PreUpdate,
        //     receive_window_messages.label(WindowLabel::Receive),
        // );
        // app.add_system(update_window_ui_rect.label(WindowLabel::UpdateUi));
    }
}

#[derive(Component, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Frontends(pub SmallVec<[Entity; 1]>);

impl std::ops::Deref for Frontends {
    type Target = SmallVec<[Entity; 1]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backend(pub Entity);
impl Backend {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Component)]
pub struct WindowMetadata {
    pub id: SurfaceId,
    pub uuid: Uuid,
    pub title: String,
    pub state: WindowState,
    pub backup_geo: Option<Rect>,
    pub bbox: Rect,
    pub geo: Rect,
}

#[derive(Bundle)]
pub struct WindowBundle {
    pub metadata: WindowMetadata,
    pub display: ImageBundle,
    pub backend: Backend,
}
pub fn focus_on_new_window(
    mut focus: ResMut<FocusedWindow>,
    new_winodws: Query<Entity, Added<WindowMetadata>>,
) {
    if let Some(new_window) = new_winodws.iter().last() {
        focus.0 = Some(new_window);
    }
}
pub fn move_window(meta: &mut Mut<WindowMetadata>, delta: Vec2) {
    set_window_position(meta, meta.geo.min + delta)
}
pub fn set_window_position(meta: &mut Mut<WindowMetadata>, pos: Vec2) {
    let vec = pos - meta.geo.min;
    if vec != Vec2::ZERO {
        let meta = &mut **meta;
        meta.geo.min += vec;
        meta.geo.max += vec;
        meta.bbox.min += vec;
        meta.bbox.max += vec;
    }
}
pub fn set_window_rect(meta: &mut Mut<WindowMetadata>, geo: Rect) {
    let scala_x = geo.width() / meta.geo.width();
    let scala_y = geo.height() / meta.geo.height();
    let bbox = Rect::new(
        geo.min.x - scala_x * (meta.geo.min.x - meta.bbox.min.x),
        geo.min.y - scala_y * (meta.geo.min.y - meta.bbox.min.y),
        geo.max.x - scala_x * (meta.geo.max.x - meta.bbox.max.x),
        geo.max.y - scala_y * (meta.geo.max.y - meta.bbox.max.y),
    );
    if meta.geo != geo || meta.bbox != bbox {
        meta.geo = geo;
        meta.bbox = bbox;
    }
}
pub fn on_window_state_changed(
    mut surface_query: Query<
        (
            &Frontends,
            &mut PhysicalRect,
            &WindowState,
            Option<&NormalModeGlobalRect>,
            Option<&AttachToOutput>,
        ),
        (Changed<WindowState>, With<WindowMark>),
    >,
    mut window_ui_query: Query<(&mut Visibility, &mut Style)>,
    output_query: Query<&GlobalPhysicalRect, With<OutputMark>>,
    mut commands: Commands,
) {
    for (frontends, mut physical_rect, state, normal_rect, attach_to_output) in
        surface_query.iter_mut()
    {
        for frontend in frontends.iter() {
            if let Ok((mut visibility, mut style)) = window_ui_query.get_mut(*frontend) {
                match state {
                    WindowState::Normal => {
                        *visibility = Visibility::Inherited;
                        if let Some(rect) = normal_rect {
                            physical_rect.0 = rect.0;
                            commands.entity(*frontend).remove::<NormalModeGlobalRect>();
                        }
                    }
                    WindowState::Minimized => {
                        *visibility = Visibility::Hidden;
                    }
                    WindowState::Maximized | WindowState::FullScreen => {
                        *visibility = Visibility::Inherited;
                        let center = rectangle_i32_center(physical_rect.0);
                        let output = attach_to_output
                            .and_then(|o| o.get(1).copied())
                            .and_then(|e| output_query.get(e).ok())
                            .or_else(|| {
                                output_query
                                    .iter()
                                    .find_map(|output| output.contains(center).then_some(output))
                            });
                        if let Some(output_rect) = output {
                            commands
                                .entity(*frontend)
                                .insert(NormalModeGlobalRect(physical_rect.0));
                            physical_rect.0 = output_rect.0;
                        }
                    }
                }
                style.size = Size::new(
                    Val::Px(physical_rect.0.size.w as f32),
                    Val::Px(physical_rect.0.size.h as f32),
                );
                style.position = UiRect {
                    left: Val::Px(physical_rect.0.loc.x as f32),
                    right: Val::Auto,
                    top: Val::Px(physical_rect.0.loc.y as f32),
                    bottom: Val::Auto,
                };
            }
        }
    }
}

pub fn create_window_ui(
    surface_query: Query<
        (
            Entity,
            &GlobalPhysicalRect,
            &SurfaceId,
            &ImportedSurface,
            &UUID,
            Option<&WindowState>,
        ),
        With<WindowMark>,
    >,
    mut events: EventReader<CreateWindow>,
    window_index: Res<WindowIndex>,
    mut commands: Commands,
) {
    for CreateWindow(id) in events.iter() {
        if let Some((entity, rect, id, surface, uuid, state)) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let state = state.cloned().unwrap_or_default();
            let style = Style {
                position: UiRect {
                    left: Val::Px(rect.0.loc.y as f32),
                    right: Val::Auto,
                    top: Val::Px(rect.0.loc.x as f32),
                    bottom: Val::Auto,
                },
                position_type: PositionType::Absolute,
                size: Size::new(Val::Px(rect.0.size.w as f32), Val::Px(rect.0.size.h as f32)),
                ..Default::default()
            };
            let backend = Backend::new(entity);
            let ui_entity = commands
                .spawn((
                    // WindowMetadata {
                    //     bbox: rect.to_rect(),
                    //     geo: rect.to_rect(),
                    //     backup_geo: None,
                    //     uuid: uuid.0,
                    //     title: "".to_string(),
                    //     state,
                    //     id: id.clone(),
                    // },
                    ButtonBundle {
                        style: style.clone(),
                        image: UiImage::new(surface.texture.clone()),
                        ..Default::default()
                    },
                    backend,
                ))
                .id();
            commands
                .entity(entity)
                .insert(Frontends(SmallVec::from_buf([ui_entity])));
            info!("create front end of {id:?} on {entity:?},texture:{:?}, rect:{rect:?}, ui: [{ui_entity:?}]",&surface.texture);
        }
    }
}
pub fn destroy_window_ui(
    mut events: EventReader<DestroyWlSurface>,
    window_index: Res<WindowIndex>,
    mut window_query: Query<&mut Frontends, With<WindowMark>>,
    mut commands: Commands,
) {
    for DestroyWlSurface(id) in events.iter() {
        if let Some(mut frontends) = window_index
            .get(id)
            .and_then(|e| window_query.get_mut(*e).ok())
        {
            for frontend in frontends.0.drain(..) {
                commands.entity(frontend).despawn_recursive();
            }
        }
    }
}
pub fn update_window_ui(
    mut window_query: Query<(&Backend, &mut WindowMetadata, &mut Style, &mut Visibility)>,
    surface_query: Query<
        (&GlobalPhysicalRect, Option<&WindowState>),
        (
            With<WindowMark>,
            Or<(Changed<ImportedSurface>, Changed<GlobalPhysicalRect>)>,
        ),
    >,
) {
    for (backend, mut meta, mut style, mut visibility) in window_query.iter_mut() {
        if let Ok((rect, state)) = surface_query.get(backend.get()) {
            if let Some(state) = state {
                if state != &meta.state {
                    match state {
                        WindowState::Normal => {
                            *visibility = Visibility::Visible;
                        }
                        WindowState::Minimized => {
                            *visibility = Visibility::Hidden;
                        }
                        WindowState::Maximized => {
                            *visibility = Visibility::Visible;
                        }
                        WindowState::FullScreen => {
                            *visibility = Visibility::Visible;
                        }
                    }
                }
                meta.state = *state;
            }
            meta.bbox = rect.to_rect();
            meta.geo = rect.to_rect();
            style.position = UiRect {
                left: Val::Px(rect.0.loc.y as f32),
                right: Val::Auto,
                top: Val::Px(rect.0.loc.x as f32),
                bottom: Val::Auto,
            };
            style.size = Size::new(Val::Px(rect.0.size.w as f32), Val::Px(rect.0.size.h as f32));
        }
    }
}
