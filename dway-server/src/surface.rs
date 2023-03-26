use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, MutexGuard,
    },
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
    log::Level,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        renderer::{RenderAdapter, RenderDevice},
        texture::GpuImage,
        view::NonSendMarker,
        Extract,
    },
    sprite::SpriteAssetEvents,
    ui::UiImageBindGroups,
    utils::tracing::{self, span},
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
        gles2::Gles2Renderer,
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
            CompositorHandler, SurfaceAttributes, TraversalAction,
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
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

#[derive(Component, Debug)]
pub struct ImportedSurface {
    pub texture: Handle<Image>,
    pub damages: Vec<Rectangle<i32, Physical>>,
    pub size: smithay::utils::Size<i32, Physical>,
    pub flush: AtomicBool,
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
    pub fn new(assets: &mut Assets<Image>, size: smithay::utils::Size<i32, Physical>) -> Self {
        let image_size = Extent3d {
            width: size.w as u32,
            height: size.h as u32,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size: image_size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(image_size);
        Self {
            size,
            texture: assets.add(image),
            damages: Default::default(),
            flush: true.into(),
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
            texture_descriptor: TextureDescriptor {
                label: None,
                size: image_size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
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
) {
    for CommitSurface {
        surface: id,
        surface_size,
    } in events.iter()
    {
        if let Some((
            entity,
            mut wl_surface_wrapper,
            imported_surface,
            window,
            popup,
            x11_window,
            window_scale,
            mut physical_rect,
            mut surface_offset,
        )) = window_index.query_mut(id, &mut surface_query)
        {
            let surface = &mut wl_surface_wrapper;
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
                    r.0.loc = offset
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                });
                physical_rect.as_mut().map(|r| {
                    r.0.size = geo
                        .size
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                });
            } else if let Some(window) = x11_window {
                let geo = window.geometry();
                let bbox = window.bbox();
                let scale = window_scale.cloned().unwrap_or_default().0;
                surface_offset.as_mut().map(|r| {
                    r.0.loc = Default::default();
                    r.0.size = bbox
                        .size
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                });
                physical_rect.as_mut().map(|r| {
                    r.0.size = geo
                        .size
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                });
            } else if let (Some(popup), Some(surface)) = (popup, wl_surface_wrapper.as_ref()) {
                if !with_states_locked(surface, |s: &mut XdgPopupSurfaceRoleAttributes| {
                    s.initial_configure_sent
                }) {
                    let PopupKind::Xdg(ref xdg_popup) = &popup.kind;
                    if let Err(error) = xdg_popup.send_configure() {
                        error!(surface = id.to_string(), %error, "initial configure failed");
                    };
                }
                let scale = window_scale.cloned().unwrap_or_default().0;
                if let Some(surface_offset) = surface_offset.as_mut() {
                    surface_offset.loc = Point::default()
                        - popup
                            .kind
                            .geometry()
                            .loc
                            .to_f64()
                            .to_physical(scale)
                            .to_i32_round();
                }
                physical_rect.as_mut().map(|r| {
                    r.0 = popup
                        .position
                        .get_geometry()
                        .to_f64()
                        .to_physical_precise_round(scale)
                        .to_i32_round();
                });
            };
            if let Some(mut surface_offset) = surface_offset {
                if let Some(surface_size) = surface_size {
                    let scale = window_scale.cloned().unwrap_or_default().0;
                    surface_offset.size = surface_size.to_f64().to_physical(scale).to_i32_round();
                } else {
                    error!("no surface size");
                }
            }
            trace!(surface = id.to_string(), ?entity, "commit finish");
        }
    }
}
#[tracing::instrument(skip_all)]
pub fn import_surface(
    _: NonSend<NonSendMarker>,
    time: Extract<Res<Time>>,
    query: Extract<Query<(Entity, &WlSurfaceWrapper, &ImportedSurface)>>,
    output_query: Extract<Query<&OutputWrapper>>,
    window_quert: Extract<
        Query<
            (
                Entity,
                &SurfaceId,
                Option<&WaylandWindow>,
                Option<&X11Window>,
            ),
            (With<WindowMark>, With<WlSurfaceWrapper>),
        >,
    >,
    render_device: Res<RenderDevice>,
    mut render_images: ResMut<RenderAssets<Image>>,
    mut events: ResMut<SpriteAssetEvents>,
) {
    let output = output_query.single();
    let mut render_states = RenderElementStates {
        states: HashMap::new(),
    };
    for (entity, surface, imported) in query.iter() {
        let gpu_image: Option<GpuImage> = imported.flush.load(Ordering::Acquire).then_some(()).and_then(|()|{
            with_states(surface, |s| {
                let Some( mut surface_state )=s.data_map.get::<RefCell<RendererSurfaceState>>().map(|c|c.borrow_mut())else{
                    error!(
                        surface = ?surface.id(),
                        ?entity,
                        "RendererSurfaceState not found in surface.",
                    );
                    return None
                } ;
                match import_wl_surface(
                    &mut surface_state,
                    &imported.damages,
                    &render_device.wgpu_device(),
                ) {
                    Ok(o) => Some(o),
                    Err(e) => {
                        error!(
                            surface = ?surface.id(),
                            ?entity,
                            error = %e,
                            "failed to import surface.",
                        );
                        None
                    }
                }
            })
        }) ;
        render_states.states.insert(
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
        if let Some(gpu_image) = gpu_image {
            trace!(
                surface = ?surface.id(),
                gpu_image = ?gpu_image.texture.id(),
                image = ?&imported.texture,
                "import surface",
            );
            events.images.push(AssetEvent::Modified {
                handle: imported.texture.clone(),
            });
            render_images.insert(imported.texture.clone(), gpu_image);
            imported.flush.store(false, Ordering::Release);
        }
    }
    unsafe {
        render_device
            .wgpu_device()
            .as_hal::<wgpu_hal::api::Gles, _, _>(|hal_device| {
                if let Some(hal_device) = hal_device {
                    let gl: &glow::Context = &hal_device.context().lock();
                    gl.flush();
                    gl.finish();
                }
            });
    }
    for (entity, id, wayland_window, x11_window) in window_quert.iter() {
        if let Some(window) = wayland_window {
            window.with_surfaces(|surface, states| {
                if let Some(output) = update_surface_primary_scanout_output(
                    surface,
                    &output,
                    states,
                    &render_states,
                    default_primary_scanout_output_compare,
                ) {
                    with_fractional_scale(states, |fraction_scale| {
                        fraction_scale
                            .set_preferred_scale(output.current_scale().fractional_scale());
                    });
                }
            });
            window.send_frame(
                &output,
                time.elapsed(),
                None,
                surface_primary_scanout_output,
            );
        } else if let Some(surface) = x11_window.and_then(|w| w.wl_surface()) {
            with_surfaces_surface_tree(&surface, |surface, states| {
                if let Some(output) = update_surface_primary_scanout_output(
                    surface,
                    &output,
                    states,
                    &render_states,
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
                &output,
                time.elapsed(),
                None,
                surface_primary_scanout_output,
            );
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
        trace!(surface=?SurfaceId::from(surface),"commit");
        X11Wm::commit_hook::<DWayServerComponent>(surface);
        on_commit_buffer_handler(&surface);
        let surface_size = with_states(&surface, |states| {
            let surface_size = states
                .data_map
                .get::<RefCell<RendererSurfaceState>>()
                .and_then(|r| r.borrow().surface_size());
            surface_size
        });
        self.send_ecs_event(CommitSurface {
            surface: surface.into(),
            surface_size,
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

    fn new_selection(&mut self, source: Option<WlDataSource>) {}

    fn send_selection(&mut self, mime_type: String, fd: std::os::fd::OwnedFd) {}
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
    fn action(&mut self, action: DndAction) {}

    fn dropped(&mut self) {}

    fn cancelled(&mut self) {}

    fn send(&mut self, mime_type: String, fd: std::os::fd::OwnedFd) {}

    fn finished(&mut self) {}
}
