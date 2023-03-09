use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use bevy::{prelude::*, utils::HashMap};
// use dway_server::log::logger;
use failure::format_err;
use smithay::{
    backend::{
        allocator::gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        drm::{DrmDeviceFd, GbmBufferedSurface},
        egl::{EGLContext, EGLDevice, EGLDisplay},
    },
    desktop::utils::OutputPresentationFeedback,
    reexports::{
        drm::{
            self,
            control::{connector, encoder, Device as drmDevice},
        },
        gbm::{self},
    },
    utils::Rectangle,
};

use crate::{device::Device, ecs::PhysicalRect, logger::logger};

#[derive(Component)]
pub struct OutputDisplay {}

pub type RenderSurface =
    GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, Option<OutputPresentationFeedback>>;
pub struct OutputSurfaceInner {
    pub raw: RenderSurface,
}
unsafe impl Send for OutputSurfaceInner {}
#[derive(Component)]
pub struct OutputSurface {
    pub inner: Arc<Mutex<OutputSurfaceInner>>,
    pub device_entity: Entity,
}

pub fn add_display_to_monitor(
    devices: Query<(Entity, &Device), Added<Device>>,
    mut commands: Commands,
) {
    // let monitor = MonitorBuilder::new()?.match_subsystem("drm")?.listen()?;
    // let logger = logger();
    for (device_entity, device) in devices.iter() {
        let logger = logger();
        let res_handles = device.drm.resource_handles().unwrap();
        let connector_infos: Vec<_> = res_handles
            .connectors()
            .iter()
            .map(|conn| device.drm.get_connector(*conn, true).unwrap())
            .filter(|conn| conn.state() == connector::State::Connected)
            .inspect(|conn| info!("drm connected: {:?}", conn.interface()))
            .collect();
        let gbm = device.gbm.lock().unwrap();

        let display: EGLDisplay = EGLDisplay::new(gbm.clone()).unwrap();
        let Some( render_node ) = EGLDevice::device_for_display(&display)
                .ok()
                .and_then(|x| x.try_get_render_node().ok().flatten())
            else{
                info!("failed to create display, gpu: {:?}", &device.path);
                continue;
            };
        let context = EGLContext::new(&display).unwrap();
        let formats = context.dmabuf_render_formats().clone();
        for connector_info in connector_infos {
            let mode = connector_info.modes()[0];
            for crtc in connector_info
                .encoders()
                .iter()
                .flat_map(|encoder_handle| device.drm.get_encoder(*encoder_handle))
                .flat_map(|encoder_info| res_handles.filter_crtcs(encoder_info.possible_crtcs()))
            {
                let surface: smithay::backend::drm::DrmSurface =
                    match device
                        .drm
                        .create_surface(crtc, mode, &[connector_info.handle()])
                    {
                        Ok(surface) => surface,
                        Err(err) => {
                            warn!("Failed to create drm surface: {}", err);
                            continue;
                        }
                    };

                let allocator = GbmAllocator::new(
                    gbm.clone(),
                    GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
                );
                let mut gbm_surface: RenderSurface =
                    match GbmBufferedSurface::new(surface, allocator, formats.clone()) {
                        Ok(renderer) => renderer,
                        Err(err) => {
                            warn!("Failed to create rendering surface: {}", err);
                            continue;
                        }
                    };
                gbm_surface.reset_buffers();
                let interface_short_name = match connector_info.interface() {
                    drm::control::connector::Interface::DVII => Cow::Borrowed("DVI-I"),
                    drm::control::connector::Interface::DVID => Cow::Borrowed("DVI-D"),
                    drm::control::connector::Interface::DVIA => Cow::Borrowed("DVI-A"),
                    drm::control::connector::Interface::SVideo => Cow::Borrowed("S-VIDEO"),
                    drm::control::connector::Interface::DisplayPort => Cow::Borrowed("DP"),
                    drm::control::connector::Interface::HDMIA => Cow::Borrowed("HDMI-A"),
                    drm::control::connector::Interface::HDMIB => Cow::Borrowed("HDMI-B"),
                    drm::control::connector::Interface::EmbeddedDisplayPort => Cow::Borrowed("eDP"),
                    other => Cow::Owned(format!("{:?}", other)),
                };
                let output_name =
                    format!("{}-{}", interface_short_name, connector_info.interface_id());
                let (width, height) = connector_info.size().unwrap_or((0, 0));
                let physical_rect =
                    Rectangle::from_loc_and_size((0, 0), (width as i32, height as i32));
                info!("add output {} at {:?}", &output_name, physical_rect);
                commands.spawn((
                    Name::new(output_name),
                    PhysicalRect(physical_rect),
                    OutputSurface {
                        device_entity,
                        inner: Arc::new(Mutex::new(OutputSurfaceInner { raw: gbm_surface })),
                    },
                ));
            }
        }
    }
}
