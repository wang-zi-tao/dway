#![feature(round_char_boundary)]

pub mod animation;
pub mod assets;
pub mod input;
pub mod prelude;
pub mod render;
pub mod shader;
pub mod theme;
pub mod widgets;
use crate::{
    prelude::*,
    render::mesh::{UiMeshMaterialPlugin, UiMeshPlugin},
    widgets::{
        button::UiButton,
        checkbox::UiCheckBox,
        slider::UiSlider,
        svg::{uisvg_update_system, SvgMagerial, UiSvg},
    },
};
use bevy::{sprite::Material2dPlugin, ui::UiSystem};
use bevy_svg::SvgPlugin;
pub use dway_ui_derive::*;

pub struct UiFrameworkPlugin;
impl Plugin for UiFrameworkPlugin {
    fn build(&self, app: &mut App) {
        use UiFrameworkSystems::*;
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        app.add_plugins((
            assets::UiAssetsPlugin,
            theme::ThemePlugin,
            theme::flat::FlatThemePlugin::default(),
            render::mesh::UiMeshPlugin,
            shader::ShaderFrameworkPlugin,
            render::mesh::UiMeshMaterialPlugin::<ColorMaterial>::default(),
            animation::AnimationPlugin,
        ))
        .add_plugins((
            widgets::slider::UiSliderPlugin,
            widgets::scroll::UiScrollPlugin,
            widgets::inputbox::UiInputBoxPlugin,
            UiMeshMaterialPlugin::<SvgMagerial>::default(),
        ))
        .register_type::<UiCheckBox>()
        .register_type::<UiSlider>()
        .register_type::<UiButton>()
        .register_type::<UiSvg>()
        .init_asset::<SvgMagerial>()
        .register_type::<input::MousePosition>()
        .init_resource::<input::MousePosition>()
        .register_type::<input::UiFocusState>()
        .init_resource::<input::UiFocusState>()
        .add_event::<input::UiFocusEvent>()
        .register_type::<input::UiFocusEvent>()
        .add_systems(
            PreUpdate,
            (
                input::update_mouse_position
                    .run_if(on_event::<CursorMoved>())
                    .in_set(InputSystems),
                update_ui_input.in_set(InputSystems),
                widgets::button::process_ui_button_event.in_set(WidgetInputSystems),
                widgets::checkbox::process_ui_checkbox_event.in_set(WidgetInputSystems),
                widgets::inputbox::process_ui_inputbox_event.in_set(WidgetInputSystems),
            ),
        )
        .add_systems(
            PostUpdate,
            (widgets::svg::uisvg_update_system.in_set(UpdateWidgets),),
        )
        .configure_sets(
            PostUpdate,
            (UpdateWidgets, UpdatePopup, UpdateTheme, ApplyAnimation)
                .before(UiSystem::Layout)
                .chain(),
        )
        .add_plugins((
            RoundedUiRectMaterial::plugin(),
            UiCircleMaterial::plugin(),
            RoundedUiImageMaterial::plugin(),
            RoundedBlockMaterial::plugin(),
            RoundedBorderBlockMaterial::plugin(),
            HollowBlockMaterial::plugin(),
            ButtonMaterial::plugin(),
            RoundedRainbowBlockMaterial::plugin(),
            Fake3dButton::plugin(),
            CheckboxMaterial::plugin(),
            RoundedInnerShadowBlockMaterial::plugin(),
            ArcMaterial::plugin(),
            AssetAnimationPlugin::<CheckboxMaterial>::default(),
        ));
    }
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum UiFrameworkSystems {
    InputSystems,
    WidgetInputSystems,
    UpdateWidgets,
    UpdatePopup,
    UpdateTheme,
    ApplyAnimation,
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
                        let diff = (src_pixel - dest_pixel).abs().max_element();
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
