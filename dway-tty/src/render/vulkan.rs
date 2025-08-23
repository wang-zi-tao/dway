use crate::gbm::{
        buffer::{GbmBuffer},
        SUPPORTED_FORMATS,
    };
use anyhow::{anyhow, bail, Result};
use ash::{
    khr::external_memory_fd, vk::{self, *}
};
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use smallvec::SmallVec;
use std::os::fd::{AsFd, AsRawFd, IntoRawFd};
use wgpu::{Extent3d, TextureDimension, TextureFormat};
use wgpu_hal::{api::Vulkan, vulkan::Texture, MemoryFlags, TextureUses};

pub const MEM_PLANE_ASCPECT: [ImageAspectFlags; 4] = [
    ImageAspectFlags::MEMORY_PLANE_0_EXT,
    ImageAspectFlags::MEMORY_PLANE_1_EXT,
    ImageAspectFlags::MEMORY_PLANE_2_EXT,
    ImageAspectFlags::MEMORY_PLANE_3_EXT,
];

pub struct Image {
    pub device: ash::Device,
    pub image: vk::Image,
    pub memorys: SmallVec<[vk::DeviceMemory; 4]>,
    pub fence: vk::Fence,
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("image", &self.image)
            .field("memorys", &self.memorys)
            .field("fence", &self.fence)
            .finish()
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.fence, None);
            self.device.destroy_image(self.image, None);
            for memory in self.memorys.iter() {
                self.device.free_memory(*memory, None);
            }
        }
    }
}

pub fn convert_format(fourcc: DrmFourcc) -> Result<Format> {
    Ok(match fourcc {
        DrmFourcc::Argb8888 => Format::B8G8R8A8_SRGB,
        f => bail!("unknown format: {f}"),
    })
}

pub fn get_formats(render_device: &wgpu::Device) -> Option<Result<Vec<DrmFormat>>> {
    unsafe {
        render_device
            .as_hal::<Vulkan, _, _>(|hal_device| {
                hal_device.map(|hal_device| {
                    let instance = hal_device.shared_instance().raw_instance();
                    let raw_phy = hal_device.raw_physical_device();

                    let mut formats = Vec::new();

                    for fourcc in SUPPORTED_FORMATS {
                        let vk_format = convert_format(fourcc)?;

                        let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
                        let mut format_properties2 =
                            vk::FormatProperties2::default().push_next(&mut list);
                        instance.get_physical_device_format_properties2(
                            raw_phy,
                            vk_format,
                            &mut format_properties2,
                        );
                        let count = list.drm_format_modifier_count;
                        let mut data = vec![Default::default(); count as usize];

                        let mut list = vk::DrmFormatModifierPropertiesListEXT {
                            p_drm_format_modifier_properties: data.as_mut_ptr(),
                            drm_format_modifier_count: count,
                            ..Default::default()
                        };
                        let mut format_properties2 =
                            vk::FormatProperties2::default().push_next(&mut list);
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

                    Ok(formats)
                })
            })
    }
}

//pub fn reset_framebuffer(
//    render_device: &wgpu::Device,
//    buffer: &mut GbmBuffer,
//) -> Option<Result<()>> {
//    unsafe {
//        render_device
//            .as_hal::<Vulkan, _, _>(|hal_device| {
//                hal_device.map(|hal_device: &wgpu_hal::vulkan::Device| {
//                    let device = hal_device.raw_device();
//                    if let RenderImage::Vulkan(image) = &mut buffer.render_image {
//                        trace!(fence=?image.fence,"reset fence");
//                        device.reset_fences(&[image.fence])?;
//                    }
//                    Ok(())
//                })
//            })
//            .flatten()
//    }
//}

pub fn create_framebuffer_texture(
    hal_device: &wgpu_hal::vulkan::Device,
    buffer: &mut GbmBuffer,
) -> Result<Texture> {
    let instance = hal_device.shared_instance().raw_instance();
    let device = hal_device.raw_device();
    let physical = hal_device.raw_physical_device();

    unsafe {
        let plane_layouts: Vec<_> = buffer
            .planes
            .iter()
            .map(|plane| {
                SubresourceLayout::default()
                    .offset(plane.offset as u64)
                    .row_pitch(plane.stride as u64)
            })
            .collect();

        let format = convert_format(buffer.format)?;

        let mut drm_info = ash::vk::ImageDrmFormatModifierExplicitCreateInfoEXT::default()
            .drm_format_modifier(buffer.modifier.into())
            .plane_layouts(&plane_layouts);
        let mut dmabuf_info = ash::vk::ExternalMemoryImageCreateInfoKHR::default()
            .handle_types(ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let create_image_info = ash::vk::ImageCreateInfo::default()
            .sharing_mode(SharingMode::EXCLUSIVE)
            .image_type(ImageType::TYPE_2D)
            .extent(Extent3D {
                width: buffer.size.x as u32,
                height: buffer.size.y as u32,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .samples(SampleCountFlags::TYPE_1)
            .initial_layout(ImageLayout::PREINITIALIZED)
            .usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .flags({
                let mut flags = ImageCreateFlags::default();
                if buffer.planes.len() > 1 {
                    flags |= ImageCreateFlags::DISJOINT;
                };
                flags
            })
            .push_next(&mut dmabuf_info)
            .push_next(&mut drm_info);
        let image = device.create_image(&create_image_info, None)?;

        let mut plane_infos = Vec::with_capacity(buffer.planes.len());
        let mut bind_infos = Vec::with_capacity(buffer.planes.len());
        let mut memorys = SmallVec::<[vk::DeviceMemory; 4]>::default();
        for (i, plane) in buffer.planes.iter().enumerate() {
            let memory_requirement = {
                let mut requirement_info = ash::vk::ImageMemoryRequirementsInfo2::default()
                    .image(image);
                let mut plane_requirement_info =
                    ash::vk::ImagePlaneMemoryRequirementsInfo::default()
                        .plane_aspect(MEM_PLANE_ASCPECT[i]);
                if buffer.planes.len() > 1 {
                    requirement_info.p_next = &mut plane_requirement_info
                        as *mut ImagePlaneMemoryRequirementsInfo
                        as *mut _;
                }
                let mut memr = ash::vk::MemoryRequirements2::default();
                device.get_image_memory_requirements2(&requirement_info, &mut memr);
                memr
            };
            let phy_mem_prop = instance.get_physical_device_memory_properties(physical);

            let fd_mem_type = if instance
                .get_device_proc_addr(
                    device.handle(),
                    c"vkGetMemoryFdPropertiesKHR"
                        .as_ptr(),
                )
                .is_some()
            {
                let mut fd = MemoryFdPropertiesKHR::default();
                external_memory_fd::Device::new(instance, device)
                    .get_memory_fd_properties(
                        ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                        plane.fd.as_fd().as_raw_fd(),
                        &mut fd,
                    )?;
                fd.memory_type_bits
            } else {
                !0
            };

            let mut fd_info = ash::vk::ImportMemoryFdInfoKHR::default()
                .fd(plane.fd.try_clone()?.into_raw_fd())
                .handle_type(ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);

            let mut dedicated_info = ash::vk::MemoryDedicatedAllocateInfo::default()
                .image(image);

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
                        .ok_or_else(|| anyhow!("no valid memory type index"))?,
                )
                .push_next(&mut fd_info)
                .push_next(&mut dedicated_info);
            let memory = device.allocate_memory(&alloc_info, None).unwrap();
            memorys.push(memory);

            let mut bind_info = BindImageMemoryInfo::default()
                .image(image)
                .memory(memory)
                .memory_offset(0);

            if buffer.planes.len() > 1 {
                let mut info = Box::new(
                    vk::BindImagePlaneMemoryInfo::default()
                        .plane_aspect(MEM_PLANE_ASCPECT[i]),
                );
                bind_info.p_next = info.as_mut() as *mut _ as *mut _;
                plane_infos.push(info);
            }

            bind_infos.push(bind_info);
        }
        device.bind_image_memory2(&bind_infos)?;

        let fence = device.create_fence(&FenceCreateInfo::default(), None)?;
        //buffer.render_image = RenderImage::Vulkan(Image {
        //    device: device.clone(),
        //    image,
        //    fence,
        //    memorys,
        //});

        Ok(wgpu_hal::vulkan::Device::texture_from_raw(
            image,
            &wgpu_hal::TextureDescriptor {
                label: Some("gbm renderbuffer"),
                size: Extent3d {
                    width: buffer.size.x as u32,
                    height: buffer.size.y as u32,
                    depth_or_array_layers: 1,
                    ..Default::default()
                },
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8Unorm,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUses::COLOR_TARGET
                    | TextureUses::DEPTH_STENCIL_READ
                    | TextureUses::DEPTH_STENCIL_WRITE,
                view_formats: vec![],
                memory_flags: MemoryFlags::empty(),
            },
            None,
        ))
    }
}

