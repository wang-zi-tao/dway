use crate::prelude::*;
use bevy::prelude::*;
pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod combobox;
pub mod drag;
pub mod inputbox;
pub mod popup;
pub mod rightclick_popup;
pub mod scroll;
pub mod shape;
pub mod slider;
pub mod svg;
pub mod text;
pub mod tips;

#[derive(Component, Debug, Clone, Deref, Reflect)]
#[require(Node)]
pub struct UiWidgetRoot(Entity);

impl From<Entity> for UiWidgetRoot {
    fn from(value: Entity) -> Self {
        Self(value)
    }
}

pub mod zoffset{
    pub const TEXT_SELECTION: f32 = -1.1;
}

pub mod zindex {
    pub const TEXT_SELECTION: i32 = -8;
}

pub mod util {
    use crate::prelude::*;

    pub fn visibility(value: bool) -> Visibility {
        if value {
            Visibility::Visible
        } else {
            Visibility::Hidden
        }
    }
}

pub mod shader {
    use crate::{
        prelude::*,
        shader::{
            effect::{Arc, Border, Fake3D, InnerShadow, Shadow},
            fill::{ColorWheel, FillColor, FillImage},
            shape::{Circle, Rect, RoundedBar, RoundedRect, Shape},
            transform::Margins,
            Material, ShaderAsset, ShapeRender, Transformed,
        },
    };

    pub type HollowBlockMaterial = ShaderAsset<ShapeRender<RoundedRect, Border>>;
    pub fn hollow_block(color: Color, corner: f32, width: f32) -> HollowBlockMaterial {
        ShapeRender::new(RoundedRect::new(corner), Border::new(color, width)).into()
    }

    pub type RoundedBlockMaterial = ShaderAsset<ShapeRender<RoundedRect, (FillColor, Shadow)>>;
    pub fn rounded_block(color: Color, corner: f32, theme: &Theme) -> RoundedBlockMaterial {
        RoundedRect::new(corner)
            .with_effect((FillColor::new(color), theme.default_shadow_material()))
            .into()
    }

    pub type RoundedBorderBlockMaterial =
        ShaderAsset<ShapeRender<RoundedRect, (Border, FillColor)>>;
    pub fn rounded_border_block(
        color: Color,
        border_color: Color,
        corner: f32,
        width: f32,
    ) -> RoundedBorderBlockMaterial {
        RoundedRect::new(corner)
            .with_effect((Border::new(border_color, width), FillColor::new(color)))
            .into()
    }

    pub type RoundedInnerShadowBlockMaterial =
        ShaderAsset<ShapeRender<RoundedRect, InnerShadow<FillColor>>>;
    pub fn rounded_inner_shadow_block(
        color: Color,
        corner: f32,
        theme: &Theme,
    ) -> RoundedInnerShadowBlockMaterial {
        RoundedRect::new(corner)
            .with_effect(theme.default_inner_shadow_material(FillColor::new(color)))
            .into()
    }

    pub type RoundedRainbowBlockMaterial =
        ShaderAsset<ShapeRender<RoundedRect, Border<ColorWheel>>>;
    pub fn rainbow_block(corner: f32, width: f32) -> RoundedRainbowBlockMaterial {
        ShapeRender::new(
            RoundedRect::new(corner),
            Border::with_filler(ColorWheel::default(), width),
        )
        .into()
    }

    pub type RoundedUiRectMaterial = ShaderAsset<ShapeRender<RoundedRect, FillColor>>;
    pub fn rounded_rect(color: Color, corner: f32) -> RoundedUiRectMaterial {
        ShapeRender::new(RoundedRect::new(corner), FillColor::new(color)).into()
    }

    pub type UiImageMaterial = ShaderAsset<ShapeRender<Rect, FillImage>>;
    pub fn ui_image(image: Handle<Image>) -> UiImageMaterial {
        Rect::new().with_effect(FillImage::from(image)).into()
    }
    pub type RoundedUiImageMaterial = ShaderAsset<ShapeRender<RoundedRect, FillImage>>;
    pub fn rounded_ui_image(
        corner: f32,
        offset_uv: Vec2,
        size_uv: Vec2,
        image: Handle<Image>,
    ) -> RoundedUiImageMaterial {
        RoundedRect::new(corner)
            .with_effect(FillImage::new(offset_uv, size_uv, image))
            .into()
    }

    pub type ButtonMaterial = ShaderAsset<ShapeRender<RoundedRect, (FillColor, Shadow)>>;
    pub fn button_material(color: Color, corner: f32, theme: &Theme) -> ButtonMaterial {
        RoundedRect::new(corner)
            .with_effect((FillColor::new(color), theme.default_shadow_material()))
            .into()
    }

    pub type UiCircleMaterial = ShaderAsset<ShapeRender<Circle, FillColor>>;
    pub fn circle_material(color: Color) -> UiCircleMaterial {
        ShapeRender::new(Circle::new(), FillColor::new(color)).into()
    }

    pub type Fake3dButton = ShaderAsset<ShapeRender<RoundedRect, (Fake3D, FillColor)>>;
    pub fn fake3d_button_material(color: Color, corner: f32) -> Fake3dButton {
        RoundedRect::new(corner)
            .with_effect((
                Fake3D::new(color, Vec3::new(1.0, 1.0, 1.0).normalize(), corner),
                FillColor::new(color),
            ))
            .into()
    }
    pub fn clicked_fake3d_button_material(color: Color, corner: f32) -> Fake3dButton {
        RoundedRect::new(corner)
            .with_effect((
                Fake3D::new(color, Vec3::new(-1.0, -1.0, 1.0).normalize(), corner),
                FillColor::new(color),
            ))
            .into()
    }

    pub type CheckboxMaterial = ShaderAsset<(
        Transformed<ShapeRender<Circle, (Border, FillColor)>, Margins>,
        ShapeRender<RoundedBar, (Border, FillColor, Shadow)>,
    )>;
    pub fn checkbox_material(state: bool, size: Vec2, theme: &Theme) -> CheckboxMaterial {
        let ui_color = theme.color("checkbox:handle");
        let shadow = theme.default_shadow_material();
        (
            Circle::default()
                .with_effect((Border::new(ui_color, 2.0), ui_color.into()))
                .with_transform(if state {
                    Margins::new(0.5 * size.x + 5.0, 5.0, 5.0, 5.0)
                } else {
                    Margins::new(5.0, 0.5 * size.x + 5.0, 5.0, 5.0)
                }),
            RoundedBar::default().with_effect((
                Border::new(theme.color("checkbox:bar"), 3.0),
                Color::WHITE.into(),
                shadow.clone(),
            )),
        )
            .into_asset()
    }

    pub type ArcMaterial = ShaderAsset<Transformed<ShapeRender<Arc, (Border, FillColor)>, Margins>>;
    pub fn arc_material(
        color: Color,
        border_color: Color,
        width: f32,
        angle: [f32; 2],
    ) -> ArcMaterial {
        Arc::new(angle, width)
            .with_effect((
                Border::new(border_color, 0.25 * width),
                FillColor::new(color),
            ))
            .with_transform(Margins::all(width))
            .into()
    }
}
