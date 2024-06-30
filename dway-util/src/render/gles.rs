use bevy::{prelude::*, render::texture::TextureFormatPixelInfo};
use glow::{HasContext, NativeTexture};
use image::RgbaImage;
use wgpu::Texture;

use crate::formats::ImageFormat;

pub fn get_gpu_image(texture: &Texture, image: NativeTexture, gl: &glow::Context) -> RgbaImage {
    let size = texture.size().width as usize
        * texture.size().height as usize
        * texture.format().pixel_size();
    let mut buffer = vec![0u8; size];
    let format: wgpu::TextureFormat = texture.format();
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
