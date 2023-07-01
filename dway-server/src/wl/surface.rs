use bevy::{core::FrameCount, ecs::reflect, utils::HashSet};
use bevy_relationship::{graph_query, relationship, AppExt};
use wayland_server::backend::smallvec::SmallVec;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use crate::{
    geometry::Geometry,
    prelude::*,
    schedule::DWayServerSet,
    util::{rect::IRect, serial::next_serial},
    wl::buffer::WlBuffer,
    xdg::{toplevel::XdgToplevel, XdgSurface},
};
use std::{borrow::Cow, num::NonZeroUsize, sync::Arc};

relationship!(ClientHasSurface=>SurfaceList-<Client);

#[derive(Default, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlSurfacePeddingState {
    pub buffer: Option<Option<Entity>>,
    pub position: Option<IVec2>,
    pub damages: SmallVec<[IRect; 7]>,
    #[reflect(ignore)]
    pub callbacks: SmallVec<[wl_callback::WlCallback; 1]>,
    pub opaque_region: Option<Entity>,
    pub input_region: Option<Entity>,
    pub scale: Option<i32>,
    pub offset: Option<IVec2>,
    pub window_geometry: Option<IRect>,
}
#[derive(Default, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlSurfaceCommitedState {
    pub buffer: Option<Entity>,
    pub position: Option<IVec2>,
    pub damages: SmallVec<[IRect; 7]>,
    #[reflect(ignore)]
    pub callbacks: SmallVec<[wl_callback::WlCallback; 1]>,
    pub opaque_region: Option<Entity>,
    pub input_region: Option<Entity>,
    pub scale: Option<i32>,
    pub offset: Option<IVec2>,
    pub window_geometry: Option<IRect>,
}

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlSurface {
    #[reflect(ignore)]
    pub raw: wl_surface::WlSurface,
    pub commited: WlSurfaceCommitedState,
    pub pending: WlSurfacePeddingState,
    pub just_commit: bool,
    pub image: Handle<Image>,
    pub size: Option<IVec2>,
    pub commit_time: u32,
}
relationship!(AttachmentRelationship => Attach--AttachedBy);
#[derive(Bundle)]
pub struct WlSurfaceBundle {
    name: Name,
    resource: WlSurface,
    attach: Attach,
    client: Client,
}

impl WlSurfaceBundle {
    pub fn new(resource: WlSurface) -> Self {
        Self {
            name: Name::new(Cow::Owned(resource.raw.id().to_string())),
            resource,
            attach: Default::default(),
            client: Default::default(),
        }
    }
}

impl WlSurface {
    pub fn new(raw: wl_surface::WlSurface, assets: &mut Assets<Image>) -> Self {
        let image_size = Extent3d {
            width: 16,
            height: 16,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: Self::texture_descriptor(image_size),
            ..default()
        };
        image.resize(image_size);
        Self {
            raw,
            commited: Default::default(),
            pending: Default::default(),
            just_commit: false,
            image: assets.add(image),
            size: Default::default(),
            commit_time: 0,
        }
    }
    pub fn texture_descriptor<'l>(image_size: Extent3d) -> TextureDescriptor<'l> {
        TextureDescriptor {
            label: None,
            size: image_size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    // | TextureUsages::STORAGE_BINDING
                    | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        }
    }
    pub fn resize(&mut self, assets: &mut Assets<Image>, size: IVec2) {
        let image_size = Extent3d {
            width: size.x as u32,
            height: size.y as u32,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: Self::texture_descriptor(image_size),
            ..default()
        };
        image.resize(image_size);
        let assets = assets.set(self.image.clone(), image);
        assert_eq!(self.image, assets);
        debug!("resize image to {:?}", size);
        self.size = Some(size);
        self.commited
            .damages
            .push(IRect::from_pos_size((0, 0).into(), size));
        self.just_commit = true;
    }
}
#[derive(Component)]
pub struct WlSubsurface {
    pub raw: wl_subsurface::WlSubsurface,
    pub position: Option<IVec2>,
    pub above: Option<Entity>,
    pub below: Option<Entity>,
    pub sync: bool,
}

impl WlSubsurface {
    pub fn new(raw: wl_subsurface::WlSubsurface) -> Self {
        Self {
            raw,
            position: None,
            above: None,
            below: None,
            sync: false,
        }
    }
}
#[derive(Resource)]
pub struct SurfaceDelegate(pub GlobalId);
delegate_dispatch!(DWay: [wl_surface::WlSurface: Entity] => SurfaceDelegate);
impl wayland_server::Dispatch<wl_surface::WlSurface, bevy::prelude::Entity, DWay>
    for SurfaceDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wl_surface::WlSurface,
        request: <wl_surface::WlSurface as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_surface::Request::Destroy => todo!(),
            wl_surface::Request::Attach { buffer, x, y } => {
                let buffer_entity = buffer.map(|buffer| DWay::get_entity(&buffer));
                let origin_buffer_entity = state.with_component(resource, |c: &mut WlSurface| {
                    if resource.version() < 5 {
                        c.pending.position = Some(IVec2::new(x, y));
                    } else {
                        if x != 0 || y != 0 {
                            resource.post_error(
                                wl_surface::Error::InvalidOffset,
                                "Passing non-zero x,y is protocol violation since versions 5",
                            );
                        }
                    };
                    let origin_buffer = c.pending.buffer.take();
                    c.pending.buffer = Some(buffer_entity);
                    origin_buffer.flatten()
                });
                let world = state.world_mut();
                let mut buffer_query = world.query::<&mut WlBuffer>();
                if let Some(origin_buffer_entity) = origin_buffer_entity {
                    if let Ok(mut origin_buffer) = buffer_query.get_mut(world, origin_buffer_entity)
                    {
                        origin_buffer.attach_by = None;
                    }
                }
                if let Some(buffer_entity) = buffer_entity {
                    if let Ok(mut buffer) = buffer_query.get_mut(world, buffer_entity) {
                        buffer.attach_by = Some(*data);
                    }
                }
            }
            wl_surface::Request::Damage {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    c.pending.damages.push(IRect::new(x, y, width, height));
                });
            }
            wl_surface::Request::Frame { callback } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    let callback = data_init.init(callback, ());
                    c.pending.callbacks.push(callback);
                });
            }
            wl_surface::Request::SetOpaqueRegion { region } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    c.pending.opaque_region = region.map(|region| DWay::get_entity(&region));
                });
            }
            wl_surface::Request::SetInputRegion { region } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    c.pending.input_region = region.map(|region| DWay::get_entity(&region));
                });
            }
            wl_surface::Request::Commit => {
                debug!("commit");
                let frame_count = state.world().resource::<FrameCount>().0;
                let (old_buffer_entity, buffer_entity) = state.query_object::<(
                    &mut WlSurface,
                    Option<&mut Geometry>,
                    Option<&mut XdgSurface>,
                    Option<&mut XdgToplevel>,
                ), _, _>(
                    resource,
                    |(mut surface, mut geometry, mut xdg_surface, mut toplevel)| {
                        let old_buffer_entity = surface.commited.buffer;
                        if let Some(v) = surface.pending.buffer.take() {
                            surface.commited.buffer = v;
                        }
                        if let Some(v) = surface.pending.position.take() {
                            let _ = surface.commited.position.insert(v);
                        }
                        if let Some(v) = surface.pending.opaque_region.take() {
                            let _ = surface.commited.opaque_region.insert(v);
                        }
                        if let Some(v) = surface.pending.input_region.take() {
                            let _ = surface.commited.input_region.insert(v);
                        }
                        if let Some(v) = surface.pending.scale.take() {
                            let _ = surface.commited.scale.insert(v);
                        }
                        if let Some(offset) = surface.pending.offset.take() {
                            *surface.commited.offset.get_or_insert_default() += offset;
                        }
                        if let Some(window_geometry) = surface.pending.window_geometry.take() {
                            *surface.commited.window_geometry.insert(window_geometry);
                        }
                        let damages = surface.pending.damages.drain(..).collect::<Vec<_>>();
                        surface.commited.damages.extend(damages);
                        let callbacks = surface.pending.callbacks.drain(..).collect::<Vec<_>>();
                        surface.commited.callbacks.extend(callbacks);

                        surface.just_commit = true;
                        surface.commit_time = frame_count;

                        (old_buffer_entity, surface.commited.buffer)
                    },
                );
                if let Some(buffer) = buffer_entity {
                    if state.world_mut().get_entity(buffer).is_some() {
                        state.connect::<AttachmentRelationship>(*data, buffer);
                    }
                } else if let Some(old_buffer) = old_buffer_entity {
                    state.disconnect::<AttachmentRelationship>(*data, old_buffer);
                }
            }
            wl_surface::Request::SetBufferTransform { transform } => todo!(),
            wl_surface::Request::SetBufferScale { scale } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    c.pending.scale = Some(scale);
                });
            }
            wl_surface::Request::DamageBuffer {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    c.pending.damages.push(IRect::new(x, y, width, height));
                });
            }
            wl_surface::Request::Offset { x, y } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    let _ = c.pending.offset.insert(IVec2::new(x, y));
                });
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
impl
    wayland_server::Dispatch<
        wayland_server::protocol::wl_subsurface::WlSubsurface,
        bevy::prelude::Entity,
    > for DWay
{
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_subsurface::WlSubsurface,
        request: <wayland_server::protocol::wl_subsurface::WlSubsurface as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            wl_subsurface::Request::Destroy => todo!(),
            wl_subsurface::Request::SetPosition { x, y } => {
                state.with_component(resource, |c: &mut WlSubsurface| {
                    c.position = Some(IVec2::new(x, y));
                });
            }
            wl_subsurface::Request::PlaceAbove { sibling } => todo!(),
            wl_subsurface::Request::PlaceBelow { sibling } => todo!(),
            wl_subsurface::Request::SetSync => {
                state.with_component(resource, |c: &mut WlSubsurface| {
                    c.sync = true;
                });
            }
            wl_subsurface::Request::SetDesync => {
                state.with_component(resource, |c: &mut WlSubsurface| {
                    c.sync = false;
                });
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

impl wayland_server::Dispatch<wl_callback::WlCallback, ()> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &wl_callback::WlCallback,
        request: <wl_callback::WlCallback as WlResource>::Request,
        data: &(),
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            _ => todo!(),
        }
    }
}

pub fn cleanup_surface(
    mut surface_query: Query<(&mut WlSurface)>,
    mut buffer_query: Query<&mut WlBuffer>,
    time: Res<Time>,
) {
    surface_query.iter_mut().for_each(|mut surface| {
        if surface.commited.callbacks.len() > 0 {
            for callback in surface.commited.callbacks.drain(..) {
                debug!("{} done", WlResource::id(&callback));
                callback.done(time.elapsed().as_millis() as u32);
            }
        }
        if surface.commited.damages.len() > 0 {
            surface.commited.damages.clear();
        }
        if surface.just_commit {
            if let Some(buffer) = surface
                .commited
                .buffer
                .and_then(|e| buffer_query.get(e).ok())
            {
                buffer.raw.release();
            }
            surface.just_commit = false;
        }
    });
}

pub struct WlSurfacePlugin;
impl Plugin for WlSurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(cleanup_surface.in_base_set(CoreSet::First));
        app.add_system(update_buffer_size.in_set(DWayServerSet::UpdateGeometry));
        app.register_type::<WlSurface>();
        app.register_type::<Attach>();
        app.register_type::<AttachedBy>();
        app.register_type::<SurfaceList>();
        app.register_relation::<ClientHasSurface>();
    }
}
pub fn update_buffer_size(
    buffer_query: Query<&WlBuffer, Changed<WlBuffer>>,
    mut surface_query: Query<(&mut WlSurface, Option<&mut Geometry>)>,
    mut assets: ResMut<Assets<Image>>,
) {
    for buffer in buffer_query.iter() {
        let size = IVec2::new(buffer.width, buffer.height);
        if let Some((mut surface, mut geometry)) = buffer
            .attach_by
            .and_then(|entity| surface_query.get_mut(entity).ok())
        {
            if surface.size != Some(size) {
                surface.resize(&mut assets, size);
            }
            if let Some(mut geometry) = geometry {
                geometry.geometry.set_size(size);
            }
        }
    }
}
