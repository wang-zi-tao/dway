use std::{
    os::fd::{AsFd, AsRawFd, IntoRawFd},
    ptr::null,
    sync::{Arc, RwLock},
};

use anyhow::{anyhow, bail, Result};
use ash::{
    khr::external_memory_fd,
    vk::{self, *},
};
use bevy::render::texture::GpuImage;
use bevy_relationship::reexport::SmallVec;
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use dway_util::formats::ImageFormat;
use nix::{libc::makedev, sys::stat::fstat};
use wgpu::{Extent3d, ImageCopyTexture, TextureAspect};
use wgpu_hal::{
    vulkan::{self, Api as Vulkan},
    DropCallback,
};

use super::{
    drm::{DrmInfo, DrmNode},
    importnode::{
        drm_fourcc_to_wgpu_format, hal_texture_descriptor, hal_texture_to_gpuimage, merge_damage,
        ImportedBuffer,
    },
    util::DWayRenderError::{self, *},
    ImportDmaBufferRequest,
};
use crate::{
    prelude::*,
    util::rect::IRect,
    wl::{
        buffer::{WlShmBuffer, WlShmPoolInner},
        surface::WlSurface,
    },
};

pub const MEM_PLANE_ASCPECT: [ImageAspectFlags; 4] = [
    ImageAspectFlags::MEMORY_PLANE_0_EXT,
    ImageAspectFlags::MEMORY_PLANE_1_EXT,
    ImageAspectFlags::MEMORY_PLANE_2_EXT,
    ImageAspectFlags::MEMORY_PLANE_3_EXT,
];

#[derive(Debug)]
pub struct ImageDropGuard {
    pub device: vk::Device,
    pub image: vk::Image,
    pub memory: SmallVec<[vk::DeviceMemory; 4]>,
    pub shm_pool: Option<Arc<RwLock<WlShmPoolInner>>>,
    pub fn_free_memory: PFN_vkFreeMemory,
    pub fn_destroy_image: PFN_vkDestroyImage,
}

impl ImageDropGuard {
    fn drop_callback(self) -> DropCallback {
        Box::new(move || {
            let _ = self;
        })
    }
}

impl Drop for ImageDropGuard {
    fn drop(&mut self) {
        unsafe {
            (self.fn_destroy_image)(self.device, self.image, null());
        }
        for memory in &self.memory {
            unsafe {
                (self.fn_free_memory)(self.device, *memory, null());
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct VulkanState {}

pub const SUPPORTED_FORMATS: [DrmFourcc; 4] = [
    DrmFourcc::Argb8888,
    DrmFourcc::Xrgb8888,
    DrmFourcc::Abgr8888,
    DrmFourcc::Xbgr8888,
];

pub fn drm_info(render_device: &wgpu::Device) -> Result<DrmInfo, DWayRenderError> {
    unsafe {
        render_device.as_hal::<Vulkan, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| BackendIsNotVulkan)?;
            info!("use vulkan");

            let instance = hal_device.shared_instance().raw_instance();
            let raw_phy = hal_device.raw_physical_device();

            let mut formats = Vec::new();
            for fourcc in SUPPORTED_FORMATS {
                let vk_format = ImageFormat::from_drm_fourcc(fourcc)?.vulkan_format;

                let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
                let mut format_properties2 = vk::FormatProperties2::default().push_next(&mut list);
                instance.get_physical_device_format_properties2(
                    raw_phy,
                    vk_format,
                    &mut format_properties2,
                );
                let count = list.drm_format_modifier_count;
                let mut modifiers_list = vec![Default::default(); count as usize];
                let mut modifier_list_prop = vk::DrmFormatModifierPropertiesListEXT::default()
                    .drm_format_modifier_properties(&mut modifiers_list);

                let mut format_properties2 =
                    vk::FormatProperties2::default().push_next(&mut modifier_list_prop);
                instance.get_physical_device_format_properties2(
                    raw_phy,
                    vk_format,
                    &mut format_properties2,
                );

                // modifiers_list.clear(); // TODO : 改进解决方法
                if modifiers_list.is_empty() {
                    warn!(format=?fourcc, "no available modifier of format");
                    formats.push(DrmFormat {
                        code: fourcc,
                        modifier: DrmModifier::Linear,
                    });
                }
                formats.extend(modifiers_list.into_iter().map(|d| DrmFormat {
                    code: fourcc,
                    modifier: DrmModifier::from(d.drm_format_modifier),
                }));
            }

            let mut drm_prop = PhysicalDeviceDrmPropertiesEXT::default();
            let mut device_prop = PhysicalDeviceProperties2::default().push_next(&mut drm_prop);
            (instance.fp_v1_1().get_physical_device_properties2)(
                hal_device.raw_physical_device(),
                &mut device_prop,
            );

            let drm_node = DrmNode::from_device_id(makedev(
                drm_prop.render_major as _,
                drm_prop.render_minor as _,
            ))?;

            Ok(DrmInfo {
                texture_formats: formats.clone(),
                render_formats: formats.clone(),
                drm_node,
            })
        })
    }
}

pub fn create_vulkan_dma_image(
    hal_device: &wgpu_hal::vulkan::Device,
    buffer: &mut ImportDmaBufferRequest,
) -> Result<ImageDropGuard> {
    let instance = hal_device.shared_instance().raw_instance();
    let device = hal_device.raw_device();
    let physical = hal_device.raw_physical_device();

    let format = DrmFourcc::try_from(buffer.format)?;

    debug!(size=?buffer.size, ?format, "create dma image");

    unsafe {
        let planes = std::mem::take(&mut buffer.planes);
        if planes.is_empty() {
            bail!(InvalidDmaBuffer);
        }
        let plane_layouts: Vec<_> = planes
            .iter()
            .map(|plane| {
                SubresourceLayout::default()
                    .offset(plane.offset as u64)
                    .row_pitch(plane.stride as u64)
            })
            .collect();

        let is_disjoint = if planes.len() == 1 {
            false
        } else {
            fstat(planes[0].fd.as_raw_fd())
                .map(|first_stat| {
                    planes.iter().any(|plane| {
                        fstat(plane.fd.as_raw_fd())
                            .map(|stat| stat.st_ino != first_stat.st_ino)
                            .unwrap_or(true)
                    })
                })
                .unwrap_or(true)
        };

        debug!(
            "dma image format: {:?} modifier: {:?}",
            format, planes[0].modifier
        );
        let mut drm_info = ash::vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
            .drm_format_modifier(planes[0].modifier.into())
            .plane_layouts(&plane_layouts);

        let mut dmabuf_info = ash::vk::ExternalMemoryImageCreateInfoKHR::default()
            .handle_types(ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let create_image_info = ash::vk::ImageCreateInfo::default()
            .sharing_mode(SharingMode::EXCLUSIVE)
            .image_type(ImageType::TYPE_2D)
            .extent(Extent3D {
                width: buffer.size.x,
                height: buffer.size.y,
                depth: 1,
            })
            .tiling(ImageTiling::DRM_FORMAT_MODIFIER_EXT)
            .mip_levels(1)
            .array_layers(1)
            .format(ImageFormat::from_drm_fourcc(format)?.vulkan_format)
            .samples(SampleCountFlags::TYPE_1)
            .usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .flags(if is_disjoint {
                ImageCreateFlags::DISJOINT
            } else {
                ImageCreateFlags::empty()
            })
            .push_next(&mut dmabuf_info)
            .push_next(&mut drm_info);
        let image = device
            .create_image(&create_image_info, None)
            .map_err(|e| anyhow!("error while create_image: {e}"))?;

        let mut plane_infos = Vec::with_capacity(planes.len());
        let mut bind_infos = Vec::with_capacity(planes.len());

        let plane_count = if is_disjoint { planes.len() } else { 1 };
        let mut memorys = SmallVec::<[_; 4]>::new();
        for (i, plane) in planes.into_iter().enumerate().take(plane_count) {
            let memory_requirement = {
                let mut requirement_info =
                    ash::vk::ImageMemoryRequirementsInfo2::default().image(image);
                let mut plane_requirement_info =
                    ash::vk::ImagePlaneMemoryRequirementsInfo::default()
                        .plane_aspect(MEM_PLANE_ASCPECT[i]);
                if is_disjoint {
                    requirement_info.p_next = &mut plane_requirement_info
                        as *mut ImagePlaneMemoryRequirementsInfo
                        as *mut _;
                }
                let mut memory_requrement = ash::vk::MemoryRequirements2::default();
                device.get_image_memory_requirements2(&requirement_info, &mut memory_requrement);
                memory_requrement
            };
            let phy_mem_prop = instance.get_physical_device_memory_properties(physical);

            let fd_mem_type = if instance
                .get_device_proc_addr(device.handle(), c"vkGetMemoryFdPropertiesKHR".as_ptr())
                .is_some()
            {
                let mut properties = MemoryFdPropertiesKHR::default();
                external_memory_fd::Device::new(instance, device).get_memory_fd_properties(
                    ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                    plane.fd.as_fd().as_raw_fd(),
                    &mut properties,
                )?;
                properties.memory_type_bits
            } else {
                !0
            };

            let mut fd_info = ash::vk::ImportMemoryFdInfoKHR::default()
                .fd(plane.fd.into_raw_fd())
                .handle_type(ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

            let alloc_info = ash::vk::MemoryAllocateInfo::default()
                .allocation_size(memory_requirement.memory_requirements.size.max(1))
                .memory_type_index(
                    phy_mem_prop
                        .memory_types
                        .iter()
                        .enumerate()
                        .position(|(i, _t)| {
                            0 != (1 << i)
                                & (memory_requirement.memory_requirements.memory_type_bits
                                    & fd_mem_type) as usize
                        })
                        .map(|v| v as u32)
                        .ok_or_else(|| NoValidMemoryType)?,
                )
                .push_next(&mut fd_info);
            let memory = device
                .allocate_memory(&alloc_info, None)
                .map_err(|e| anyhow!("error while allocate_memory: {e}"))?;

            let mut bind_info = BindImageMemoryInfo::default()
                .image(image)
                .memory(memory)
                .memory_offset(0);

            if is_disjoint {
                let mut info = Box::new(
                    vk::BindImagePlaneMemoryInfo::default().plane_aspect(MEM_PLANE_ASCPECT[i]),
                );
                bind_info.p_next = info.as_mut() as *mut _ as *mut _;
                plane_infos.push(info);
            }

            bind_infos.push(bind_info);
            memorys.push(memory);
        }
        device
            .bind_image_memory2(&bind_infos)
            .map_err(|e| anyhow!("error while bind_image_memory2: {e}"))?;

        Ok(ImageDropGuard {
            device: device.handle(),
            image,
            memory: memorys,
            shm_pool: None,
            fn_free_memory: device.fp_v1_0().free_memory,
            fn_destroy_image: device.fp_v1_0().destroy_image,
        })
    }
}

pub unsafe fn import_shm(
    surface: &WlSurface,
    queue: &wgpu::Queue,
    shm_buffer: &WlShmBuffer,
    texture: &wgpu::Texture,
) -> Result<(), DWayRenderError> {
    span!(Level::ERROR, "import_shm", shm_buffer = %WlResource::id(&shm_buffer.raw));
    let buffer_guard = shm_buffer.pool.read().unwrap();
    let size = shm_buffer.size;

    let data = buffer_guard.as_slice(shm_buffer)?;

    let image_area = IRect::from_pos_size(IVec2::default(), size);
    let texture_extent = texture.size();
    let texture_size = IVec2::new(texture_extent.width as i32, texture_extent.height as i32);
    let emit_rect = |rect: IRect| -> Result<()> {
        let rect = rect.intersection(IRect::from_pos_size(IVec2::ZERO, texture_size));
        debug!(?rect, "write_texture");
        queue.write_texture(
            ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect.x() as u32,
                    y: rect.y() as u32,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: (shm_buffer.stride * rect.y()
                    + rect.x()
                        * ImageFormat::from_wayland_format(shm_buffer.format)?.pixel_size() as i32)
                    as u64,
                bytes_per_row: (shm_buffer.stride as u32).try_into().ok(),
                rows_per_image: None,
            },
            Extent3d {
                width: rect.width() as u32,
                height: rect.height() as u32,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    };

    let damage = merge_damage(&surface.commited.damages);
    if damage.is_empty() {
        emit_rect(image_area)?;
    } else {
        for rect in damage {
            emit_rect(rect)?;
        }
    }

    Ok(())
}

pub fn create_wgpu_dma_image(
    device: &wgpu::Device,
    request: &mut ImportDmaBufferRequest,
) -> Result<(GpuImage, ImportedBuffer), DWayRenderError> {
    unsafe {
        let image_guard = device.as_hal::<Vulkan, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| BackendIsNotVulkan)?;
            let image = create_vulkan_dma_image(hal_device, request)?;
            Result::<_, DWayRenderError>::Ok(image)
        })?;
        let image = image_guard.image;
        let format = drm_fourcc_to_wgpu_format(request)?;
        let hal_texture = vulkan::Device::texture_from_raw(
            image,
            &hal_texture_descriptor(request.size, format)?,
            None,
        );
        let gpu_image =
            hal_texture_to_gpuimage::<Vulkan>(device, request.size, format, hal_texture)?;
        Ok((gpu_image, ImportedBuffer::VULKAN))
    }
}
