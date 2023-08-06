use crate::{
    prelude::*,
    util::rect::IRect,
    wl::{
        buffer::{DmaBuffer, EGLBuffer, WlBuffer, WlShmPool},
        surface::WlSurface,
    },
};

use drm_fourcc::DrmModifier;
use image::{io::Reader as ImageReader, Rgba};
use std::{
    collections::HashMap,
    default,
    ffi::{c_int, c_uint, c_void},
    num::NonZeroU32,
    os::fd::AsRawFd,
    path::Path,
    ptr::null_mut,
};

use bevy::{
    ecs::system::lifetimeless::{Read, SQuery, SRes, Write},
    prelude::{debug, info, Component, Query, QueryState, Res, UiCameraConfig, Vec2},
    render::{
        render_graph::Node,
        render_phase::{DrawFunctions, PhaseItem, RenderCommand, RenderPhase},
        render_resource::PipelineCache,
        renderer::RenderDevice,
        texture::GpuImage,
        view::ViewTarget,
    },
    sprite::SpritePipeline,
};
use failure::{format_err, Fallible};
use glow::{
    Fence, HasContext, NativeFence, NativeRenderbuffer, NativeTexture, PixelPackData, TEXTURE_2D,
};
pub const LINUX_DMA_BUF_EXT: u32 = 0x3270;
pub const WAYLAND_PLANE_WL: c_uint = 0x31D6;
pub const LINUX_DRM_FOURCC_EXT: u32 = 0x3271;

pub const DMA_BUF_PLANE0_FD_EXT: u32 = 0x3272;
pub const DMA_BUF_PLANE0_OFFSET_EXT: u32 = 0x3273;
pub const DMA_BUF_PLANE0_PITCH_EXT: u32 = 0x3274;
pub const DMA_BUF_PLANE0_MODIFIER_LO_EXT: u32 = 0x3443;
pub const DMA_BUF_PLANE0_MODIFIER_HI_EXT: u32 = 0x3444;

pub const DMA_BUF_PLANE1_FD_EXT: u32 = 0x3275;
pub const DMA_BUF_PLANE1_OFFSET_EXT: u32 = 0x3276;
pub const DMA_BUF_PLANE1_PITCH_EXT: u32 = 0x3277;
pub const DMA_BUF_PLANE1_MODIFIER_LO_EXT: u32 = 0x3445;
pub const DMA_BUF_PLANE1_MODIFIER_HI_EXT: u32 = 0x3446;

pub const DMA_BUF_PLANE2_FD_EXT: u32 = 0x3278;
pub const DMA_BUF_PLANE2_OFFSET_EXT: u32 = 0x3279;
pub const DMA_BUF_PLANE2_PITCH_EXT: u32 = 0x327A;
pub const DMA_BUF_PLANE2_MODIFIER_LO_EXT: u32 = 0x3447;
pub const DMA_BUF_PLANE2_MODIFIER_HI_EXT: u32 = 0x3448;

pub const DMA_BUF_PLANE3_FD_EXT: u32 = 0x3440;
pub const DMA_BUF_PLANE3_OFFSET_EXT: u32 = 0x3441;
pub const DMA_BUF_PLANE3_PITCH_EXT: u32 = 0x3442;
pub const DMA_BUF_PLANE3_MODIFIER_LO_EXT: u32 = 0x3449;
pub const DMA_BUF_PLANE3_MODIFIER_HI_EXT: u32 = 0x344A;

use image::ImageBuffer;
use khronos_egl::{EGLClientBuffer, EGLContext, EGLDisplay, EGLImage, EGLSurface, Enum, Int, NONE};
use wgpu::{util::DeviceExt, FilterMode, SamplerDescriptor, Texture, TextureAspect};
use wgpu_hal::Api;
use wgpu_hal::{api::Gles, MemoryFlags, TextureUses};

pub type TextureId = (NativeTexture, u32);

pub unsafe fn import_dma(
    buffer: &WlBuffer,
    dma_buffer: &DmaBuffer,
    egl_create_image_khr: extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
    display: EGLDisplay,
) -> Fallible<(EGLImage, IVec2)> {
    let mut out: Vec<c_int> = Vec::with_capacity(50);

    out.extend([
        khronos_egl::WIDTH as i32,
        dma_buffer.size.x,
        khronos_egl::HEIGHT as i32,
        dma_buffer.size.y,
        LINUX_DRM_FOURCC_EXT as i32,
        dma_buffer.format as i32,
    ]);

    let names = [
        [
            DMA_BUF_PLANE0_FD_EXT,
            DMA_BUF_PLANE0_OFFSET_EXT,
            DMA_BUF_PLANE0_PITCH_EXT,
            DMA_BUF_PLANE0_MODIFIER_LO_EXT,
            DMA_BUF_PLANE0_MODIFIER_HI_EXT,
        ],
        [
            DMA_BUF_PLANE1_FD_EXT,
            DMA_BUF_PLANE1_OFFSET_EXT,
            DMA_BUF_PLANE1_PITCH_EXT,
            DMA_BUF_PLANE1_MODIFIER_LO_EXT,
            DMA_BUF_PLANE1_MODIFIER_HI_EXT,
        ],
        [
            DMA_BUF_PLANE2_FD_EXT,
            DMA_BUF_PLANE2_OFFSET_EXT,
            DMA_BUF_PLANE2_PITCH_EXT,
            DMA_BUF_PLANE2_MODIFIER_LO_EXT,
            DMA_BUF_PLANE2_MODIFIER_HI_EXT,
        ],
        [
            DMA_BUF_PLANE3_FD_EXT,
            DMA_BUF_PLANE3_OFFSET_EXT,
            DMA_BUF_PLANE3_PITCH_EXT,
            DMA_BUF_PLANE3_MODIFIER_LO_EXT,
            DMA_BUF_PLANE3_MODIFIER_HI_EXT,
        ],
    ];
    for (i, plane) in dma_buffer.planes.iter().enumerate() {
        let fd = &plane.fd;
        let offset = plane.offset;
        let stride = plane.stride;
        out.extend([
            names[i][0] as i32,
            fd.as_raw_fd(),
            names[i][1] as i32,
            offset as i32,
            names[i][2] as i32,
            stride as i32,
        ]);
        if dma_buffer.planes[0].modifier != DrmModifier::Invalid
            && dma_buffer.planes[0].modifier != DrmModifier::Linear
        {
            out.extend([
                names[i][3] as i32,
                (Into::<u64>::into(dma_buffer.planes[0].modifier) & 0xFFFFFFFF) as i32,
                names[i][4] as i32,
                (Into::<u64>::into(dma_buffer.planes[0].modifier) >> 32) as i32,
            ])
        }
    }

    out.push(NONE as i32);

    let image = egl_create_image_khr(
        display,
        khronos_egl::NO_CONTEXT,
        LINUX_DMA_BUF_EXT,
        std::ptr::null_mut(),
        out.as_ptr(),
    );

    if image == null_mut() {
        Err(format_err!("failed to create dma image"))
    } else {
        Ok((image, dma_buffer.size))
    }
}
pub unsafe fn image_target_renderbuffer_storage_oes(
    fn_bind_image: extern "system" fn(target: Enum, image: *const c_void),
    raw_image: EGLImage,
) -> Fallible<()> {
    fn_bind_image(glow::RENDERBUFFER, raw_image);
    Ok(())
}
pub unsafe fn image_target_texture_oes(
    fn_bind_image: extern "system" fn(target: Enum, image: *const c_void),
    gl: &glow::Context,
    raw_image: EGLImage,
) -> Fallible<NativeTexture> {
    let texture = gl.create_texture().map_err(|e| format_err!("{e}"))?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    fn_bind_image(glow::TEXTURE_2D, raw_image);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(texture)
}

pub unsafe fn import_memory(
    surface: &WlSurface,
    buffer: &WlBuffer,
    gl: &glow::Context,
    dest: TextureId,
) -> Fallible<()> {
    let offset = buffer.offset;
    let width = buffer.size.x;
    let height = buffer.size.y;
    let stride = buffer.stride;
    let pixelsize = 4i32;
    let shm_inner = buffer.pool.read().unwrap();
    let ptr = std::ptr::from_raw_parts::<[u8]>(
        shm_inner.ptr.as_ptr().offset(buffer.offset as isize).cast(),
        (width * height * 4) as usize,
    )
    .as_ref()
    .unwrap();
    assert!(
        (buffer.offset + (buffer.stride * height) - buffer.stride + width * pixelsize) as usize
            <= shm_inner.size
    );
    gl.bind_texture(dest.1, Some(dest.0));
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::NEAREST as i32,
    );

    gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, stride / pixelsize);

    // let image: ImageBuffer<Rgba<u8>, Vec<_>> =
    //     image::ImageBuffer::from_vec(width as u32, height as u32, ptr.to_vec()).unwrap();
    // let snapshtip_count = std::fs::read_dir(".snapshot").unwrap().count();
    // image.save(format!(".snapshot/snapshot_{}.png", snapshtip_count + 1))?;
    // dbg!(ptr.iter().map(|v| *v as usize).sum::<usize>() / ptr.len());

    let (gl_format, shader_idx) = match buffer.format {
        wl_shm::Format::Abgr8888 => (glow::RGBA, 0),
        wl_shm::Format::Xbgr8888 => (glow::RGBA, 1),
        wl_shm::Format::Argb8888 => (glow::BGRA, 0),
        wl_shm::Format::Xrgb8888 => (glow::BGRA, 1),
        format => return Err(format_err!("unsupported format: {:?}", format)),
    };
    if surface.commited.damages.len() == 0 {
        trace!(surface=%WlResource::id(&surface.raw),"import {:?}", IRect::new(0, 0, width, height));
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            width,
            height,
            gl_format,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(ptr),
        );
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
    } else {
        for region in surface.commited.damages.iter() {
            trace!(surface=%WlResource::id(&surface.raw),"import {:?}", region);
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, region.pos().x.max(0));
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, region.pos().y.max(0));
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                region.pos().x.max(0),
                region.pos().y.max(0),
                region.size().x.min(width),
                region.size().y.min(height),
                gl_format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(ptr),
            );
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        }
    }
    gl.generate_mipmap(glow::TEXTURE_2D);
    //glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(())
}

pub unsafe fn import_egl(
    buffer: &WlBuffer,
    egl_buffer: &EGLBuffer,
    egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4>,
    display: khronos_egl::Display,
    egl_create_image_khr: extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
) -> Fallible<(EGLImage, IVec2)> {
    let egl_surface: khronos_egl::Surface =
        khronos_egl::Surface::from_ptr(buffer.raw.id().as_ptr() as _);

    let width = egl.query_surface(display, egl_surface, khronos_egl::WIDTH)?;
    let height = egl.query_surface(display, egl_surface, khronos_egl::HEIGHT)?;
    let format = egl.query_surface(display, egl_surface, khronos_egl::TEXTURE_FORMAT)?;
    let image_count = match format {
        khronos_egl::TEXTURE_RGB => 1,
        khronos_egl::TEXTURE_RGBA => 1,
        // Format::RGB | Format::RGBA | Format::External => 1,
        // Format::Y_UV | Format::Y_XUXV => 2,
        // Format::Y_U_V => 3,
        _ => panic!(),
    };
    // let inverted = egl.query_surface(*display, egl_surface, 0x31DB)?;

    let out = [WAYLAND_PLANE_WL as i32, 0 as i32, khronos_egl::NONE as i32];
    let image = egl_create_image_khr(
        display.as_ptr(),
        khronos_egl::NO_CONTEXT,
        LINUX_DMA_BUF_EXT,
        std::ptr::null_mut(),
        out.as_ptr(),
    );

    Ok((image, IVec2::new(width, height)))
}

pub unsafe fn import_buffer(
    gl: &glow::Context,
    gl_eglimage_target_renderbuffer_storage_oes: extern "system" fn(
        target: Enum,
        image: *const c_void,
    ),
    egl_image: EGLImage,
) -> Fallible<NativeRenderbuffer> {
    let render_buffer = gl
        .create_renderbuffer()
        .map_err(|e| format_err!("failed to create render buffer :{e}"))?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(render_buffer));
    gl_eglimage_target_renderbuffer_storage_oes(glow::RENDERBUFFER, egl_image);
    Ok(render_buffer)
}

pub unsafe fn create_gpu_image(
    device: &wgpu::Device,
    raw_image: NonZeroU32,
    size: IVec2,
) -> Fallible<GpuImage> {
    let texture_format = wgpu::TextureFormat::Rgba8Unorm;
    let hal_texture: <Gles as Api>::Texture = device.as_hal::<Gles, _, _>(|hal_device| {
        Fallible::Ok(
            hal_device
                .ok_or_else(|| format_err!("failed to get hal device"))?
                .texture_from_raw(
                    raw_image,
                    &wgpu_hal::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d {
                            width: size.x as u32,
                            height: size.y as u32,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: texture_format,
                        memory_flags: MemoryFlags::empty(),
                        usage: TextureUses::COPY_DST,
                        view_formats: vec![texture_format],
                    },
                    None,
                ),
        )
    })?;
    let wgpu_texture = device.create_texture_from_hal::<Gles>(
        hal_texture,
        &wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[texture_format],
        },
    );
    let texture: wgpu::Texture = wgpu_texture.into();
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(texture_format),
        dimension: None,
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1.try_into().unwrap()),
        base_array_layer: 0,
        array_layer_count: None,
    });
    let sampler: wgpu::Sampler = device
        .create_sampler(&SamplerDescriptor {
            label: None,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            compare: None,
            anisotropy_clamp: None,
            border_color: None,
            address_mode_u: Default::default(),
            address_mode_v: Default::default(),
            address_mode_w: Default::default(),
            lod_min_clamp: Default::default(),
            lod_max_clamp: Default::default(),
        })
        .into();
    let image = GpuImage {
        texture: texture.into(),
        texture_view: texture_view.into(),
        texture_format,
        sampler: sampler.into(),
        size: Vec2::new(size.x as f32, size.y as f32),
        mip_level_count: 1,
    };
    Ok(image)
}
pub enum BufferType {
    Shm,
    Egl,
    Dma,
}

pub fn import_wl_surface(
    surface: &WlSurface,
    buffer: &WlBuffer,
    dma_buffer: Option<&DmaBuffer>,
    egl_buffer: Option<&EGLBuffer>,
    texture: &Texture,
    device: &wgpu::Device,
) -> Fallible<()> {
    unsafe {
        let display: khronos_egl::Display = device.as_hal::<Gles, _, _>(|hal_device| {
            hal_device
                .ok_or_else(|| format_err!("gpu backend is not egl"))?
                .context()
                .raw_display()
                .cloned()
                .ok_or_else(|| format_err!("no opengl display available"))
        })?;
        let mut texture_id = None;
        texture.as_hal::<Gles, _>(|texture| {
            let texture = texture.unwrap();
            // debug!("dest texture: {:?}",texture);
            match &texture.inner {
                wgpu_hal::gles::TextureInner::Texture { raw, target } => {
                    texture_id = Some((*raw, *target));
                }
                _ => {}
            }
        });
        let Some(texture_id) = texture_id else {
            return Err(format_err!("failed to get raw texture"));
        };
        device.as_hal::<Gles, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| format_err!("render device is not egl"))?;
            let egl_context = hal_device.context();
            let gl: &glow::Context = &egl_context.lock();
            gl.enable(glow::DEBUG_OUTPUT);
            gl.debug_message_callback(gl_debug_message_callback);
            let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> =
                egl_context.egl_instance().ok_or_else(|| {
                    gl.disable(glow::DEBUG_OUTPUT);
                    format_err!("render adapter is not egl")
                })?;
            let egl_create_image_khr: extern "system" fn(
                EGLDisplay,
                EGLContext,
                Enum,
                EGLClientBuffer,
                *const Int,
            ) -> EGLImage = std::mem::transmute(
                egl.get_proc_address("eglCreateImageKHR").ok_or_else(|| {
                    gl.disable(glow::DEBUG_OUTPUT);
                    format_err!("failed to get function eglCreateImageKHR")
                })?,
            );
            let gl_eglimage_target_texture2_does: extern "system" fn(
                target: Enum,
                image: *const c_void,
            ) = std::mem::transmute(
                egl.get_proc_address("glEGLImageTargetTexture2DOES")
                    .ok_or_else(|| {
                        gl.disable(glow::DEBUG_OUTPUT);
                        format_err!("failed to get function glEGLImageTargetTexture2DOES")
                    })?,
            );
            if let Some(egl_buffer) = egl_buffer {
                let (raw_image, size) =
                    import_egl(buffer, egl_buffer, egl, display, egl_create_image_khr)?;
                let texture =
                    image_target_texture_oes(gl_eglimage_target_texture2_does, gl, raw_image)?;
            } else if let Some(dma_buffer) = dma_buffer {
                let (raw_image, size) =
                    import_dma(buffer, dma_buffer, egl_create_image_khr, display.as_ptr())?;
                let texture =
                    image_target_texture_oes(gl_eglimage_target_texture2_does, gl, raw_image)?;
            } else {
                import_memory(surface, buffer,  gl, texture_id)?;
            }
            gl.disable(glow::DEBUG_OUTPUT);
            Ok(())
        })
    }
}
pub fn gl_debug_message_callback(source: u32, gltype: u32, id: u32, severity: u32, message: &str) {
    let source_str = match source {
        glow::DEBUG_SOURCE_API => "API",
        glow::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
        glow::DEBUG_SOURCE_SHADER_COMPILER => "ShaderCompiler",
        glow::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
        glow::DEBUG_SOURCE_APPLICATION => "Application",
        glow::DEBUG_SOURCE_OTHER => "Other",
        _ => unreachable!(),
    };

    let type_str = match gltype {
        glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated Behavior",
        glow::DEBUG_TYPE_ERROR => "Error",
        glow::DEBUG_TYPE_MARKER => "Marker",
        glow::DEBUG_TYPE_OTHER => "Other",
        glow::DEBUG_TYPE_PERFORMANCE => "Performance",
        glow::DEBUG_TYPE_POP_GROUP => "Pop Group",
        glow::DEBUG_TYPE_PORTABILITY => "Portability",
        glow::DEBUG_TYPE_PUSH_GROUP => "Push Group",
        glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
        _ => unreachable!(),
    };

    let _ = std::panic::catch_unwind(|| {
        info!(
            "GLES: [{}/{}] ID {} : {}",
            source_str, type_str, id, message
        );
    });
}
