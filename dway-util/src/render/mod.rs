use std::{path::PathBuf, time::SystemTime};

use image::RgbaImage;

pub mod vulkan;
pub mod gles;

pub fn save_image(image: &RgbaImage, label:&str){
    let mut  file_path = PathBuf::from(".output");
    let time = chrono::Local::now().naive_local();
    file_path.push(label);

    if !std::fs::exists(&file_path).unwrap(){
        std::fs::create_dir_all(&file_path).unwrap();
    }

    file_path.push(time.to_string());
    file_path.set_extension(".png");

    image.save(&file_path).unwrap();
}
