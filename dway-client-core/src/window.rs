use dway_util::ecs::QueryResultExt;
use std::{mem::replace, time::SystemTime};

use bevy::{
    app::AppExit,
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat},
    ui::FocusPolicy,
    window::WindowMode,
    winit::WinitWindows,
};

use crossbeam_channel::TryRecvError;
use dway_protocol::window::WindowState;
use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use dway_server::{
    components::{Id, SurfaceOffset, WindowScale},
    events::{CreateWindow, MouseButtonOnWindow, MouseMoveOnWindow},
    math::{ivec2_to_point, point_to_ivec2, rectangle_i32_center, vec2_to_point},
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
use dway_util::rect;
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    components::{AttachToOutput, OutputMark},
    desktop::{CursorOnOutput, FocusedWindow, WindowSet},
    protocol::{WindowMessageReceiver, WindowMessageSender},
    resizing::ResizingMethod,
    DWayClientSystem,
};

pub struct DWayWindowPlugin;
impl Plugin for DWayWindowPlugin {
    fn build(&self, app: &mut App) {
        use DWayClientSystem::*;
        app.add_system(focus_on_new_window.in_set(DWayClientSystem::UpdateFocus));
        app.add_system(create_window_ui.in_set(Create));
        app.add_system(update_window_state.in_set(UpdateState));
        app.add_system(update_window_geo.in_set(UpdateState));
        app.add_system(destroy_window_ui.in_set(Destroy));
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Frontends(pub SmallVec<[Entity; 1]>);

impl std::ops::Deref for Frontends {
    type Target = SmallVec<[Entity; 1]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Backend(pub Entity);
impl Backend {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Debug)]
pub struct WindowUiRoot {
    pub input_rect_entity: Entity,
    pub surface_rect_entity: Entity,
}

#[derive(Bundle)]
pub struct WindowBundle {
    pub root: WindowUiRoot,
    pub display: ImageBundle,
    pub backend: Backend,
}
pub fn focus_on_new_window(
    mut focus: ResMut<FocusedWindow>,
    new_winodws: Query<Entity, Added<Backend>>,
) {
    if let Some(new_window) = new_winodws.iter().last() {
        focus.0 = Some(new_window);
    }
}

pub fn create_window_ui(
    surface_query: Query<
        (
            Entity,
            &GlobalPhysicalRect,
            &SurfaceId,
            &ImportedSurface,
            Option<&SurfaceOffset>,
        ),
        With<WindowMark>,
    >,
    mut events: EventReader<CreateWindow>,
    window_index: Res<WindowIndex>,
    mut commands: Commands,
) {
    for CreateWindow(id) in events.iter() {
        if let Some((entity, rect, id, surface, offset)) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).map_err(|e| error!("{e}")).ok())
        {
            let backend = Backend::new(entity);
            let offset = offset.cloned().unwrap_or_default().0;
            let input_rect_entity = commands
                .spawn((
                    ButtonBundle {
                        style: Style {
                            position: UiRect {
                                left: Val::Px(-offset.loc.x as f32),
                                right: Val::Auto,
                                top: Val::Px(-offset.loc.y as f32),
                                bottom: Val::Auto,
                            },
                            size: Size::new(
                                Val::Px(rect.0.size.w as f32),
                                Val::Px(rect.0.size.h as f32),
                            ),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::WHITE.with_a(0.2)),
                        ..default()
                    },
                    backend,
                    id.clone(),
                ))
                .id();
            let bbox_loc = rect.0.loc + offset.loc;
            let bbox_size = rect.0.size.to_point() - offset.loc - offset.loc;
            let surface_rect_entity = commands
                .spawn((
                    ImageBundle {
                        style: Style {
                            position: UiRect {
                                left: Val::Px(bbox_loc.x as f32),
                                right: Val::Auto,
                                top: Val::Px(bbox_loc.y as f32),
                               bottom: Val::Auto,
                            },
                            size: Size::new(
                                Val::Px(bbox_size.x as f32),
                                Val::Px(bbox_size.y as f32),
                            ),
                            ..default()
                        },
                        focus_policy: FocusPolicy::Pass,
                        image: UiImage::new(surface.texture.clone()),
                        ..default()
                    },
                    backend,
                    id.clone(),
                ))
                .add_child(input_rect_entity)
                .id();
            let ui_entity = commands
                .spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        focus_policy: FocusPolicy::Pass,
                        ..Default::default()
                    },
                    WindowUiRoot {
                        input_rect_entity,
                        surface_rect_entity,
                    },
                    backend,
                    id.clone(),
                ))
                .add_child(surface_rect_entity)
                .id();
            commands.entity(ui_entity).log_components();
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

pub fn update_window_state(
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

pub fn update_window_geo(
    window_query: Query<(&Backend, &WindowUiRoot)>,
    mut style_query: Query<&mut Style>,
    surface_query: Query<
        (&GlobalPhysicalRect, Option<&SurfaceOffset>),
        (
            With<WindowMark>,
            Or<(Changed<GlobalPhysicalRect>, Changed<SurfaceOffset>)>,
        ),
    >,
) {
    for (backend, ui_root) in window_query.iter() {
        if let Ok((rect, surface_offset)) = surface_query.get(backend.get()) {
            let offset = surface_offset.cloned().unwrap_or_default();
            let bbox_loc = rect.0.loc + offset.0.loc;
            let bbox_size = rect.0.size.to_point() - offset.0.loc - offset.0.loc;
            if let Ok(mut style) = style_query.get_mut(ui_root.input_rect_entity) {
                style.position = UiRect {
                    left: Val::Px(-offset.loc.x as f32),
                    right: Val::Auto,
                    top: Val::Px(-offset.loc.y as f32),
                    bottom: Val::Auto,
                };
                style.size =
                    Size::new(Val::Px(rect.0.size.w as f32), Val::Px(rect.0.size.h as f32));
            }
            if let Ok(mut style) = style_query.get_mut(ui_root.surface_rect_entity) {
                style.position = UiRect {
                    left: Val::Px(bbox_loc.x as f32),
                    right: Val::Auto,
                    top: Val::Px(bbox_loc.y as f32),
                    bottom: Val::Auto,
                };
                style.size = Size::new(Val::Px(bbox_size.x as f32), Val::Px(bbox_size.y as f32));
            }
        }
    }
}
