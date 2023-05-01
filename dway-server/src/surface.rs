use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use crate::{
    components::{
        OutputWrapper, PhysicalRect, PopupWindow, SurfaceId, SurfaceOffset, WaylandWindow,
        WindowIndex, WindowMark, WindowScale, WlSurfaceWrapper, X11Window, XdgPopupWrapper,
    },
    egl::{gl_debug_message_callback, import_wl_surface},
    events::{CommitSurface, CreateTopLevelEvent, CreateWindow, CreateX11WindowEvent},
    wayland_window, DWay, DWayServerComponent,
};
use bevy::{
    core_pipeline::{clear_color::ClearColorConfig, core_2d::Transparent2d},
    ecs::system::lifetimeless::{Read, SRes, SResMut},
    log::Level,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraph, SlotInfo, SlotType},
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase},
        renderer::{RenderAdapter, RenderDevice, RenderQueue},
        texture::GpuImage,
        view::{ExtractedView, NonSendMarker, ViewTarget},
        Extract,
    },
    sprite::SpriteAssetEvents,
    ui::UiImageBindGroups,
    utils::{
        tracing::{self, span},
        HashSet,
    },
};
use failure::Fallible;
use glow::HasContext;
use smithay::{
    backend::renderer::{
        buffer_type,
        element::{
            default_primary_scanout_output_compare, Id, RenderElementPresentationState,
            RenderElementState, RenderElementStates,
        },
        utils::{on_commit_buffer_handler, with_renderer_surface_state, RendererSurfaceState},
        BufferType,
    },
    delegate_compositor, delegate_data_device, delegate_shm,
    desktop::{
        find_popup_root_surface,
        space::SpaceElement,
        utils::{
            send_frames_surface_tree, surface_primary_scanout_output,
            update_surface_primary_scanout_output, with_surfaces_surface_tree,
        },
        PopupKind, PopupManager,
    },
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction},
        wayland_protocols::xdg::decoration::{
            self as xdg_decoration,
            zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{
                wl_buffer::{self, WlBuffer},
                wl_data_device_manager::DndAction,
                wl_data_source::WlDataSource,
                wl_surface::{self, WlSurface},
            },
            Display, DisplayHandle, Resource,
        },
    },
    utils::{Logical, Physical, Point, Rectangle},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_states, with_surface_tree_downward,
            with_surface_tree_upward, CompositorHandler, SurfaceAttributes, TraversalAction,
        },
        data_device::{ClientDndGrabHandler, DataDeviceHandler, ServerDndGrabHandler},
        fractional_scale::with_fractional_scale,
        seat::WaylandFocus,
        shell::xdg::{
            XdgPopupSurfaceData, XdgPopupSurfaceRoleAttributes, XdgToplevelSurfaceRoleAttributes,
        },
        shm::{ShmHandler, ShmState},
    },
    xwayland::X11Wm,
};
use wgpu::{
    Extent3d, LoadOp, Operations, RenderPass, RenderPassDescriptor, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

#[derive(Component, Clone, Debug)]
pub struct ImportedSurface {
    pub texture: Handle<Image>,
    pub damages: Vec<Rectangle<i32, Physical>>,
    pub size: smithay::utils::Size<i32, Physical>,
    pub flush: Arc<AtomicBool>,
}
impl ImportedSurface {
    pub fn changed(&self) -> bool {
        !self.damages.is_empty() || self.flush.load(Ordering::Acquire)
    }
    pub fn reset(&mut self) {
        self.damages.clear();
        self.flush.store(false, Ordering::Release);
    }
}
impl ImportedSurface {
    fn texture_descriptor<'a>(image_size: Extent3d) -> TextureDescriptor<'a> {
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

    pub fn new(assets: &mut Assets<Image>, size: smithay::utils::Size<i32, Physical>) -> Self {
        let image_size = Extent3d {
            width: size.w as u32,
            height: size.h as u32,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: Self::texture_descriptor(image_size),
            ..default()
        };
        image.resize(image_size);
        Self {
            size,
            texture: assets.add(image),
            damages: Default::default(),
            flush: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn resize(
        &mut self,
        assets: &mut Assets<Image>,
        size: smithay::utils::Size<i32, Physical>,
    ) {
        let image_size = Extent3d {
            width: size.w as u32,
            height: size.h as u32,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: Self::texture_descriptor(image_size),
            ..default()
        };
        dbg!("resize", image_size);
        image.resize(image_size);
        let _ = assets.set(self.texture.clone(), image);
        self.size = size;
        self.flush.store(true, Ordering::Release);
    }
}

#[tracing::instrument(skip_all)]
pub fn create_surface(
    mut events: EventReader<CreateWindow>,
    window_index: Res<WindowIndex>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    for CreateWindow(new_surface) in events.iter() {
        if let Some(entity) = window_index.get(&new_surface) {
            let imported = ImportedSurface::new(&mut images, (512, 512).into());
            info!(
                surface=?new_surface,
                ?entity,
                texture=?&imported.texture,
                "create surface.",
            );
            commands.entity(*entity).insert(imported);
        } else {
            error!(surface=?new_surface,"window index not found");
        }
    }
}
#[tracing::instrument(skip_all)]
pub fn do_commit(
    _: Option<NonSend<bevy::core::NonSendMarker>>,
    mut events: EventReader<CommitSurface>,
    mut surface_query: Query<(
        Entity,
        Option<&mut WlSurfaceWrapper>,
        &mut ImportedSurface,
        Option<&mut WaylandWindow>,
        Option<&mut PopupWindow>,
        Option<&mut X11Window>,
        Option<&WindowScale>,
        Option<&mut PhysicalRect>,
        Option<&mut SurfaceOffset>,
    )>,
    window_index: Res<WindowIndex>,
    mut assets: ResMut<Assets<Image>>,
) {
    let mut toplevels = HashSet::new();
    for CommitSurface { surface: id } in events.iter() {
        if let Some((
            entity,
            mut wl_surface_wrapper,
            mut imported_surface,
            window,
            popup,
            x11_window,
            window_scale,
            mut physical_rect,
            mut surface_offset,
        )) = window_index.query_mut(id, &mut surface_query)
        {
            let scale = window_scale.cloned().unwrap_or_default().0;
            imported_surface.flush.store(true, Ordering::Release);
            if let (Some(window), Some(surface)) = (window, wl_surface_wrapper.as_ref()) {
                let initial_configure_sent =
                    with_states_locked(surface, |s: &mut XdgToplevelSurfaceRoleAttributes| {
                        s.initial_configure_sent
                    });
                if !initial_configure_sent {
                    window.toplevel().send_configure();
                }

                window.on_commit();
                let geo = window.geometry();
                let bbox = window.bbox();
                let scale = window_scale.cloned().unwrap_or_default().0;
                let offset = bbox.loc - geo.loc;
                surface_offset.as_mut().map(|r| {
                    let value = offset
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                    if r.0.loc != value {
                        r.0.loc = value;
                    }
                });
                //physical_rect.as_mut().map(|r| {
                //    let value = geo
                //        .size
                //        .to_f64()
                //        .to_physical_precise_round(scale)
                //        .to_i32_round();
                //    if r.0.size != value {
                //        r.0.size = value;
                //    }
                //});
            } else if let Some(window) = x11_window {
                //let geo = window.geometry();
                //let bbox = window.bbox();
                //surface_offset.as_mut().map(|r| {
                //    let value = Rectangle::from_loc_and_size(
                //        (0, 0),
                //        bbox.size
                //            .to_f64()
                //            .to_physical_precise_round(scale)
                //            .to_i32_round(),
                //    );
                //    if value != r.0 {
                //        r.0 = value;
                //    }
                //});
                //physical_rect.as_mut().map(|r| {
                //    let value = geo
                //        .size
                //        .to_f64()
                //        .to_physical_precise_round(scale)
                //        .to_i32_round();
                //    if value != r.0.size {
                //        r.0.size = value;
                //    }
                //});
            } else if let (Some(popup), Some(surface)) = (popup, wl_surface_wrapper.as_ref()) {
                if !with_states_locked(surface, |s: &mut XdgPopupSurfaceRoleAttributes| {
                    s.initial_configure_sent
                }) {
                    let PopupKind::Xdg(ref xdg_popup) = &popup.kind;
                    if let Err(error) = xdg_popup.send_configure() {
                        error!(surface = id.to_string(), %error, "initial configure failed");
                    };
                    trace!(surface=?SurfaceId::from(&surface.0),"send configuring");
                }
                if !is_sync_subsurface(surface) {
                    let mut root = surface.0.clone();
                    while let Some(parent) = get_parent(&root) {
                        root = parent;
                    }
                    toplevels.insert(SurfaceId::from(&root));
                }
                if let Some(surface_offset) = surface_offset.as_mut() {
                    let value = Point::default()
                        - popup
                            .kind
                            .geometry()
                            .loc
                            .to_f64()
                            .to_physical(scale)
                            .to_i32_round();
                    if value != surface_offset.loc {
                        surface_offset.loc = value;
                    }
                }
                physical_rect.as_mut().map(|r| {
                    let value = popup
                        .position
                        .get_geometry()
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                    if value != r.0 {
                        r.0 = value;
                    }
                });
            };
            if let Some(surface) = wl_surface_wrapper.as_ref() {
                with_states(surface, |s| {
                    let Some( state )=s.data_map.get::<RefCell<RendererSurfaceState>>().map(|c|c.borrow_mut())else{
                    error!(?entity,surface=?id,thread=?thread::current().id(),"RendererSurfaceState not found in surface.");
                    return
                };
                    let Some(surface_size)=state.buffer_size()else{
                    error!(?entity,surface=?id,thread=?thread::current().id(),"buffer not found in surface.");
                    return
                };
                    let scale = window_scale.cloned().unwrap_or_default().0;
                    let value = surface_size.to_f64().to_physical(scale).to_i32_round();
                    if let Some(surface_offset) = surface_offset.as_mut() {
                        if value != surface_offset.size {
                            surface_offset.size = value;
                        }
                    }
                    if let Some(physical_rect) = physical_rect.as_mut() {
                        if value != physical_rect.size {
                            physical_rect.size = value;
                        }
                    }
                    let physical_size = surface_size.to_f64().to_physical(scale).to_i32_round();
                    if physical_size != imported_surface.size {
                        imported_surface.resize(&mut assets, physical_size);
                    }
                });
            }
            trace!(surface = id.to_string(), ?entity, "commit finish");
        }
    }
    for (
        entity,
        mut wl_surface_wrapper,
        mut imported_surface,
        window,
        popup,
        x11_window,
        window_scale,
        mut physical_rect,
        mut surface_offset,
    ) in surface_query.iter()
    {
        if let (Some(window), Some(surface)) = (window, wl_surface_wrapper) {
            if toplevels.contains(&SurfaceId::from(surface)) {
                window.on_commit();
                trace!(surface=?SurfaceId::from(surface),"toplevel commit")
            }
        }
    }
}

pub fn change_size(
    mut query: Query<(
        Option<&WaylandWindow>,
        Option<&X11Window>,
        Option<&PopupWindow>,
        Option<&WindowScale>,
        &mut ImportedSurface,
    )>,
    mut assets: ResMut<Assets<Image>>,
) {
    for (wayland_window, x11_window, popup_window, scale, mut imported) in query.iter_mut() {
        let bbox = if let Some(WaylandWindow(w)) = wayland_window {
            w.bbox().size
        } else if let Some(X11Window(w)) = x11_window {
            w.bbox().size
        } else if let Some(PopupWindow { kind, position }) = popup_window {
            position.rect_size
        } else {
            continue;
        };
        // let size = bbox.to_physical_precise_round(scale.cloned().unwrap_or_default().0);
        // if size != Default::default() && size != imported.size {
        //     info!("resize {:?} => {:?}", imported.size, size);
        //     imported.size = (size.w, size.h).into();
        //     // imported.resize(&mut assets, (size.w, size.h).into());
        // }
    }
}

delegate_compositor!(DWay);
impl CompositorHandler for DWay {
    fn compositor_state(&mut self) -> &mut smithay::wayland::compositor::CompositorState {
        &mut self.compositor
    }

    fn commit(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        trace!(surface=?SurfaceId::from(surface),thread=?thread::current().id(),"commit");
        X11Wm::commit_hook::<DWayServerComponent>(surface);
        on_commit_buffer_handler(&surface);
        let (buffer, buffer_size) = with_states(&surface, |states| {
            let render_state = states
                .data_map
                .get::<RefCell<RendererSurfaceState>>()
                .unwrap()
                .borrow();
            (render_state.buffer().cloned(), render_state.buffer_size())
        });

        trace!(surface=?SurfaceId::from(surface),"commit {:?}",buffer);

        dbg!(SurfaceId::from(surface));
        dbg!(smithay::wayland::compositor::get_role(surface));
        dbg!(get_parent(surface));

        self.send_ecs_event(CommitSurface {
            surface: surface.into(),
        });
    }
}

pub fn try_with_states_locked<F, T, C>(surface: &WlSurface, f: F) -> Option<T>
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        states
            .data_map
            .get::<Mutex<C>>()
            .and_then(|l| l.lock().ok())
            .map(|mut l| f(&mut l))
    })
}
pub fn try_with_states_borrowed<F, T, C>(surface: &WlSurface, f: F) -> Option<T>
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        states
            .data_map
            .get::<RefCell<C>>()
            .map(|l| l.borrow_mut())
            .map(|mut l| f(&mut l))
    })
}
pub fn with_states_locked<F, T, C>(surface: &WlSurface, f: F) -> T
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        let mut state = get_component_locked(states);
        f(&mut state)
    })
}
pub fn with_states_borrowed<F, T, C>(surface: &WlSurface, f: F) -> T
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        let mut state = get_component_borrowed(states);
        f(&mut state)
    })
}
pub fn get_component_borrowed<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> RefMut<C> {
    states.data_map.get::<RefCell<C>>().unwrap().borrow_mut()
}
pub fn get_component_locked<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> MutexGuard<C> {
    states.data_map.get::<Mutex<C>>().unwrap().lock().unwrap()
}
pub fn try_get_component_locked<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> Option<MutexGuard<C>> {
    states
        .data_map
        .get::<Mutex<C>>()
        .and_then(|l| l.lock().ok())
}
pub fn try_get_component_borrowed<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> Option<RefMut<C>> {
    states.data_map.get::<RefCell<C>>().map(|l| l.borrow_mut())
}

delegate_shm!(DWay);
impl ShmHandler for DWay {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
impl BufferHandler for DWay {
    fn buffer_destroyed(&mut self, buffer: &WlBuffer) {
        info!(buffer=?buffer.id(),"buffer destroyed");
    }
}

delegate_data_device!(DWay);
impl DataDeviceHandler for DWay {
    fn data_device_state(&self) -> &smithay::wayland::data_device::DataDeviceState {
        &self.data_device_state
    }

    fn action_choice(&mut self, available: DndAction, preferred: DndAction) -> DndAction {
        smithay::wayland::data_device::default_action_chooser(available, preferred)
    }

    fn new_selection(&mut self, source: Option<WlDataSource>, seat: smithay::input::Seat<Self>) {}

    fn send_selection(
        &mut self,
        mime_type: String,
        fd: std::os::fd::OwnedFd,
        seat: smithay::input::Seat<Self>,
    ) {
    }
}
impl ClientDndGrabHandler for DWay {
    fn started(
        &mut self,
        source: Option<WlDataSource>,
        icon: Option<WlSurface>,
        seat: smithay::input::Seat<Self>,
    ) {
    }

    fn dropped(&mut self, seat: smithay::input::Seat<Self>) {}
}
impl ServerDndGrabHandler for DWay {
    fn action(&mut self, action: DndAction, seat: smithay::input::Seat<Self>) {}

    fn dropped(&mut self, seat: smithay::input::Seat<Self>) {}

    fn cancelled(&mut self, seat: smithay::input::Seat<Self>) {}

    fn send(
        &mut self,
        mime_type: String,
        fd: std::os::fd::OwnedFd,
        seat: smithay::input::Seat<Self>,
    ) {
    }

    fn finished(&mut self, seat: smithay::input::Seat<Self>) {}
}

pub struct ImportedSurfacePhaseItem {
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}
impl PhaseItem for ImportedSurfacePhaseItem {
    type SortKey = u32;

    fn entity(&self) -> bevy::prelude::Entity {
        self.entity
    }

    fn sort_key(&self) -> Self::SortKey {
        1
    }

    fn draw_function(&self) -> bevy::render::render_phase::DrawFunctionId {
        self.draw_function
    }
}
#[derive(Component)]
pub struct ExtractedBuffer {
    pub buffer: smithay::backend::renderer::utils::Buffer,
}

#[tracing::instrument(skip_all)]
pub fn extract_surface(
    _: NonSend<NonSendMarker>,
    surface_query: Extract<Query<(Entity, &WlSurfaceWrapper, &ImportedSurface)>>,
    mut feedback: ResMut<ImportSurfaceFeedback>,
    mut commands: Commands,
) {
    let mut values = Vec::new();
    feedback.surfaces.clear();
    feedback.render_state.lock().unwrap().states.clear();
    for (entity, surface, imported) in surface_query.iter() {
        feedback.surfaces.push(surface.0.clone());
        if imported.flush.load(Ordering::Acquire) {
            with_states(surface, |s| {
                let Some( state )=s.data_map.get::<RefCell<RendererSurfaceState>>().map(|c|c.borrow_mut())else{
                    error!(?entity,surface=?SurfaceId::from(surface),thread=?thread::current().id(),"RendererSurfaceState not found in surface.");
                    return
                };
                let Some(buffer)=state.buffer()else{
                    error!(?entity,surface=?SurfaceId::from(surface),thread=?thread::current().id(),"buffer not found in surface.");
                    return
                };
                values.push((
                    entity,
                    (
                        imported.clone(),
                        ExtractedBuffer {
                            buffer: buffer.clone(),
                        },
                        surface.clone(),
                    ),
                ));
            });
        }
    }
    commands.insert_or_spawn_batch(values);
    commands.spawn(RenderPhase::<ImportedSurfacePhaseItem>::default());
    feedback.render_state.lock().unwrap().states.clear();
}

pub fn queue_import(
    draw_functions: Res<DrawFunctions<ImportedSurfacePhaseItem>>,
    mut phase_query: Query<&mut RenderPhase<ImportedSurfacePhaseItem>>,
    surface_query: Query<Entity, (With<ExtractedBuffer>, With<ImportedSurface>)>,
) {
    let function = draw_functions.read().id::<ImportSurface>();
    let mut phase = phase_query.single_mut();
    for entity in &surface_query {
        phase.add(ImportedSurfacePhaseItem {
            draw_function: function,
            entity,
        });
    }
}

#[derive(Debug, Resource)]
pub struct ImportSurfaceFeedback {
    pub render_state: Mutex<RenderElementStates>,
    pub surfaces: Vec<WlSurface>,
    pub output: Output,
}
impl ImportSurfaceFeedback {
    pub fn send_frame(&self, time: &Time) {
        let render_state = self.render_state.lock().unwrap();
        for surface in self.surfaces.iter() {
            with_surfaces_surface_tree(&surface, |surface, states| {
                if let Some(output) = update_surface_primary_scanout_output(
                    surface,
                    &self.output,
                    states,
                    &render_state,
                    default_primary_scanout_output_compare,
                ) {
                    with_fractional_scale(states, |fraction_scale| {
                        fraction_scale
                            .set_preferred_scale(output.current_scale().fractional_scale());
                    });
                }
            });
            send_frames_surface_tree(
                &surface,
                &self.output,
                time.elapsed(),
                None,
                surface_primary_scanout_output,
            );
        }
    }
}

impl Default for ImportSurfaceFeedback {
    fn default() -> Self {
        let output = Output::new(
            "output".to_string(),
            PhysicalProperties {
                size: (i32::MAX, i32::MAX).into(),
                subpixel: Subpixel::Unknown,
                make: "output".into(),
                model: "output".into(),
            },
        );
        Self {
            render_state: Mutex::new(RenderElementStates {
                states: HashMap::new(),
            }),
            surfaces: Default::default(),
            output,
        }
    }
}

pub struct ImportSurface;
impl<P: PhaseItem> RenderCommand<P> for ImportSurface {
    type Param = (
        SRes<RenderDevice>,
        SRes<ImportSurfaceFeedback>,
        SRes<RenderAssets<Image>>,
    );
    type ItemWorldQuery = (
        Read<ExtractedBuffer>,
        Read<WlSurfaceWrapper>,
        Read<ImportedSurface>,
    );
    type ViewWorldQuery = ();

    #[tracing::instrument(skip_all)]
    fn render<'w>(
        item: &P,
        view: bevy::ecs::query::ROQueryItem<'w, Self::ViewWorldQuery>,
        (buffer, surface, imported): bevy::ecs::query::ROQueryItem<'w, Self::ItemWorldQuery>,
        (render_device, feedback, textures): bevy::ecs::system::SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        let success = if imported.flush.load(Ordering::Acquire) {
            let texture = textures.get(&imported.texture).unwrap();
            match import_wl_surface(
                &buffer.buffer,
                &texture.texture,
                &imported.damages,
                render_device.wgpu_device(),
            ) {
                Ok(_) => true,
                Err(e) => {
                    error!(
                        surface = ?surface.id(),
                        error = %e,
                        entity=?item.entity(),
                        "failed to import surface.",
                    );
                    false
                }
            }
        } else {
            false
        };
        feedback.render_state.lock().unwrap().states.insert(
            Id::from_wayland_resource(&surface.0),
            if imported.flush.load(Ordering::Acquire) {
                RenderElementState {
                    visible_area: (imported.size.w * imported.size.h) as usize,
                    presentation_state: RenderElementPresentationState::Rendering { reason: None },
                }
            } else {
                RenderElementState {
                    visible_area: 0,
                    presentation_state: RenderElementPresentationState::Skipped,
                }
            },
        );
        if success {
            trace!(
                surface = ?surface.id(),
                image = ?&imported.texture,
                "import surface",
            );
            imported.flush.store(false, Ordering::Release);
        }
        bevy::render::render_phase::RenderCommandResult::Success
    }
}

pub struct ImportSurfacePassNode {
    query: QueryState<(Entity, &'static RenderPhase<ImportedSurfacePhaseItem>)>,
    view_query: QueryState<
        (
            &'static ExtractedCamera,
            &'static ViewTarget,
            &'static Camera2d,
        ),
        With<ExtractedView>,
    >,
}
impl ImportSurfacePassNode {
    pub const IN_VIEW: &'static str = "view";
    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query(),
            view_query: world.query_filtered(),
        }
    }
}
impl Node for ImportSurfacePassNode {
    fn run(
        &self,
        graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, target, camera_2d) =
            if let Ok(result) = self.view_query.get_manual(world, view_entity) {
                result
            } else {
                return Ok(());
            };
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("main_pass_2d"),
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: match camera_2d.clear_color {
                        ClearColorConfig::Default => {
                            LoadOp::Clear(world.resource::<ClearColor>().0.into())
                        }
                        ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                        ClearColorConfig::None => LoadOp::Load,
                    },
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            for (entity, phase) in self.query.iter_manual(world) {
                phase.render(&mut render_pass, world, entity);
            }
        }
        let time = world.resource::<Time>();
        let feedback = world.resource::<ImportSurfaceFeedback>();
        feedback.send_frame(time);
        Ok(())
    }

    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut bevy::prelude::World) {
        self.query.update_archetypes(world);
        self.view_query.update_archetypes(world);
    }
}
pub mod node {
    pub const NAME: &'static str = "import_wayland_surface";
}
