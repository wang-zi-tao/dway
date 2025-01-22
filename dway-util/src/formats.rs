use anyhow::{bail, Result};
use ash::vk;
use bevy::render::texture::{TextureFormatPixelInfo};
use drm_fourcc::DrmFourcc;
use wayland_server::protocol::wl_shm;
use wgpu::TextureFormat;

#[derive(Clone, Copy, Debug)]
pub struct ImageFormat {
    pub wl_format: wl_shm::Format,
    pub wgpu_format: wgpu::TextureFormat,
    pub vulkan_format: vk::Format,
    pub drm_format: DrmFourcc,
    pub gles_format: u32,
}

///ARGB little endian
pub const ARGB8888: ImageFormat = ImageFormat {
    wl_format: wl_shm::Format::Argb8888,
    wgpu_format: TextureFormat::Bgra8Unorm,
    vulkan_format: vk::Format::B8G8R8A8_UNORM,
    drm_format: DrmFourcc::Argb8888,
    gles_format: glow::BGRA,
};

pub const ARGB8888_SRGB: ImageFormat = ImageFormat {
    wl_format: wl_shm::Format::Argb8888,
    wgpu_format: TextureFormat::Bgra8UnormSrgb,
    vulkan_format: vk::Format::B8G8R8A8_SRGB,
    drm_format: DrmFourcc::Argb8888,
    gles_format: glow::SRGB_ALPHA,
};


///RGB little endian
pub const XRGB8888: ImageFormat = ImageFormat {
    wl_format: wl_shm::Format::Xrgb8888,
    wgpu_format: TextureFormat::Rgba8Unorm,
    vulkan_format: vk::Format::B8G8R8_UNORM,
    drm_format: DrmFourcc::Xrgb8888,
    gles_format: glow::BGRA,
};

pub const ABGR8888: ImageFormat = ImageFormat {
    wl_format: wl_shm::Format::Abgr8888,
    wgpu_format: TextureFormat::Rgba8Unorm,
    vulkan_format: vk::Format::R8G8B8_UNORM,
    drm_format: DrmFourcc::Abgr8888,
    gles_format: glow::RGBA,
};

pub const XBGR8888: ImageFormat = ImageFormat {
    wl_format: wl_shm::Format::Xbgr8888,
    wgpu_format: TextureFormat::Rgba8Unorm,
    vulkan_format: vk::Format::R8G8B8A8_UNORM,
    drm_format: DrmFourcc::Xbgr8888,
    gles_format: glow::RGBA,
};

impl ImageFormat {
    pub fn from_wayland_format(format: wl_shm::Format) -> Result<ImageFormat> {
        Ok( match format {
            wl_shm::Format::Argb8888 => ARGB8888,
            wl_shm::Format::Xrgb8888 => ARGB8888,
            wl_shm::Format::Abgr8888 => ABGR8888,
            wl_shm::Format::Xbgr8888 => ABGR8888,
            _ => {
                bail!("unsupported format ({format:?})");
            },
        } )
    }
    pub fn from_drm_fourcc(fourcc: DrmFourcc) -> Result<ImageFormat> {
        Ok( match fourcc {
            DrmFourcc::Argb8888 => ARGB8888,
            DrmFourcc::Xrgb8888 => ARGB8888,
            DrmFourcc::Abgr8888 => ABGR8888,
            DrmFourcc::Xbgr8888 => ABGR8888,
            _ => {
                bail!("unsupported fourcc ({fourcc:?})");
            },
        } )
    }
    pub fn from_wgpu(format: wgpu::TextureFormat) -> Result<Self>{
        Ok( match format {
            wgpu::TextureFormat::Bgra8UnormSrgb => ARGB8888_SRGB,
            wgpu::TextureFormat::Rgba8Unorm => ABGR8888,
            _ => {
                bail!("unsupported wgpu format ({format:?})");
            },
        } )
    }
    pub fn pixel_size(&self)->usize{
        self.wgpu_format.components() as usize
    }
}
