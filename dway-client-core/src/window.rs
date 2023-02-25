use std::{mem::replace, time::SystemTime};

use bevy::{
    app::AppExit,
    prelude::*,
    render::render_resource::{TextureDimension, TextureFormat},
};

use crossbeam_channel::{TryRecvError};
use dway_protocol::window::WindowState;
use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};


use uuid::Uuid;

use crate::{
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
            CoreStage::PreUpdate,
            receive_window_messages.label(WindowLabel::Receive),
        );
        app.add_system(update_window_ui_rect.label(WindowLabel::UpdateUi));
    }
}

#[derive(Component, Default)]
pub struct WindowMetadata {
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
}
pub fn focus_on_new_window(
    mut focus: ResMut<FocusedWindow>,
    new_winodws: Query<Entity, Added<WindowMetadata>>,
) {
    if let Some(new_window) = new_winodws.iter().last() {
        focus.0 = Some(new_window);
    }
}
pub fn receive_window_messages(
    world: &mut World,
    // mut app_exit_events: EventWriter<AppExit>,
    // queue: Res<WindowMessageReceiver>,
    mut system: Local<Option<Box<dyn System<In = WindowMessage, Out = ()>>>>,
) {
    let Some( queue )=world.get_resource::<WindowMessageReceiver>()else{
        return
    };
    let queue = queue.0.clone();
    loop {
        match queue.try_recv() {
            Err(TryRecvError::Empty) => break,
            Err(e) => {
                error!("channel error {e}");
                world.send_event(AppExit);
                return;
            }
            Ok(message) => {
                let system = system.get_or_insert_with(|| {
                    let mut system = IntoSystem::into_system(receive_window_message);
                    system.initialize(world);
                    Box::new(system)
                });
                system.run(message, world);
                system.apply_buffers(world);
            }
        }
    }
}
pub fn receive_window_message(
    message: In<WindowMessage>,
    // mut commands: ParallelCommands,
    mut commands: Commands,
    mut windows: Query<(&mut WindowMetadata, &mut UiImage)>,
    mut desktop: ResMut<WindowSet>,
    mut images: ResMut<Assets<Image>>,
    mut message_count: Local<usize>,
    mut status: ResMut<State<DWayStage>>,
    mut resize_method: ResMut<ResizingMethod>,
) {
    let request = message.0;
    let _tick = *message_count;
    *message_count += 1;
    let window_id = &request.uuid;
    match &request.data {
        WindowMessageKind::Create { pos, size } => {
            create_window(
                window_id,
                *size,
                *pos,
                windows.iter().count() as i32,
                &mut commands,
                &mut images,
                &mut desktop,
            );
        }
        _ => {}
    }
    let Some(&window_entity)=desktop.window_set.get(window_id) else{
            error!("window not found: {}",window_id);
            return;
        };
    let Ok((mut meta,mut image))=windows.get_mut(window_entity)else{
            error!("window entity not found: {:?} {window_id}",window_entity);
            return;
        };
    match request.data {
        WindowMessageKind::Create { .. } => {}
        WindowMessageKind::Destroy => {
            info!("destroy window {:?} {window_id}", window_entity);
            commands.entity(window_entity).despawn_recursive();
            desktop.window_set.remove(window_id);
        }
        WindowMessageKind::Move(pos) => {
            let pos = pos.as_vec2();
            let rect = meta.bbox;
            meta.bbox = Rect::from_corners(pos, rect.size() + pos);
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
            let bbox = Rect::from_corners(bbox.min, bbox.min + size);
            if meta.bbox != bbox {
                meta.bbox = bbox;
            }
            if meta.geo != geo {
                meta.geo = geo;
            }
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
        WindowMessageKind::SetRect(geo) => {
            set_window_rect(&mut meta, geo);
        }
        WindowMessageKind::Maximized => {
            info!("WindowMessageKind::Maximized");
            if meta.backup_geo.is_none() {
                meta.backup_geo = Some(meta.geo);
            }
            meta.state = WindowState::Maximized;
        }
        WindowMessageKind::Minimized => {
            if meta.backup_geo.is_none() {
                meta.backup_geo = Some(meta.geo);
            }
            meta.state = WindowState::Minimized;
        }
        WindowMessageKind::FullScreen => {
            if meta.backup_geo.is_none() {
                meta.backup_geo = Some(meta.geo);
            } else {
                warn!("no normal geometry info");
            }
            meta.state = WindowState::FullScreen;
        }
        WindowMessageKind::UnFullScreen => {
            if let Some(geo) = meta.backup_geo.take() {
                set_window_rect(&mut meta, geo);
            } else {
                warn!("no normal geometry info");
            }
            meta.state = WindowState::Normal;
        }
        WindowMessageKind::Unmaximized => {
            info!("WindowMessageKind::Unmaximize");
            if let Some(geo) = meta.backup_geo.take() {
                set_window_rect(&mut meta, geo);
            } else {
                warn!("no normal geometry info");
            }
            meta.state = WindowState::Normal;
        }
        WindowMessageKind::Unminimized => {
            info!("WindowMessageKind::Unminimized");
            if let Some(geo) = meta.backup_geo.take() {
                set_window_rect(&mut meta, geo);
            } else {
                warn!("no normal geometry info");
            }
            meta.state = WindowState::Normal;
        }
        o => {
            panic!("not implemented, message: {o:?}");
        }
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
pub fn update_window_ui_rect(
    focused_output: Res<CursorOnOutput>,
    outputs: Res<Windows>,
    mut windows: Query<(&mut WindowMetadata, &mut Style, &mut Visibility), Changed<WindowMetadata>>,
    sender: Res<WindowMessageSender>,
) {
    if windows.is_empty() {
        return;
    }
    let  Some(output)=focused_output.0.as_ref().and_then(|(id,_)|outputs.get(*id)) else{
        return;
    };
    for (mut meta, mut style, mut visibility) in windows.iter_mut() {
        match meta.state {
            WindowState::Normal => {
                if !visibility.is_visible {
                    visibility.is_visible = true;
                }
                let rect = meta.geo;
                set_window_rect(&mut meta, rect);
                if let Err(e) = sender.0.send(WindowMessage {
                    uuid: meta.uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Normal,
                }) {
                    error!("failed to send message: {}", e);
                    continue;
                };
                if let Err(e) = sender.0.send(WindowMessage {
                    uuid: meta.uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::SetRect(rect),
                }) {
                    error!("failed to send message: {}", e);
                    continue;
                };
            }
            WindowState::Minimized => {
                if visibility.is_visible {
                    visibility.is_visible = false;
                }
            }
            WindowState::Maximized => {
                if !visibility.is_visible {
                    visibility.is_visible = true;
                }
                let pos = Vec2::new(0.0, 0.0);
                let rect =
                    Rect::from_corners(pos, pos + Vec2::new(output.width(), output.height()));
                if meta.geo != rect {
                    set_window_rect(&mut meta, rect);
                    if let Err(e) = sender.0.send(WindowMessage {
                        uuid: meta.uuid,
                        time: SystemTime::now(),
                        data: WindowMessageKind::SetRect(rect),
                    }) {
                        error!("failed to send message: {}", e);
                        continue;
                    };
                }
                if let Err(e) = sender.0.send(WindowMessage {
                    uuid: meta.uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Maximized,
                }) {
                    error!("failed to send message: {}", e);
                    continue;
                };
            }
            WindowState::FullScreen => {
                if !visibility.is_visible {
                    visibility.is_visible = false;
                }
                let pos = Vec2::new(0.0, 0.0);
                let rect =
                    Rect::from_corners(pos, pos + Vec2::new(output.width(), output.height()));
                if meta.geo != rect {
                    set_window_rect(&mut meta, rect);
                    if let Err(e) = sender.0.send(WindowMessage {
                        uuid: meta.uuid,
                        time: SystemTime::now(),
                        data: WindowMessageKind::SetRect(rect),
                    }) {
                        error!("failed to send message: {}", e);
                        continue;
                    };
                }
            }
        }
        let bbox = meta.bbox;
        style.size = Size::new(Val::Px(bbox.width()), Val::Px(bbox.height()));
        style.position = UiRect {
            left: Val::Px(bbox.min.x),
            right: Val::Auto,
            top: Val::Px(bbox.min.y),
            bottom: Val::Auto,
        };
    }
}

pub fn create_window(
    window_id: &Uuid,
    size: Vec2,
    pos: Vec2,
    z_index: i32,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    desktop: &mut WindowSet,
) {
    let buffer = vec![0; (size.x * size.y * 4.0) as usize];
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
            bbox: Rect::from_corners(pos, pos + size),
            geo: Rect::from_corners(pos, pos + size),
            backup_geo: None,
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
            z_index: ZIndex::Global(z_index),
            ..Default::default()
        },
    ));
    info!("new window {:?} {window_id}", entity.id());
    desktop.window_set.insert(*window_id, entity.id());
}
