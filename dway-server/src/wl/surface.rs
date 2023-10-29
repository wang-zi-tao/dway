use bevy::core::FrameCount;
use bevy_relationship::{relationship, AppExt, Connectable};
use wayland_server::backend::smallvec::SmallVec;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use crate::{
    geometry::Geometry,
    prelude::*,
    schedule::DWayServerSet,
    util::rect::IRect,
    wl::buffer::{UninitedWlBuffer, WlShmBuffer},
    xdg::popup::XdgPopup,
    zwp::dmabufparam::DmaBuffer,
};
use std::borrow::Cow;

relationship!(ClientHasSurface=>SurfaceList-<ClientRef);

#[derive(Default, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlSurfacePeddingState {
    #[reflect(ignore)]
    pub wl_buffer: Option<Option<wl_buffer::WlBuffer>>,
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
    #[reflect(ignore)]
    pub wl_buffer: Option<wl_buffer::WlBuffer>,
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
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_surface::WlSurface,
    pub commited: WlSurfaceCommitedState,
    pub pending: WlSurfacePeddingState,
    pub just_commit: bool,
    pub image: Handle<Image>,
    pub size: Option<IVec2>,
    pub commit_time: u32,
    pub commit_count: u32,
}
relationship!(AttachmentRelationship => AttachTo--AttachedBy);
relationship!(SurfaceHasInputRegion => InputRegion>-IsInputRegionOf);
relationship!(SurfaceHasOpaqueRegion => OpaqueRegion>-IsOpaqueRegionOf);
#[derive(Bundle)]
pub struct WlSurfaceBundle {
    name: Name,
    resource: WlSurface,
    attach: AttachTo,
    client: ClientRef,
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
            commit_count: 0,
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
        debug!(resource=%self.raw.id(),"resize image to {:?}", size);
        self.size = Some(size);
        self.commited
            .damages
            .push(IRect::from_pos_size((0, 0).into(), size));
        self.just_commit = true;
    }
    pub fn image_rect(&self) -> IRect {
        IRect::from_pos_size(
            -self.commited.window_geometry.unwrap_or_default().pos(),
            self.size.unwrap_or_default(),
        )
    }
}
#[derive(Component)]
pub struct WlSubsurface {
    pub raw: wl_subsurface::WlSubsurface,
    pub position: Option<IVec2>,
    pub above: Option<Entity>,
    pub below: Option<Entity>,
    pub sync: bool,
    pub desync: bool,
}

impl WlSubsurface {
    pub fn new(raw: wl_subsurface::WlSubsurface) -> Self {
        Self {
            raw,
            position: None,
            above: None,
            below: None,
            sync: false,
            desync: false,
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
        _client: &wayland_server::Client,
        resource: &wl_surface::WlSurface,
        request: <wl_surface::WlSurface as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_surface::Request::Destroy => {
                state.despawn(*data);
            }
            wl_surface::Request::Attach { buffer, x, y } => {
                let buffer_entity = if let Some(buffer) = &buffer {
                    if buffer.data::<Entity>().is_none() {
                        let entity = state
                            .spawn((
                                Name::new(buffer.id().to_string()),
                                UninitedWlBuffer::new(buffer.clone()),
                            ))
                            .set_parent(*data)
                            .id();
                        debug!(?entity,resource =%buffer.id(), "create uninited wl_buffer");
                        Some(entity)
                    } else {
                        Some(DWay::get_entity(buffer))
                    }
                } else {
                    None
                };
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    if resource.version() < 5 {
                        c.pending.position = Some(IVec2::new(x, y));
                    } else if x != 0 || y != 0 {
                        resource.post_error(
                            wl_surface::Error::InvalidOffset,
                            "Passing non-zero x,y is protocol violation since versions 5",
                        );
                    };
                    let _origin_buffer = c.pending.buffer.take();
                    c.pending.buffer = Some(buffer_entity);
                    if let Some(Some(wl_buffer)) = &c.pending.wl_buffer {
                        if wl_buffer.is_alive() {
                            wl_buffer.release()
                        }
                    }
                    c.pending.wl_buffer = Some(buffer);
                };
            }
            wl_surface::Request::Damage {
                x,
                y,
                width,
                height,
            } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    c.pending.damages.push(IRect::new(x, y, width, height));
                };
            }
            wl_surface::Request::Frame { callback } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    let callback = data_init.init(callback, ());
                    c.pending.callbacks.push(callback);
                };
            }
            wl_surface::Request::SetOpaqueRegion { region } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    c.pending.opaque_region = region.map(|region| DWay::get_entity(&region));
                }
            }
            wl_surface::Request::SetInputRegion { region } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    c.pending.input_region = region.map(|region| DWay::get_entity(&region));
                }
            }
            wl_surface::Request::Commit => {
                let _enterd = span!(Level::DEBUG, "commit").entered();
                let frame_count = state.world().resource::<FrameCount>().0;
                let _system = || {};
                let (old_buffer_entity, buffer_entity, input_region_entity, opaque_region_entity) =
                    state.query_object::<(
                        &mut WlSurface,
                        Option<&mut Geometry>,
                        Option<&mut XdgPopup>,
                    ), _, _>(
                        resource,
                        |(mut surface, geometry, popup)| {
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
                                let _ = *surface.commited.window_geometry.insert(window_geometry);
                                if let Some(mut geometry) = geometry {
                                    geometry.geometry.set_size(window_geometry.size());
                                }
                            }
                            if let Some(wl_buffer) = surface.pending.wl_buffer.take() {
                                surface.commited.wl_buffer.as_ref().map(|b| {
                                    if b.is_alive() {
                                        b.release()
                                    }
                                });
                                surface.commited.wl_buffer = wl_buffer;
                            }
                            let damages = surface.pending.damages.drain(..).collect::<Vec<_>>();
                            surface.commited.damages.extend(damages);
                            let callbacks = surface.pending.callbacks.drain(..).collect::<Vec<_>>();
                            surface.commited.callbacks.extend(callbacks);

                            surface.just_commit = true;
                            surface.commit_time = frame_count;
                            surface.commit_count += 1;

                            if let Some(mut popup) = popup {
                                if !popup.send_configure {
                                    let size = surface.size.unwrap_or_default();
                                    popup.raw.configure(0, 0, size.x, size.y);
                                    popup.send_configure = true;
                                }
                            }

                            (
                                old_buffer_entity,
                                surface.commited.buffer,
                                surface.commited.input_region,
                                surface.commited.opaque_region,
                            )
                        },
                    );
                if let Some(buffer_entity) = buffer_entity {
                    state.connect::<AttachmentRelationship>(*data, buffer_entity);
                } else if let Some(old_buffer_entity) = old_buffer_entity {
                    state.disconnect::<AttachmentRelationship>(*data, old_buffer_entity);
                }
                if let Some(e) = input_region_entity {
                    state.connect::<SurfaceHasInputRegion>(*data, e)
                }
                if let Some(e) = opaque_region_entity {
                    state.connect::<SurfaceHasOpaqueRegion>(*data, e)
                }
            }
            wl_surface::Request::SetBufferTransform { transform: _ } => todo!(),
            wl_surface::Request::SetBufferScale { scale } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    c.pending.scale = Some(scale);
                }
            }
            wl_surface::Request::DamageBuffer {
                x,
                y,
                width,
                height,
            } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    c.pending.damages.push(IRect::new(x, y, width, height));
                }
            }
            wl_surface::Request::Offset { x, y } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data){
                    let _ = c.pending.offset.insert(IVec2::new(x, y));
                }
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_surface::WlSurface,
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
        _client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_subsurface::WlSubsurface,
        request: <wayland_server::protocol::wl_subsurface::WlSubsurface as WlResource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            wl_subsurface::Request::Destroy => {
                state.destroy_object(resource);
            }
            wl_subsurface::Request::SetPosition { x, y } => {
                if let Some(mut c) = state.get_mut::<WlSubsurface>(*data){
                    c.position = Some(IVec2::new(x, y));
                }
            }
            wl_subsurface::Request::PlaceAbove { sibling: _ } => todo!(),
            wl_subsurface::Request::PlaceBelow { sibling: _ } => todo!(),
            wl_subsurface::Request::SetSync => {
                if let Some(mut c) = state.get_mut::<WlSubsurface>(*data){
                    c.sync = true;
                }
            }
            wl_subsurface::Request::SetDesync => {
                if let Some(mut c) = state.get_mut::<WlSubsurface>(*data){
                    c.desync = false;
                }
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_subsurface::WlSubsurface,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

impl wayland_server::Dispatch<wl_callback::WlCallback, ()> for DWay {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &wl_callback::WlCallback,
        _request: <wl_callback::WlCallback as WlResource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}

pub fn cleanup_buffer(buffer_query: Query<(&WlShmBuffer, &AttachedBy)>) {
    buffer_query.for_each(|(buffer, attached_by)| {
        if attached_by.iter().next().is_some() {
            buffer.raw.release();
        }
    });
}

pub fn cleanup_surface(mut surface_query: Query<&mut WlSurface>, time: Res<Time>) {
    surface_query.iter_mut().for_each(|mut surface| {
        if !surface.commited.callbacks.is_empty() {
            for callback in surface.commited.callbacks.drain(..) {
                debug!("{} done", WlResource::id(&callback));
                callback.done(time.elapsed().as_millis() as u32);
            }
        }
        if !surface.commited.damages.is_empty() {
            surface.commited.damages.clear();
        }
        if surface.just_commit {
            surface.just_commit = false;
        }
    });
}

pub struct WlSurfacePlugin;
impl Plugin for WlSurfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, cleanup_surface);
        app.add_systems(
            PreUpdate,
            update_buffer_size.in_set(DWayServerSet::UpdateGeometry),
        );
        app.register_type::<WlSurface>();
        app.register_relation::<AttachmentRelationship>();
        app.register_relation::<ClientHasSurface>();
        app.register_relation::<SurfaceHasInputRegion>();
        app.register_relation::<SurfaceHasOpaqueRegion>();
    }
}
pub fn update_buffer_size(
    buffer_query: Query<
        (Option<&WlShmBuffer>, Option<&DmaBuffer>, &AttachedBy),
        Or<(
            Changed<WlShmBuffer>,
            Changed<DmaBuffer>,
            Changed<AttachedBy>,
        )>,
    >,
    mut surface_query: Query<&mut WlSurface>,
    mut assets: ResMut<Assets<Image>>,
) {
    for (shm_buffer, dma_buffer, attached_by) in buffer_query.iter() {
        let size = if let Some(shm_buffer) = shm_buffer {
            shm_buffer.size
        } else if let Some(dma_buffer) = dma_buffer {
            dma_buffer.size
        } else {
            unreachable!();
        };
        if let Some(mut surface) = attached_by
            .get()
            .and_then(|entity| surface_query.get_mut(entity).ok())
        {
            if surface.size != Some(size) {
                surface.resize(&mut assets, size);
            }
        }
    }
}
