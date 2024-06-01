use crate::prelude::*;
pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod combobox;
pub mod inputbox;
pub mod popup;
pub mod rightclick_popup;
pub mod scroll;
pub mod shape;
pub mod slider;
pub mod svg;
pub mod text;
pub mod tips;

pub trait WidgetTemplate<Input> {
    fn spawn(input: &Input, commands: &mut Commands);
}

#[derive(Component, Default)]
pub struct Callback(pub Option<SystemId>);

pub mod bundles {
    use bevy::ui::{widget::UiImageSize, ContentSize};

    use crate::{
        prelude::*,
        theme::{StyleFlags, ThemeComponent, WidgetKind},
    };

    #[macro_export]
    macro_rules! make_bundle {
        ($name:ident {$($tt:tt)*}) => {

            #[derive(Bundle, SmartDefault)]
            pub struct $name {
                pub node: Node,
                pub style: Style,
                pub transform: Transform,
                pub global_transform: GlobalTransform,
                pub visibility: Visibility,
                pub inherited_visibility: InheritedVisibility,
                pub view_visibility: ViewVisibility,
                pub z_index: ZIndex,
                $($tt)*
            }
        };
        (@from $field:ident:$component:ident -> $bundle:ident) => {
            impl From<$component> for $bundle {
                fn from($field: $component) -> Self {
                    Self {
                        $field,
                        ..default()
                    }
                }
            }
        };
        (@from $field:ident:$component:ident, @addon $addon_name:ident, $bundle:ident {$($tt:tt)*}) => {
            make_bundle!(@addon $addon_name, $bundle {$($tt)*});
            make_bundle!(@from $field:$component->$addon_name);
            make_bundle!(@from $field:$component->$bundle);
        };
        (@addon $addon_name:ident, $name:ident {$($tt:tt)*}) => {
            make_bundle!{
                $name {$($tt)*}
            }

            #[derive(Bundle, SmartDefault)]
            pub struct $addon_name {
                $($tt)*
            }
        };
        (@material2d $name:ident {$($tt:tt)*}) => {
            #[derive(Bundle, SmartDefault)]
            pub struct $name<M: Material2d = ColorMaterial> {
                pub node: Node,
                pub material: Handle<M>,
                pub style: Style,
                pub transform: Transform,
                pub global_transform: GlobalTransform,
                pub visibility: Visibility,
                pub inherited_visibility: InheritedVisibility,
                pub view_visibility: ViewVisibility,
                pub z_index: ZIndex,
                $($tt)*
            }
        };
        (@material $name:ident {$($tt:tt)*}) => {
            #[derive(Bundle, SmartDefault)]
            pub struct $name<M: UiMaterial> {
                pub node: Node,
                pub material: Handle<M>,
                pub style: Style,
                pub transform: Transform,
                pub global_transform: GlobalTransform,
                pub visibility: Visibility,
                pub inherited_visibility: InheritedVisibility,
                pub view_visibility: ViewVisibility,
                pub z_index: ZIndex,
                $($tt)*
            }
        };
    }

    make_bundle!(
        @from image: UiImage,
        @addon UiImageExt,
        UiImageBundle {
            pub image: UiImage,
            pub image_size: UiImageSize,
            pub focus_policy: FocusPolicy,
            pub calculated_size: ContentSize,
            pub background_color: BackgroundColor,
        }
    );
    make_bundle!(UiNodeBundle {
        pub focus_policy: FocusPolicy,
    });
    make_bundle!(UiBlockBundle {
        pub focus_policy: FocusPolicy,
        #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Block))]
        pub theme: ThemeComponent,
    });
    make_bundle!(UiHollowBlockBundle {
        pub focus_policy: FocusPolicy,
        #[default(ThemeComponent::new(StyleFlags::HOLLOW, WidgetKind::Block))]
        pub theme: ThemeComponent,
    });
    make_bundle!(UiSunkenBlockBundle {
        pub focus_policy: FocusPolicy,
        #[default(ThemeComponent::new(StyleFlags::SUNKEN, WidgetKind::Block))]
        pub theme: ThemeComponent,
    });
    make_bundle!(UiHighlightBlockBundle {
        pub focus_policy: FocusPolicy,
        #[default(ThemeComponent::new(StyleFlags::HIGHLIGHT, WidgetKind::Block))]
        pub theme: ThemeComponent,
    });

    make_bundle!(MiniNodeBundle {
        pub focus_policy: FocusPolicy,
    });

    make_bundle!(MiniButtonBundle {
        pub button: UiButton,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
        pub interaction: Interaction,
    });
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
            shape::{Circle, RoundedBar, RoundedRect, Shape},
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
