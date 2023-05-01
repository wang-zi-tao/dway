use image::io::Reader as ImageReader;
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
use image::ImageBuffer;
use khronos_egl::{EGLClientBuffer, EGLContext, EGLDisplay, EGLImage, EGLSurface, Enum, Int};
use smithay::{
    backend::{
        allocator::{dmabuf::Dmabuf, Buffer},
        egl::ffi::egl::{LINUX_DMA_BUF_EXT, WAYLAND_PLANE_WL},
        renderer::{
            buffer_type,
            element::{default_primary_scanout_output_compare, RenderElementStates},
            utils::RendererSurfaceState,
            BufferType,
        },
    },
    desktop::utils::{
        send_frames_surface_tree, surface_primary_scanout_output,
        update_surface_primary_scanout_output,
    },
    reexports::wayland_server::{
        protocol::{wl_buffer::WlBuffer, wl_shm, wl_surface::WlSurface},
        Resource,
    },
    utils::{Physical, Rectangle, Size},
    wayland::{
        dmabuf::get_dmabuf,
        fractional_scale::with_fractional_scale,
        shm::{with_buffer_contents, BufferData},
    },
};
use wgpu::{util::DeviceExt, FilterMode, SamplerDescriptor, Texture, TextureAspect};
use wgpu_hal::Api;
use wgpu_hal::{api::Gles, MemoryFlags, TextureUses};

use crate::{
    components::{SurfaceId, WaylandWindow, WlSurfaceWrapper, X11Window},
    surface::{try_with_states_borrowed, with_states_borrowed, ImportedSurface},
};

pub type TextureId = (NativeTexture, u32);

pub unsafe fn import_dma(
    dma_buffer: Dmabuf,
    egl_create_image_khr: extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
    display: EGLDisplay,
) -> Fallible<(EGLImage, Size<i32, Physical>)> {
    use smithay::backend::egl::ffi::egl::*;
    let mut out: Vec<c_int> = Vec::with_capacity(50);

    out.extend([
        khronos_egl::WIDTH as i32,
        dma_buffer.width() as i32,
        khronos_egl::HEIGHT as i32,
        dma_buffer.height() as i32,
        LINUX_DRM_FOURCC_EXT as i32,
        dma_buffer.format().code as i32,
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

    for (i, ((fd, offset), stride)) in dma_buffer
        .handles()
        .zip(dma_buffer.offsets())
        .zip(dma_buffer.strides())
        .enumerate()
    {
        out.extend([
            names[i][0] as i32,
            fd.as_raw_fd(),
            names[i][1] as i32,
            offset as i32,
            names[i][2] as i32,
            stride as i32,
        ]);
        if dma_buffer.has_modifier() {
            out.extend([
                names[i][3] as i32,
                (Into::<u64>::into(dma_buffer.format().modifier) & 0xFFFFFFFF) as i32,
                names[i][4] as i32,
                (Into::<u64>::into(dma_buffer.format().modifier) >> 32) as i32,
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
        let size = dma_buffer.size();
        Ok((image, (size.w, size.h).into()))
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
    buffer: &[u8],
    metadata: BufferData,
    gl: &glow::Context,
    damage: &[Rectangle<i32, Physical>],
    dest: TextureId,
) -> Fallible<()> {
    let offset = metadata.offset as i32;
    let width = metadata.width as i32;
    let height = metadata.height as i32;
    let stride = metadata.stride as i32;
    let pixelsize = 4i32;
    assert!((offset + (height - 1) * stride + width * pixelsize) as usize <= buffer.len());
    dbg!(buffer.iter().map(|d| *d as usize).sum::<usize>() / buffer.len());
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

    let (gl_format, shader_idx) = match metadata.format {
        wl_shm::Format::Abgr8888 => (glow::RGBA, 0),
        wl_shm::Format::Xbgr8888 => (glow::RGBA, 1),
        wl_shm::Format::Argb8888 => (glow::BGRA, 0),
        wl_shm::Format::Xrgb8888 => (glow::BGRA, 1),
        format => return Err(format_err!("unsupported format: {:?}", format)),
    };
    if damage.len() == 0 {
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
            glow::PixelUnpackData::Slice(buffer),
        );
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
    } else {
        for region in damage.iter() {
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, region.loc.x);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, region.loc.y);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                region.loc.x,
                region.loc.y,
                region.size.w,
                region.size.h,
                gl_format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(buffer),
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
    egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4>,
    display: khronos_egl::Display,
    egl_create_image_khr: extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
    damage: &[Rectangle<i32, Physical>],
) -> Fallible<(EGLImage, Size<i32, Physical>)> {
    let egl_surface: khronos_egl::Surface =
        khronos_egl::Surface::from_ptr(buffer.id().as_ptr() as _);

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

    Ok((image, Size::from((width, height))))
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
    damage: &[Rectangle<i32, Physical>],
    raw_image: NonZeroU32,
    size: Size<i32, Physical>,
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
                            width: size.w as u32,
                            height: size.h as u32,
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
                width: size.w as u32,
                height: size.h as u32,
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
        size: Vec2::new(size.w as f32, size.h as f32),
        mip_level_count: 1,
    };
    Ok(image)
}

pub fn import_wl_surface(
    buffer: &WlBuffer,
    texture: &Texture,
    damage: &[Rectangle<i32, Physical>],
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
            match &texture.inner {
                wgpu_hal::gles::TextureInner::Texture { raw, target } => {
                    texture_id = Some((*raw, *target));
                }
                _ => {}
            }
        });
        let Some( texture_id )=texture_id else{
            return Err(format_err!("failed to get raw texture"));
        };
        device.as_hal::<Gles, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| format_err!("render device is not egl"))?;
            let egl_context = hal_device.context();
            let gl: &glow::Context = &egl_context.lock();
            // gl.enable(glow::DEBUG_OUTPUT);
            // gl.debug_message_callback(gl_debug_message_callback);
            let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
                .egl_instance()
                .ok_or_else(|| format_err!("render adapter is not egl"))?;
            let egl_create_image_khr: extern "system" fn(
                EGLDisplay,
                EGLContext,
                Enum,
                EGLClientBuffer,
                *const Int,
            ) -> EGLImage = std::mem::transmute(
                egl.get_proc_address("eglCreateImageKHR")
                    .ok_or_else(|| format_err!("failed to get function eglCreateImageKHR"))?,
            );
            let gl_eglimage_target_texture2_does: extern "system" fn(
                target: Enum,
                image: *const c_void,
            ) = std::mem::transmute(
                egl.get_proc_address("glEGLImageTargetTexture2DOES")
                    .ok_or_else(|| {
                        format_err!("failed to get function glEGLImageTargetTexture2DOES")
                    })?,
            );
            match buffer_type(buffer) {
                Some(BufferType::Shm) => {
                    with_buffer_contents(buffer, |ptr, len, metadata| {
                        // device.create_texture_with_data(queue, desc, data)
                        import_memory(
                            std::slice::from_raw_parts(ptr, len),
                            metadata,
                            gl,
                            damage,
                            texture_id,
                        )
                    })
                    .map_err(|e| format_err!("{e}"))
                    .flatten()?;
                }
                Some(BufferType::Egl) => {
                    let (raw_image, size) =
                        import_egl(buffer, egl, display, egl_create_image_khr, damage)?;
                    let texture =
                        image_target_texture_oes(gl_eglimage_target_texture2_does, gl, raw_image)?;
                }
                Some(BufferType::Dma) => {
                    let dmabuf = get_dmabuf(buffer)?;
                    let (raw_image, size) =
                        import_dma(dmabuf, egl_create_image_khr, display.as_ptr())?;
                    let texture =
                        image_target_texture_oes(gl_eglimage_target_texture2_does, gl, raw_image)?;
                }
                _ => {
                    return Err(format_err!("unnkown buffer type"));
                }
            };
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
