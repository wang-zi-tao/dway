use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use bevy::render::{
    render_resource::TextureView,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::GpuImage,
};
use image::RgbaImage;
use log::error;
use wgpu::{
    Origin3d, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
};

pub mod gles;
pub mod vulkan;

pub fn save_image(image: &RgbaImage, label: &str) {
    let mut file_path = PathBuf::from(".output");
    let time = chrono::Local::now().naive_local();
    file_path.push(label);

    if !std::fs::exists(&file_path).unwrap() {
        std::fs::create_dir_all(&file_path).unwrap();
    }

    file_path.push(time.to_string());
    file_path.set_extension(".png");

    image.save(&file_path).unwrap();
}

/**
* output bevy texture to image file
*/
pub fn output_image(
    texture: &GpuImage,
    path: &Path,
    render_context: &mut RenderContext,
    render_device: &RenderDevice,
) -> anyhow::Result<()> {
    let format = texture.texture.format();
    let size = texture.size;

    let buffer = Arc::new(render_device.create_buffer(
        &bevy::render::render_resource::BufferDescriptor {
            label: Some("output_texture_buffer"),
            size: (4 * texture.size.width * texture.size.height) as u64,
            usage: bevy::render::render_resource::BufferUsages::COPY_DST
                | bevy::render::render_resource::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        },
    ));

    let bytes_pre_row = texture.size.width * 4;

    render_context.command_encoder().copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture: &texture.texture,
            mip_level: 1,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_pre_row as u32),
                rows_per_image: Some(texture.size.height as u32),
            },
        },
        texture.size,
    );

    let buffer_clone = buffer.clone();
    let path = path.to_path_buf();
    render_device.map_buffer(&buffer.slice(..), wgpu::MapMode::Read, move |r| {
        let buffer = buffer_clone;
        if let Err(e) = r {
            panic!("Failed to map buffer for reading: {:?}", e);
        }

        let inner = || {
            let buffer_view = buffer.slice(..).get_mapped_range();
            let image = RgbaImage::from_raw(size.width, size.height, buffer_view.to_vec())
                .ok_or_else(|| anyhow::anyhow!("Failed to create image from buffer data"))?;
            image.save(path)?;

            Ok(())
        };

        let result: anyhow::Result<()> = inner();
        buffer.unmap();

        if let Err(e) = result {
            error!("Failed to process image data: {:?}", e);
        }
    });

    Ok(())
}
