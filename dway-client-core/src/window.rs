use std::{collections::HashMap, mem::replace, time::SystemTime};

use bevy::{
    app::AppExit,
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat},
    sprite::MaterialMesh2dBundle,
};
use bevy_mod_picking::{highlight::Highlight, Hover, PickableBundle, PickableMesh, Selection};
use crossbeam_channel::{Receiver, TryRecvError};
use dway_protocol::window::WindowState;
use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use dway_util::stat::PerfLog;
use rand::Rng;
use uuid::Uuid;

use crate::{
    desktop::{FocusedWindow, WindowSet},
    protocol::WindowMessageReceiver,
    resizing::ResizingMethod,
    stages::DWayStage,
};

pub struct DWayWindowPlugin;
impl Plugin for DWayWindowPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PostUpdate, focus_on_new_window);
        app.add_system_to_stage(CoreStage::PreUpdate, receive_window_message);
    }
}

#[derive(Component, Default)]
pub struct WindowMetadata {
    pub uuid: Uuid,
    pub title: String,
    pub state: WindowState,
    pub bbox: Rect,
    pub geo: Rect,
}

#[derive(Bundle)]
pub struct WindowBundle {
    pub metadata: WindowMetadata,
    pub display: ImageBundle,
}
pub fn focus_on_new_window(
    mut focus: ResMut<FocusedWindow>,
    new_winodws: Query<Entity, Added<WindowMetadata>>,
) {
    if let Some(new_window) = new_winodws.iter().last() {
        focus.0 = Some(new_window);
    }
}
pub fn receive_window_message(
    mut commands: Commands,
    mut windows: Query<(&mut WindowMetadata, &mut UiImage, &mut Style)>,
    mut desktop: ResMut<WindowSet>,
    queue: Res<WindowMessageReceiver>,
    mut images: ResMut<Assets<Image>>,
    mut app_exit_events: EventWriter<AppExit>,
    mut message_count: Local<usize>,
    mut status: ResMut<State<DWayStage>>,
    mut resize_method: ResMut<ResizingMethod>,
) {
    // info!("poll messages");
    loop {
        match queue.0.try_recv() {
            Err(TryRecvError::Empty) => break,
            Err(e) => {
                error!("channel error {e}");
                app_exit_events.send(AppExit);
                return;
            }
            Ok(request) => {
                let tick = *message_count;
                *message_count += 1;
                let window_id = &request.uuid;
                match &request.data {
                    WindowMessageKind::Create { pos, size } => {
                        info!("new window {window_id}");
                        let buffer = vec![255; (size.x * size.y * 4.0) as usize];
                        let image = images.add(Image::new(
                            bevy::render::render_resource::Extent3d {
                                width: size.x as u32,
                                height: size.y as u32,
                                depth_or_array_layers: 1,
                            },
                            TextureDimension::D2,
                            buffer,
                            TextureFormat::Bgra8UnormSrgb,
                        ));
                        let entity = commands.spawn((
                            WindowMetadata {
                                bbox: Rect::from_corners(*pos, *pos + *size),
                                geo: Rect::from_corners(*pos, *pos + *size),
                                uuid: *window_id,
                                title: "".to_string(),
                                state: WindowState::Normal,
                            },
                            ImageBundle {
                                style: Style {
                                    size: Size::new(Val::Px(size.x), Val::Px(size.y)),
                                    position: UiRect {
                                        left: Val::Px(pos.x),
                                        right: Val::Auto,
                                        top: Val::Px(pos.y),
                                        bottom: Val::Auto,
                                    },
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                image: UiImage(image),
                                z_index: ZIndex::Global(windows.iter().count() as i32),
                                ..Default::default()
                            },
                        ));
                        desktop.window_set.insert(*window_id, entity.id());
                        continue;
                    }
                    _ => {}
                }
                let Some(&window_entity)=desktop.window_set.get(window_id) else{
            error!("window not found: {}",window_id);
            continue;
        };
                // let Ok((mut metadata,mut image,mut transform))=windows.get_mut(window_entity)else{
                //     error!("window entity not found: {:?}",window_entity);
                //     continue;
                // };
                let Ok((mut metadata,mut image,mut style))=windows.get_mut(window_entity)else{
            error!("window entity not found: {:?}",window_entity);
            continue;
        };
                match request.data {
                    WindowMessageKind::Create { .. } => {}
                    WindowMessageKind::Destroy => {
                        commands.entity(window_entity).despawn_recursive();
                    }
                    WindowMessageKind::Move(pos) => {
                        let pos = pos.as_vec2();
                        style.position = UiRect {
                            left: Val::Px(pos.x),
                            right: Val::Auto,
                            top: Val::Px(pos.y),
                            bottom: Val::Auto,
                        };
                        let rect = metadata.bbox;
                        metadata.bbox = Rect::from_corners(pos, rect.size() + pos);
                    }
                    WindowMessageKind::UpdateImage {
                        bbox,
                        geo,
                        image: ImageBuffer(size, data),
                    } => {
                        // let diff = geo.min - bbox.min;
                        // let bbox = Rect::from_corners(bbox.min - diff, bbox.max - diff);
                        // let geo = Rect::from_corners(geo.min - diff, geo.max - diff);
                        let new_image = UiImage(images.add(Image::new(
                            bevy::render::render_resource::Extent3d {
                                width: size.x as u32,
                                height: size.y as u32,
                                depth_or_array_layers: 1,
                            },
                            TextureDimension::D2,
                            data,
                            TextureFormat::Bgra8UnormSrgb,
                        )));
                        trace!(
                            "update image: duration: {:?}, image: {:?}",
                            SystemTime::now().duration_since(request.time).unwrap(),
                            new_image,
                        );
                        let UiImage(old_image) = replace(&mut *image, new_image);
                        images.remove(old_image);
                        style.size = Size::new(Val::Px(size.x), Val::Px(size.y));
                        style.position = UiRect {
                            left: Val::Px(bbox.min.x),
                            right: Val::Auto,
                            top: Val::Px(bbox.min.y),
                            bottom: Val::Auto,
                        };
                        metadata.bbox = Rect::from_corners(bbox.min, bbox.min + size);
                        metadata.geo = geo;
                    }
                    WindowMessageKind::MoveRequest => {
                        if let Err(e) = status.push(DWayStage::Moving) {
                            error!("failed to enter moving stage: {}", e);
                        };
                    }
                    WindowMessageKind::ResizeRequest {
                        top,
                        bottom,
                        left,
                        right,
                    } => {
                        *resize_method = ResizingMethod {
                            top,
                            bottom,
                            left,
                            right,
                        };
                        if let Err(e) = status.push(DWayStage::Resizing) {
                            error!("failed to enter resizing stage: {}", e);
                        };
                    }
                    _ => {
                        todo!()
                    }
                }
            }
        }
    }
}
pub fn move_window_relative(meta: &mut WindowMetadata, style: &mut Style, delta: Vec2) {
    move_window(meta, style, meta.geo.min + delta)
}
pub fn move_window(meta: &mut WindowMetadata, style: &mut Style, pos: Vec2) {
    let vec = pos - meta.geo.min;
    meta.geo.min += vec;
    meta.geo.max += vec;
    meta.bbox.min += vec;
    meta.bbox.max += vec;
    style.position = UiRect {
        left: Val::Px(meta.bbox.min.x),
        right: Val::Auto,
        top: Val::Px(meta.bbox.min.y),
        bottom: Val::Auto,
    };
}
pub fn set_window_rect(meta: &mut WindowMetadata, style: &mut Style, geo: Rect) {
    let scala_x = geo.width() / meta.geo.width();
    let scala_y = geo.height() / meta.geo.height();
    let bbox = Rect::new(
        geo.min.x - scala_x * (meta.geo.min.x - meta.bbox.min.x),
        geo.min.y - scala_y * (meta.geo.min.y - meta.bbox.min.y),
        geo.max.x - scala_x * (meta.geo.max.x - meta.bbox.max.x),
        geo.max.y - scala_y * (meta.geo.max.y - meta.bbox.max.y),
    );
    meta.geo = geo;
    meta.bbox = bbox;
    style.position = UiRect {
        left: Val::Px(bbox.min.x),
        right: Val::Auto,
        top: Val::Px(bbox.min.y),
        bottom: Val::Auto,
    };
}
