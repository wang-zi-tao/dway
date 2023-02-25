use bevy::{prelude::*, utils::HashMap};
use dway_server::log::logger;
use failure::format_err;
use smithay::{
    backend::{
        drm::{DrmDeviceFd, GbmBufferedSurface},
        egl::{EGLContext, EGLDevice, EGLDisplay},
    },
    desktop::utils::OutputPresentationFeedback,
    reexports::{
        drm::control::{connector, encoder, Device as drmDevice},
        gbm,
    },
};

use crate::device::Device;

#[derive(Component)]
pub struct OutputDisplay {}

pub type RenderSurface =
    GbmBufferedSurface<gbm::Device<DrmDeviceFd>, Option<OutputPresentationFeedback>>;

pub fn add_display_to_monitor(
    mut devices: Query<(Entity, &Device), Added<Device>>,
    mut commands: Commands,
) {
    // let monitor = MonitorBuilder::new()?.match_subsystem("drm")?.listen()?;
    let logger = logger();
    for (entity, device) in devices.iter() {
        let res_handles = device.drm.resource_handles().unwrap();
        let connector_infos: Vec<_> = res_handles
            .connectors()
            .iter()
            .map(|conn| device.drm.get_connector(*conn, true).unwrap())
            .filter(|conn| conn.state() == connector::State::Connected)
            .inspect(|conn| info!("drm connected: {:?}", conn.interface()))
            .collect();
        let gbm = device.gbm.lock().unwrap();

        let (render_node, formats) = {
            let display = EGLDisplay::new(gbm.clone(), logger.clone()).unwrap();
            let Some( node ) = EGLDevice::device_for_display(&display)
                .ok()
                .and_then(|x| x.try_get_render_node().ok().flatten())
            else{
                info!("failed to create display, gpu: {:?}", &device.path);
                continue;
            };
            let context = EGLContext::new(&display, logger.clone()).unwrap();
            (node, context.dmabuf_render_formats().clone())
        };
        for connector_info in connector_infos {
            let encoder_infos = connector_info
                .encoders()
                .iter()
                .flat_map(|encoder_handle| device.drm.get_encoder(*encoder_handle))
                .collect::<Vec<encoder::Info>>();
            let crtcs = encoder_infos
                .iter()
                .flat_map(|encoder_info| res_handles.filter_crtcs(encoder_info.possible_crtcs()));
            let mode = connector_info.modes()[0];
            for crtc in crtcs {
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
                let mut gbm_surface:RenderSurface = match GbmBufferedSurface::new(
                    surface,
                    gbm.clone(),
                    formats.clone(),
                    logger.clone(),
                ) {
                    Ok(renderer) => renderer,
                    Err(err) => {
                        warn!("Failed to create rendering surface: {}", err);
                        continue;
                    }
                };
                gbm_surface.reset_buffers();
                let (dmabuf, age) = gbm_surface.next_buffer().unwrap();
                // let display = EGLDisplay::new(gbm.clone(), logger.clone()).unwrap();
            }
        }
    }
}
