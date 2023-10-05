use crate::prelude::*;
use crate::util::rect::IRect;
use crate::wl::buffer::UninitedWlBuffer;
use crate::wl::buffer::WlShmBuffer;
use crate::wl::buffer::WlShmPoolInner;
use crate::wl::surface::WlSurface;
use crate::zwp::dmabufparam::DmaBuffer;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use ash::extensions::ext::PhysicalDeviceDrm;
use ash::extensions::khr::ExternalMemoryFd;
use ash::extensions::khr::Maintenance4;
use ash::prelude::*;
use ash::vk;
use ash::vk::*;
use ash::Device;
use ash::RawPtr;
use bevy::asset::Handle;
use bevy::prelude::IVec2;
use bevy::prelude::Vec2;
use bevy::render::render_asset::RenderAssets;
use bevy::render::renderer::RenderContext;
use bevy::render::renderer::RenderQueue;
use bevy::render::texture::GpuImage;
use bevy::utils::Entry;
use bevy::utils::HashMap;
use bevy::utils::HashSet;
use bevy_relationship::reexport::SmallVec;
use drm_fourcc::DrmFormat;
use drm_fourcc::DrmFourcc;
use drm_fourcc::DrmModifier;
use nix::libc::makedev;
use scopeguard::defer;
use std::ffi::CStr;
use std::os::fd::AsFd;
use std::os::fd::AsRawFd;
use std::ptr::null;
use std::sync::Arc;
use std::sync::RwLock;
use wayland_server::protocol::wl_buffer;
use wayland_server::protocol::wl_shm;
use wayland_server::Resource;
use wgpu::util::DeviceExt;
use wgpu::CommandEncoder;
use wgpu::Extent3d;
use wgpu::FilterMode;
use wgpu::ImageCopyTexture;
use wgpu::SamplerDescriptor;
use wgpu::TextureAspect;
use wgpu::TextureDimension;
use wgpu::TextureFormat;
use wgpu_hal::api::Vulkan;
use wgpu_hal::vulkan::Texture;
use wgpu_hal::CommandEncoderDescriptor;
use wgpu_hal::Device as HalDevice;
use wgpu_hal::MemoryFlags;
use wgpu_hal::TextureDescriptor;
use wgpu_hal::TextureUses;

use super::drm::DrmInfo;
use super::drm::DrmNode;
use super::importnode::RenderImage;
use super::util::DWayRenderError;
use super::util::DWayRenderError::*;

pub const MEM_PLANE_ASCPECT: [ImageAspectFlags; 4] = [
    ImageAspectFlags::MEMORY_PLANE_0_EXT,
    ImageAspectFlags::MEMORY_PLANE_1_EXT,
    ImageAspectFlags::MEMORY_PLANE_2_EXT,
    ImageAspectFlags::MEMORY_PLANE_3_EXT,
];

#[derive(Debug)]
pub struct ImportedImage {
    pub image: vk::Image,
    pub fence: vk::Fence,
    pub memory: SmallVec<[vk::DeviceMemory; 4]>,
    pub buffer_to_release: Option<wl_buffer::WlBuffer>,
    pub shm_pool: Option<Arc<RwLock<WlShmPoolInner>>>,
}

#[derive(Debug, Default)]
pub struct VulkanState {
    pub image_map: HashMap<wl_buffer::WlBuffer, (ImportedImage, GpuImage)>,
}

pub fn convert_wl_format(
    format: wl_shm::Format,
) -> Result<(vk::Format, wgpu::TextureFormat), DWayRenderError> {
    Ok(match format {
        wl_shm::Format::Argb8888 => (Format::B8G8R8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        wl_shm::Format::Xrgb8888 => (Format::B8G8R8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        wl_shm::Format::Abgr8888 => (Format::R8G8B8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        wl_shm::Format::Xbgr8888 => (Format::R8G8B8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        _ => todo!(),
        f => return Err(UnsupportedFormat(f)),
    })
}

pub fn convert_drm_format(
    fourcc: DrmFourcc,
) -> Result<(vk::Format, wgpu::TextureFormat), DWayRenderError> {
    Ok(match fourcc {
        DrmFourcc::Argb8888 => (Format::B8G8R8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        DrmFourcc::Xrgb8888 => (Format::B8G8R8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        DrmFourcc::Abgr8888 => (Format::R8G8B8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        DrmFourcc::Abgr8888 => (Format::R8G8B8A8_SRGB, wgpu::TextureFormat::Bgra8Unorm),
        f => return Err(UnsupportedDrmFormat(f)),
    })
}

pub const SUPPORTED_FORMATS: [DrmFourcc; 2] = [DrmFourcc::Argb8888, DrmFourcc::Xrgb8888];

pub fn drm_info(render_device: &wgpu::Device) -> Result<DrmInfo, DWayRenderError> {
    unsafe {
        render_device.as_hal::<Vulkan, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| BackendIsNotVulkan)?;

            let instance = hal_device.shared_instance().raw_instance();
            let raw_phy = hal_device.raw_physical_device();

            let mut formats = Vec::new();
            let phy_info = instance.get_physical_device_properties(raw_phy);

            for fourcc in SUPPORTED_FORMATS {
                let vk_format = convert_drm_format(fourcc)?.0;

                let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
                let mut format_properties2 = vk::FormatProperties2::builder().push_next(&mut list);
                instance.get_physical_device_format_properties2(
                    raw_phy,
                    vk_format,
                    &mut format_properties2,
                );
                let count = list.drm_format_modifier_count;
                let mut data = vec![Default::default(); count as usize];

                let mut list = vk::DrmFormatModifierPropertiesListEXT {
                    p_drm_format_modifier_properties: data.as_mut_ptr(),
                    drm_format_modifier_count: count as u32,
                    ..Default::default()
                };
                let mut format_properties2 = vk::FormatProperties2::builder().push_next(&mut list);
                instance.get_physical_device_format_properties2(
                    raw_phy,
                    vk_format,
                    &mut format_properties2,
                );

                formats.extend(data.into_iter().map(|d| DrmFormat {
                    code: fourcc,
                    modifier: DrmModifier::from(d.drm_format_modifier),
                }));
            }

            let drm_prop =
                PhysicalDeviceDrm::get_properties(instance, hal_device.raw_physical_device());
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

pub fn create_dma_image(
    hal_device: &wgpu_hal::vulkan::Device,
    buffer: &DmaBuffer,
) -> Result<ImportedImage> {
    let instance = hal_device.shared_instance().raw_instance();
    let device = hal_device.raw_device();
    let physical = hal_device.raw_physical_device();
    unsafe {
        let planes = &buffer.planes.lock().unwrap().list;
        if planes.len() == 0 {
            bail!(InvalidDmaBuffer);
        }

        let mut dmabuf_info = ash::vk::ExternalMemoryImageCreateInfoKHR::builder()
            .handle_types(ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
            .build();
        let create_image_info = ash::vk::ImageCreateInfo::builder()
            .sharing_mode(SharingMode::EXCLUSIVE)
            .image_type(ImageType::TYPE_2D)
            .extent(Extent3D {
                width: buffer.size.x as u32,
                height: buffer.size.y as u32,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(convert_drm_format(DrmFourcc::try_from(buffer.format)?)?.0)
            .samples(SampleCountFlags::TYPE_1)
            .initial_layout(ImageLayout::PREINITIALIZED)
            .usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .flags({
                let mut flags = ImageCreateFlags::empty();
                if planes.len() > 1 {
                    flags |= ImageCreateFlags::DISJOINT;
                };
                flags
            })
            .push_next(&mut dmabuf_info)
            .build();
        let image = device.create_image(&create_image_info, None)?;

        let mut plane_infos = Vec::with_capacity(planes.len());
        let mut bind_infos = Vec::with_capacity(planes.len());
        let mut memorys = SmallVec::<[_; 4]>::new();
        for (i, plane) in planes.iter().enumerate() {
            let memory_requirement = {
                let mut requirement_info = ash::vk::ImageMemoryRequirementsInfo2::builder()
                    .image(image)
                    .build();
                let mut plane_requirement_info =
                    ash::vk::ImagePlaneMemoryRequirementsInfo::builder()
                        .plane_aspect(MEM_PLANE_ASCPECT[i])
                        .build();
                if planes.len() > 1 {
                    requirement_info.p_next = &mut plane_requirement_info
                        as *mut ImagePlaneMemoryRequirementsInfo
                        as *mut _;
                }
                let mut memr = ash::vk::MemoryRequirements2::builder().build();
                device.get_image_memory_requirements2(&requirement_info, &mut memr);
                memr
            };
            let phy_mem_prop = instance.get_physical_device_memory_properties(physical);

            let fd_mem_type = if instance
                .get_device_proc_addr(
                    device.handle(),
                    CStr::from_bytes_with_nul(b"vkGetMemoryFdPropertiesKHR\0")
                        .unwrap()
                        .as_ptr(),
                )
                .is_some()
            {
                ExternalMemoryFd::new(instance, device)
                    .get_memory_fd_properties(
                        ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                        plane.fd.as_fd().as_raw_fd(),
                    )?
                    .memory_type_bits
            } else {
                !0
            };

            let mut fd_info = ash::vk::ImportMemoryFdInfoKHR::builder()
                .fd(plane.fd.as_fd().as_raw_fd())
                .handle_type(ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
                .build();

            let alloc_info = ash::vk::MemoryAllocateInfo::builder()
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
                .push_next(&mut fd_info)
                .build();
            let memory = device.allocate_memory(&alloc_info, None)?;

            let mut bind_info = BindImageMemoryInfo::builder()
                .image(image)
                .memory(memory)
                .memory_offset(0)
                .build();

            if planes.len() > 1 {
                let mut info = Box::new(
                    vk::BindImagePlaneMemoryInfo::builder()
                        .plane_aspect(MEM_PLANE_ASCPECT[i])
                        .build(),
                );
                bind_info.p_next = info.as_mut() as *mut _ as *mut _;
                plane_infos.push(bind_info);
            }

            bind_infos.push(bind_info);
            memorys.push(memory);
        }
        device.bind_image_memory2(&bind_infos)?;

        let fence = device.create_fence(&FenceCreateInfo::builder().build(), None)?;
        // buffer.render_image = RenderImage::Vulkan(Image { image, fence });

        Ok(ImportedImage {
            image,
            fence,
            memory: memorys,
            buffer_to_release: Some(buffer.raw.clone()),
            shm_pool: None,
        })
    }
}

pub fn create_shm_image(
    hal_device: &wgpu_hal::vulkan::Device,
    buffer: &WlShmBuffer,
) -> Result<ImportedImage> {
    let device = hal_device.raw_device();
    unsafe {
        let create_image_info = ash::vk::ImageCreateInfo::builder()
            .sharing_mode(SharingMode::EXCLUSIVE)
            .image_type(ImageType::TYPE_2D)
            .extent(Extent3D {
                width: buffer.size.x as u32,
                height: buffer.size.y as u32,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(convert_wl_format(buffer.format)?.0)
            .samples(SampleCountFlags::TYPE_1)
            .initial_layout(ImageLayout::UNDEFINED)
            .usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .flags(ImageCreateFlags::empty())
            .build();
        let image = device.create_image(&create_image_info, None)?;
        let req = device.get_image_memory_requirements(image);

        let index = req.memory_type_bits.trailing_zeros();
        if index == 32 {
            bail!(NoValidMemoryType);
        }
        let memory = device.allocate_memory(
            &mut vk::MemoryAllocateInfo::builder()
                .allocation_size(req.size.max(1))
                .memory_type_index(index)
                .build(),
            None,
        )?;
        device.bind_image_memory(image, memory, 0)?;

        let fence = device.create_fence(&FenceCreateInfo::builder().build(), None)?;

        Ok(ImportedImage {
            image,
            fence,
            memory: SmallVec::from_slice(&[memory]),
            buffer_to_release: Some(buffer.raw.lock().unwrap().clone()),
            shm_pool: Some(buffer.pool.clone()),
        })
    }
}

pub unsafe fn image_to_hal_texture(
    size: IVec2,
    texture_format: wgpu::TextureFormat,
    image: vk::Image,
) -> wgpu_hal::vulkan::Texture {
    wgpu_hal::vulkan::Device::texture_from_raw(
        image,
        &wgpu_hal::TextureDescriptor {
            label: Some("gbm renderbuffer"),
            size: Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth_or_array_layers: 1,
                ..Default::default()
            },
            dimension: TextureDimension::D2,
            format: texture_format,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUses::COLOR_TARGET
                | TextureUses::DEPTH_STENCIL_READ
                | TextureUses::DEPTH_STENCIL_WRITE,
            view_formats: vec![],
            memory_flags: MemoryFlags::empty(),
        },
        None,
    )
}

pub unsafe fn hal_texture_to_gpuimage(
    device: &wgpu::Device,
    size: IVec2,
    texture_format: wgpu::TextureFormat,
    hal_texture: wgpu_hal::vulkan::Texture,
) -> GpuImage {
    let wgpu_texture = device.create_texture_from_hal::<Vulkan>(
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
    let texture: wgpu::Texture = wgpu_texture;
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
    let sampler: wgpu::Sampler = device.create_sampler(&SamplerDescriptor {
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
    });
    GpuImage {
        texture: texture.into(),
        texture_view: texture_view.into(),
        texture_format,
        sampler: sampler.into(),
        size: size.as_vec2(),
        mip_level_count: 1,
    }
}

pub unsafe fn import_dma(
    device: &wgpu::Device,
    render_context: &mut CommandEncoder,
    buffer: &DmaBuffer,
    dest: vk::Image,
    surface: &WlSurface,
) -> Result<(), DWayRenderError> {
    Ok(())
}

pub unsafe fn import_shm(
    queue: &wgpu::Queue,
    shm_buffer: &WlShmBuffer,
    texture: &wgpu::Texture,
    surface: &WlSurface,
) -> Result<()> {
    let buffer_guard = shm_buffer.pool.read().unwrap();
    let size = shm_buffer.size;

    let data = std::ptr::from_raw_parts::<[u8]>(
        buffer_guard
            .ptr
            .as_ptr()
            .offset(shm_buffer.offset as isize)
            .cast(),
        (shm_buffer.stride * size.y) as usize,
    )
    .as_ref()
    .unwrap();

    let image_area = IRect::from_pos_size(IVec2::default(), size);
    let emit_rect = |rect: IRect| {
        let rect = rect.intersection(image_area);
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
                // offset: 4 * (shm_buffer.stride * rect.y() + rect.x()) as u64, // TODO
                offset: 0,
                bytes_per_row: (shm_buffer.stride as u32).try_into().ok(),
                rows_per_image: None,
            },
            Extent3d {
                width: rect.width() as u32,
                height: rect.height() as u32,
                depth_or_array_layers: 1,
            },
        );
    };
    emit_rect(image_area);
    // if surface.commited.damages.is_empty() { // TODO
    //     emit_rect(image_area);
    // } else {
    //     for &region in &surface.commited.damages {
    //         emit_rect(region);
    //     }
    // }

    Ok(())
}

pub fn prepare_wl_surface(
    state: &mut VulkanState,
    device: &wgpu::Device,
    surface: &WlSurface,
    shm_buffer: Option<&WlShmBuffer>,
    dma_buffer: Option<&DmaBuffer>,
    image_assets: &mut RenderAssets<bevy::render::texture::Image>,
) -> Result<()> {
    unsafe {
        if let Some(dma_buffer) = dma_buffer {
            match state.image_map.entry(dma_buffer.raw.clone()) {
                Entry::Occupied(mut o) => {
                    image_assets.insert(surface.image.clone(), o.get().1.clone());
                }
                Entry::Vacant(mut v) => {
                    let (size, format, image) = device.as_hal::<Vulkan, _, _>(|hal_device| {
                        let hal_device = hal_device.ok_or_else(|| BackendIsNotVulkan)?;
                        let size = dma_buffer.size;
                        let format = convert_drm_format(
                            DrmFourcc::try_from(dma_buffer.format)
                                .map_err(|e| Unknown(anyhow!("{e}")))?,
                        )?
                        .1;
                        let image = create_dma_image(hal_device, dma_buffer)?;
                        Result::<_, DWayRenderError>::Ok((size, format, image))
                    })?;
                    let hal_texture = image_to_hal_texture(size, format, image.image);
                    let gpu_image = hal_texture_to_gpuimage(device, size, format, hal_texture);
                    image_assets.insert(surface.image.clone(), gpu_image.clone());
                    v.insert((image, gpu_image));
                }
            };
        }
        Ok(())
    }
}

pub fn remove_image(
    state: &mut VulkanState,
    device: &wgpu::Device,
    image: &Handle<bevy::render::texture::Image>,
) -> Result<()> {
    todo!()
}

#[tracing::instrument(skip_all)]
pub fn import_wl_surface(
    surface: &WlSurface,
    shm_buffer: Option<&WlShmBuffer>,
    dma_buffer: Option<&DmaBuffer>,
    egl_buffer: Option<&UninitedWlBuffer>,
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    render_context: &mut RenderContext,
    state: &mut VulkanState,
) -> Result<(), DWayRenderError> {
    unsafe {
        let mut image = None;
        texture.as_hal::<Vulkan, _>(|texture| image = texture.map(|t| t.raw_handle()));
        let Some(image) = image else {
            return Err(BackendIsNotVulkan);
        };
        if let Some(shm_buffer) = shm_buffer {
            import_shm(queue, shm_buffer, texture, surface)?;
        }

        Ok(())
    }
}
