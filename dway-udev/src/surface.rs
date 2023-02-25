use std::ffi::{c_uint, c_void};
use std::fs::OpenOptions;

use std::os::fd::IntoRawFd;
use std::time::{Duration, Instant};

use bevy::render::renderer::{RenderAdapter, RenderInstance};

use bevy::{app::AppExit, ecs::event::ManualEventReader, prelude::*, window::RequestRedraw};
use failure::{format_err, Fallible};
use khronos_egl::{EGLClientBuffer, EGLContext, EGLDisplay};
use log::info;
use raw_window_handle::{DrmDisplayHandle, DrmWindowHandle};

use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::Session;
use smithay::backend::udev::primary_gpu;
use smithay::reexports::nix::fcntl::OFlag;
use wgpu_hal::api::Gles;
use wgpu_hal::Api;

use crate::device::Device;
use crate::{EGLEnum, EGLImage, EGLInt};

#[derive(Default)]
pub struct UDevBackendPlugin {}

#[derive(Component)]
pub struct Surface{

}

pub fn scan_outputs(render_adapter: Mut<RenderAdapter>, mut devices: Query<(&mut Device)>) {
    for device in devices.iter_mut() {
        unsafe {
            let result = render_adapter.as_hal::<Gles, _, _>(|adapter| {
                let adapter = adapter.ok_or_else(|| format_err!("failed to lock"))?;
                let egl_context = adapter.adapter_context();
                let egl = egl_context
                    .egl_instance()
                    .ok_or_else(|| format_err!("render adapter is not egl"))?;
                let _gl_display = egl_context.raw_display();
                let fn_bind_image: extern "system" fn(c_uint, *const c_void) = std::mem::transmute(
                    egl.get_proc_address("glEGLImageTargetTexture2DOES")
                        .ok_or_else(|| {
                            format_err!("failed to get function glEGLImageTargetTexture2DOES")
                        })?,
                );
                let fn_bind_image = egl
                    .get_proc_address("glEGLImageTargetRenderbufferStorageOES")
                    .ok_or_else(|| {
                        format_err!("failed to get function glEGLImageTargetRenderbufferStorageOES")
                    })?;
                let fn_import_image: extern "system" fn(
                    EGLDisplay,
                    EGLContext,
                    EGLEnum,
                    EGLClientBuffer,
                    *const EGLInt,
                ) -> EGLImage = std::mem::transmute(
                    egl.get_proc_address("eglCreateImageKHR")
                        .ok_or_else(|| format_err!("failed to get function eglCreateImageKHR"))?,
                );
                Fallible::Ok(())
            });
            if let Err(e) = result {
                error!("error while scan outputs: {e:?}");
            }

            // let gl_instace: &<Gles as Api>::Instance =
            //     render_instance.as_hal::<Gles>().expect("need gles backend");
        }
    }
}
