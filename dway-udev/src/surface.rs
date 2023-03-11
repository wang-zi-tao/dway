use std::ffi::{c_int, c_uint, c_void};
use std::fs::OpenOptions;

use std::num::NonZeroU32;
use std::os::fd::{AsRawFd, IntoRawFd};
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{Sampler, Texture};
use bevy::render::renderer::{RenderAdapter, RenderDevice, RenderInstance};

use bevy::render::texture::GpuImage;
use bevy::{app::AppExit, ecs::event::ManualEventReader, prelude::*, window::RequestRedraw};
use dway_server::egl::{image_target_renderbuffer_storage_oes, import_dma};
use failure::{format_err, Fallible};
use glow::HasContext;
use khronos_egl::{Config, EGLClientBuffer, EGLContext, EGLDisplay, EGLImage, Enum, Int};
use log::info;
use raw_window_handle::{DrmDisplayHandle, DrmWindowHandle};

use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::Buffer;
// use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::Session;
use smithay::backend::udev::primary_gpu;
use smithay::reexports::nix::fcntl::OFlag;
use wgpu::{
    DeviceDescriptor, Features, Limits, SamplerDescriptor, TextureAspect, TextureDescriptor,
    TextureUsages,
};
use wgpu_hal::api::Gles;
use wgpu_hal::{Api, MemoryFlags, TextureUses};

use crate::device::Device;
use crate::ecs::PhysicalRect;
use crate::output::{OutputDisplay, OutputSurface};

#[derive(Default)]
pub struct UDevBackendPlugin {}

#[derive(Component)]
pub struct Surface {}

pub fn generate_surface_image(
    render_adapter: Res<RenderAdapter>,
    render_devices: Res<RenderDevice>,
    mut outputs: Query<(Entity, &OutputSurface, &PhysicalRect)>,
    commands: Commands,
    // mut devices: Query<(&mut Device)>,
) {
    for (output_entity, output, rect) in outputs.iter_mut() {
        let output_inner = &mut output.inner.lock().unwrap().raw;
        // render_adapter.request_device(&DeviceDescriptor {
        //         label: None,
        //         features: Features::all(),
        //         limits: Limits::default(),
        //     }, None);
        unsafe {
            let result = render_adapter.as_hal::<Gles, _, _>(|adapter| {
                let device = render_devices.wgpu_device();
                device.as_hal::<Gles, _, _>(|hal_device| {
                    let hal_device =
                        hal_device.ok_or_else(|| format_err!("failed to get device"))?;
                    let gl: &glow::Context = &hal_device.context().lock();

                    let adapter = adapter.ok_or_else(|| format_err!("failed to lock"))?;

                    let egl_context = adapter.adapter_context();
                    let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
                        .egl_instance()
                        .ok_or_else(|| format_err!("render adapter is not egl"))?;

                    let egl_create_image_khr: extern "system" fn(
                        EGLDisplay,
                        EGLContext,
                        Enum,
                        EGLClientBuffer,
                        *const Int,
                    ) -> EGLImage =
                        std::mem::transmute(egl.get_proc_address("eglCreateImageKHR").ok_or_else(
                            || format_err!("failed to get function eglCreateImageKHR"),
                        )?);
                    let fn_bind_image: extern "system" fn(c_uint, *const c_void) =
                        std::mem::transmute(
                            egl.get_proc_address("glEGLImageTargetTexture2DOES")
                                .ok_or_else(|| {
                                    format_err!(
                                        "failed to get function glEGLImageTargetTexture2DOES"
                                    )
                                })?,
                        );
                    let fn_bind_image_storage: extern "system" fn(
                        target: Enum,
                        image: *const c_void,
                    ) = std::mem::transmute(
                        egl.get_proc_address("glEGLImageTargetRenderbufferStorageOES")
                            .ok_or_else(|| {
                                format_err!(
                                    "failed to get function glEGLImageTargetRenderbufferStorageOES"
                                )
                            })?,
                    );

                    let display = egl_context
                        .raw_display()
                        .ok_or_else(|| format_err!("failed to get egl display"))?;
                    let (dma_buffer, age) = output_inner.next_buffer()?;
                    trace!("get dma buffer {dma_buffer:?}, age:{age}");
                    let (raw_image, size) =
                        import_dma(dma_buffer, egl_create_image_khr, display.as_ptr())?;
                    let rbo = gl
                        .create_renderbuffer()
                        .map_err(|e| format_err!("failed to create render buffer :{e}"))?;
                    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rbo));
                    image_target_renderbuffer_storage_oes(fn_bind_image_storage, raw_image)?;
                    // gl.bind_renderbuffer(glow::RENDERBUFFER, None);
                    let texture_format = wgpu::TextureFormat::Rgba8Snorm;

                    let hal_texture: <Gles as Api>::Texture = hal_device.texture_from_raw(
                        rbo.0,
                        &wgpu_hal::TextureDescriptor {
                            label: None,
                            size: wgpu::Extent3d {
                                width: rect.width(),
                                height: rect.height(),
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 0,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: texture_format,
                            memory_flags: MemoryFlags::empty(),
                            usage: TextureUses::COPY_DST,
                            view_formats: vec![texture_format],
                        },
                        None,
                    );
                    let wgpu_texture = device.create_texture_from_hal::<Gles>(
                        hal_texture,
                        &TextureDescriptor {
                            label: None,
                            size: wgpu::Extent3d {
                                width: rect.width(),
                                height: rect.height(),
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 0,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format: texture_format,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING
                                | TextureUsages::STORAGE_BINDING,
                            view_formats: &[texture_format],
                        },
                    );
                    let texture: Texture = wgpu_texture.into();
                    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
                        label: None,
                        format: None,
                        dimension: None,
                        aspect: TextureAspect::DepthOnly,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: 0,
                        array_layer_count: None,
                    });
                    let sampler: Sampler =
                        device.create_sampler(&SamplerDescriptor::default()).into();
                    let image = GpuImage {
                        texture,
                        texture_view,
                        texture_format,
                        sampler,
                        size: rect.size_vec2(),
                        mip_level_count: 1,
                    };
                    // let texture=;
                    // let image: wgpu::Texture;
                    // image.as_hal::<wgpu_hal::api::Gles, _>(|r| {
                    //     let r: &<Gles as Api>::Texture = r.unwrap();
                    //     // r.
                    //     // let e=r.raw;
                    // });

                    // egl.create_pixmap_surface(display, Config::from_ptr(null_mut()), image, &[])?;

                    // let mut rbo = 0;

                    // gl.GenRenderbuffers(1, &mut rbo as *mut _);
                    // gl.BindRenderbuffer(ffi::RENDERBUFFER, rbo);
                    // gl
                    //     .EGLImageTargetRenderbufferStorageOES(ffi::RENDERBUFFER, image);
                    // gl.BindRenderbuffer(ffi::RENDERBUFFER, 0);
                    //
                    // let mut fbo = 0;
                    // gl.GenFramebuffers(1, &mut fbo as *mut _);
                    // gl.BindFramebuffer(ffi::FRAMEBUFFER, fbo);
                    // gl.FramebufferRenderbuffer(
                    //     ffi::FRAMEBUFFER,
                    //     ffi::COLOR_ATTACHMENT0,
                    //     ffi::RENDERBUFFER,
                    //     rbo,
                    // );
                    // let status = gl.CheckFramebufferStatus(ffi::FRAMEBUFFER);
                    // gl.BindFramebuffer(ffi::FRAMEBUFFER, 0);
                    //
                    // if status != ffi::FRAMEBUFFER_COMPLETE {
                    //     //TODO wrap image and drop here
                    //     return Err(Gles2Error::FramebufferBindingError);
                    // }
                    //
                    // let buf = Gles2Buffer {
                    //     dmabuf: dmabuf.weak(),
                    //     image,
                    //     rbo,
                    //     fbo,
                    // };
                    //
                    // buffers.push(buf.clone());
                    //
                    // Ok((buf, dmabuf))

                    // let image = create_dma_image(dma_buffer, egl, display.as_ptr())?;

                    Fallible::Ok(())
                })
            });
            if let Err(e) = result {
                error!("error while scan outputs: {e:?}");
            }

            // let gl_instace: &<Gles as Api>::Instance =
            //     render_instance.as_hal::<Gles>().expect("need gles backend");
        }
    }
}
