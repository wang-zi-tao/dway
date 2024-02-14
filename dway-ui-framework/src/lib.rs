pub mod animation;
pub mod prelude;
pub mod shader;
pub mod theme;
pub mod widgets;
pub mod render;
use crate::prelude::*;
pub use dway_ui_derive::*;

pub struct UiFrameworkPlugin;
impl Plugin for UiFrameworkPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}

#[cfg(test)]
pub mod tests {
    use std::path::{Path, PathBuf};

    use image::{open, DynamicImage, GenericImageView, Rgba};

    use self::shader::Material;

    use super::*;

    fn render(app: &mut App) -> DynamicImage {
        todo!()
    }

    pub fn compare_image(
        src: &Path,
        dest: &Path,
        tmp: &Path,
    ) -> Result<Option<PathBuf>, anyhow::Error> {
        println!("compare_image({src:?}, {dest:?})");
        println!("loading image: {src:?}");
        let src_iamge: DynamicImage = open(src)?;
        println!("loading image: {dest:?}");
        let dest_iamge: DynamicImage = open(dest)?;
        'l: {
            if src_iamge.width() == dest_iamge.width() || src_iamge.height() == src_iamge.height() {
                let width = src_iamge.width();
                let height = src_iamge.height();
                for y in 0..height {
                    for x in 0..width {
                        let src_pixel =
                            Vec4::from_array(src_iamge.get_pixel(x, y).0.map(|m| m as f32 / 256.0));
                        let dest_pixel = Vec4::from_array(
                            dest_iamge.get_pixel(x, y).0.map(|m| m as f32 / 256.0),
                        );
                        let diff = ( src_pixel - dest_pixel ).abs().max_element();
                        if diff > 4.0 / 256.0 {
                            break 'l;
                        }
                    }
                }
                return Ok(None);
            }
        }
        println!("image is different");
        let diff_image = image_diff::diff(&dest_iamge, &src_iamge)?;
        let mut tmp = tmp.to_owned();
        tmp.push("diff.png");
        diff_image.save(&tmp)?;
        Ok(Some(tmp))
    }
}
