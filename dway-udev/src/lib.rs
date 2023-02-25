pub mod ecs;
pub mod surface;
pub mod device;
pub mod seat;
pub mod output;

use std::ffi::{c_uint, c_void};
use std::fs::OpenOptions;

use std::os::fd::IntoRawFd;
use std::time::{Duration, Instant};

use bevy::render::renderer::{RenderAdapter, RenderInstance};

use bevy::{app::AppExit, ecs::event::ManualEventReader, prelude::*, window::RequestRedraw};
use khronos_egl::{EGLClientBuffer, EGLContext, EGLDisplay};
use log::info;
use raw_window_handle::{DrmDisplayHandle, DrmWindowHandle};

use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::Session;
use smithay::backend::udev::primary_gpu;
use wgpu_hal::api::Gles;
use wgpu_hal::Api;

#[derive(Default)]
pub struct UDevBackendPlugin {}

impl Plugin for UDevBackendPlugin {
    fn build(&self, app: &mut App) {
        app.set_runner(main_loop);
    }
}
pub type EGLImage = *const c_void;
pub type EGLInt = i32;
pub type EGLEnum = c_uint;

pub fn main_loop(mut app: App) {
    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();
    let _redraw_event_reader = ManualEventReader::<RequestRedraw>::default();

    let (session, _notifier) = match LibSeatSession::new(None) {
        Ok(ret) => ret,
        Err(err) => {
            error!("Could not initialize a session: {}", err);
            return;
        }
    };
    let drm_path = primary_gpu(&session.seat())
        .ok()
        .flatten()
        .expect("No GPU!");
    // let drm_path = "/dev/dri/card0".to_string();
    let drm_fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(drm_path.clone())
        .unwrap()
        .into_raw_fd();
    info!("Using {:?} as primary gpu.", drm_path);
    let mut window_handle = DrmWindowHandle::empty();
    window_handle.plane = 0;
    let mut display_handle = DrmDisplayHandle::empty();
    display_handle.fd = drm_fd;
    // let raw_handle = RawHandleWrapper {
    //     window_handle: RawWindowHandle::Drm(window_handle),
    //     display_handle: RawDisplayHandle::Drm(display_handle),
    // };
    // let window = Window::new(
    //     WindowId::new(),
    //     &WindowDescriptor {
    //         width: 1920.0,
    //         height: 1080.0,
    //         position: WindowPosition::At((0.0, 0.0).into()),
    //         monitor: MonitorSelection::Primary,
    //         resize_constraints: bevy::window::WindowResizeConstraints {
    //             min_width: 0.0,
    //             min_height: 0.0,
    //             max_width: 1920.0,
    //             max_height: 1080.0,
    //         },
    //         scale_factor_override: None,
    //         title: "dway".into(),
    //         present_mode: bevy::window::PresentMode::Immediate,
    //         resizable: false,
    //         decorations: false,
    //         cursor_visible: true,
    //         cursor_grab_mode: bevy::window::CursorGrabMode::None,
    //         mode: WindowMode::Fullscreen,
    //         transparent: false,
    //         canvas: None,
    //         fit_canvas_to_parent: false,
    //         alpha_mode: bevy::window::CompositeAlphaMode::Auto,
    //     },
    //     1920,
    //     1080,
    //     1.0,
    //     None,
    //     Some(raw_handle),
    // );
    // let mut windows = app.world.resource_mut::<Windows>();
    // windows.add(window);

    // let render_instance = app.world.resource_mut::<RenderInstance>();
    let render_adapter = app.world.resource_mut::<RenderAdapter>();
    unsafe {
        let gl_adapter = render_adapter.as_hal::<Gles, _, _>(|adapter| {
            let adapter = adapter.unwrap();
            let egl_context = adapter.adapter_context();
            let egl = egl_context.egl_instance().unwrap();
            let _gl_display = egl_context.raw_display();
            let fn_bind_image: extern "system" fn(c_uint, *const c_void) = std::mem::transmute(
                egl.get_proc_address("glEGLImageTargetTexture2DOES")
                    .unwrap(),
            );
            let fn_bind_image = egl
                .get_proc_address("glEGLImageTargetRenderbufferStorageOES")
                .unwrap();
            let fn_import_image: extern "system" fn(
                EGLDisplay,
                EGLContext,
                EGLEnum,
                EGLClientBuffer,
                *const EGLInt,
            ) -> EGLImage = std::mem::transmute(egl.get_proc_address("eglCreateImageKHR").unwrap());
        });
        
        // let gl_instace: &<Gles as Api>::Instance =
        //     render_instance.as_hal::<Gles>().expect("need gles backend");
    }

    let _exit = false;
    let _redraw = false;
    loop {
        let start_time = Instant::now();

        if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
            if let Some(_exit) = app_exit_event_reader.iter(&app_exit_events).last() {
                // break;
            }
        }

        app.update();

        if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
            if let Some(_exit) = app_exit_event_reader.iter(&app_exit_events).last() {
                // break;
            }
        }

        let end_time = Instant::now();

        if Duration::from_secs_f64(1.0 / 60.0) > (end_time - start_time) {
            // std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0) - (end_time - start_time));
        }
    }
}
