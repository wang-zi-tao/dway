use bevy::prelude::*;
use glow::{HasContext, NativeTexture, PixelPackData};
use image::{ImageBuffer, Rgba, RgbaImage};
use wgpu::Texture;

use crate::formats::ImageFormat;

pub fn get_gpu_image(texture: &Texture, image: NativeTexture, gl: &glow::Context) -> RgbaImage {
    let format: wgpu::TextureFormat = texture.format();
    let size = texture.size().width as usize
        * texture.size().height as usize
        * format.components() as usize
        * size_of::<u8>();
    let mut buffer = vec![0u8; size];
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(image));
        gl.read_pixels(
            0,
            0,
            texture.width() as i32,
            texture.height() as i32,
            ImageFormat::from_wgpu(format).unwrap().gles_format,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(&mut buffer),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
    RgbaImage::from_vec(texture.width(), texture.height(), buffer).unwrap()
}

pub fn debug_output_texture(name: &str, gl: &glow::Context, texture: NativeTexture, size: IVec2) {
    unsafe {
        let framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        let mut buffer = vec![0u8; 4 * size.x as usize * size.y as usize];
        gl.read_pixels(
            0,
            0,
            size.x,
            size.y,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelPackData::Slice(&mut buffer[..]),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        debug!(
            "avg value: {}",
            buffer.iter().map(|v| *v as usize).sum::<usize>() / buffer.len()
        );
        let image: ImageBuffer<Rgba<u8>, Vec<_>> =
            ImageBuffer::from_vec(size.x as u32, size.y as u32, buffer).unwrap();
        let snapshtip_count = std::fs::read_dir(".snapshot").unwrap().count();
        let path = format!(".snapshot/{name}_{}.png", snapshtip_count + 1);
        info!("take snapshot, save at {path}");
        image.save(&path).unwrap();
    }
}
