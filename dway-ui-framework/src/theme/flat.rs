use super::{insert_material_tween, StyleFlags, ThemeDispatch};
use crate::animation::AnimationEaseMethod;
use crate::shader::effect::Border;
use crate::shader::fill::Fill;
use crate::shader::shape::{RoundedBar, Shape};
use crate::shader::transform::Margins;
use crate::shader::{ShaderAsset, ShaderPlugin, Transformed};
use crate::{
    prelude::*,
    shader::{
        effect::{InnerShadow, Shadow},
        fill::FillColor,
        shape::{Circle, RoundedRect},
        ShapeRender,
    },
};

type BlockMaterial = ShapeRender<RoundedRect, (FillColor, Shadow)>;
type HollowBlockMaterial = ShapeRender<RoundedRect, Border>;
type SunkenBlockMaterial = ShapeRender<RoundedRect, InnerShadow<FillColor>>;
type HightlightButtonMaterial = ShapeRender<RoundedRect, (Border, FillColor, Shadow)>;
type ListItemMaterial = ShapeRender<RoundedRect, FillColor>;
type ButtonMaterial = ShapeRender<RoundedRect, (InnerShadow<FillColor>, FillColor, Shadow)>;
type CheckboxMaterial = (
    Transformed<ShapeRender<Circle, FillColor>, Margins>,
    ShapeRender<RoundedBar, (FillColor, Shadow)>,
);
type SliderMaterial = ShapeRender<RoundedBar, (InnerShadow<FillColor>, FillColor)>;
type SliderHightlightBarMaterial = ShapeRender<RoundedBar, FillColor>;
type SliderHandlerMaterial = ShapeRender<Circle, (Border, FillColor, Shadow)>;
type InputboxMaterial = ShapeRender<RoundedRect, (Border, FillColor)>;
type ScrollBarMaterial = ShapeRender<RoundedRect, FillColor>;

#[derive(SmartDefault, Clone)]
pub struct FlatTheme {
    #[default(color!("#2777ff"))]
    pub main_color: Color,
    #[default(2.0)]
    pub border_width: f32,
    #[default(color!("#ffffff"))]
    pub fill_color: Color,
    #[default(color!("#eeeeee"))]
    pub fill_color2: Color,
    #[default(color!("#dddddd"))]
    pub fill_color3: Color,
    #[default(16.0)]
    pub block_cornor: f32,
    #[default(8.0)]
    pub cornor: f32,
    #[default(color!("#888888"))]
    pub shadow_color: Color,
    #[default(Vec2::new(0.0, 1.0))]
    pub shadow_offset: Vec2,
    #[default(Vec2::new(1.0, 1.0))]
    pub shadow_margin: Vec2,
    #[default(color!("#888888"))]
    pub inner_shadow_color: Color,
    #[default(Vec2::new(1.0, 1.0))]
    pub inner_shadow_offset: Vec2,
    #[default(1.0)]
    pub inner_shadow_radius: f32,
    #[default(2.0)]
    pub shadow_radius: f32,
    #[default(Duration::from_secs_f32(0.2))]
    pub animation_duration: Duration,
    // #[default(AnimationEaseMethod::EaseFunction(EaseFunction::QuadraticIn))]
    #[default(AnimationEaseMethod::Linear)]
    pub animation_ease: AnimationEaseMethod,

    pub block_material: Handle<ShaderAsset<BlockMaterial>>,
    pub popup_block_material: Handle<ShaderAsset<BlockMaterial>>,
    pub hollow_block_material: Handle<ShaderAsset<HollowBlockMaterial>>,
    pub hightlight_hollow_block_material: Handle<ShaderAsset<HollowBlockMaterial>>,
    pub sunken_block_material: Handle<ShaderAsset<SunkenBlockMaterial>>,
    pub hightlight_block_material: Handle<ShaderAsset<HollowBlockMaterial>>,
    pub button_material: Handle<ShaderAsset<ButtonMaterial>>,
    pub button_material_hover: Handle<ShaderAsset<ButtonMaterial>>,
    pub button_material_clicked: Handle<ShaderAsset<ButtonMaterial>>,
    pub hightlight_button_material: Handle<ShaderAsset<HightlightButtonMaterial>>,
    pub hightlight_button_material_hover: Handle<ShaderAsset<HightlightButtonMaterial>>,
    pub hightlight_button_material_clicked: Handle<ShaderAsset<HightlightButtonMaterial>>,
    pub list_item_hightlight: Handle<ShaderAsset<ListItemMaterial>>,
    pub list_item_hover: Handle<ShaderAsset<ListItemMaterial>>,
    pub list_item: Handle<ShaderAsset<ListItemMaterial>>,
    pub checkbox_material: Handle<ShaderAsset<CheckboxMaterial>>,
    pub checkbox_material_down: Handle<ShaderAsset<CheckboxMaterial>>,
    pub checkbox_material_hover: Handle<ShaderAsset<CheckboxMaterial>>,
    pub checkbox_material_down_hover: Handle<ShaderAsset<CheckboxMaterial>>,
    pub slider_material: Handle<ShaderAsset<SliderMaterial>>,
    pub slider_hightlight_bar_material: Handle<ShaderAsset<SliderHightlightBarMaterial>>,
    pub slider_handler_material: Handle<ShaderAsset<SliderHandlerMaterial>>,
    pub slider_handler_material_clicked: Handle<ShaderAsset<SliderHandlerMaterial>>,
    pub inputbox_material: Handle<ShaderAsset<InputboxMaterial>>,
    pub inputbox_material_focused: Handle<ShaderAsset<InputboxMaterial>>,
    pub scroll_bar_material: Handle<ShaderAsset<ScrollBarMaterial>>,
    pub scroll_bar_material_hover: Handle<ShaderAsset<ScrollBarMaterial>>,
    pub scroll_bar_material_click: Handle<ShaderAsset<ScrollBarMaterial>>,
}
impl FlatTheme {
    fn init(&mut self, world: &mut World) {
        {
            self.block_material = world.resource_mut::<Assets<_>>().add(ShaderAsset::new(
                self.block_rounded_rect()
                    .with_effect((self.fill_color(), self.shadow())),
            ));
            self.popup_block_material = world.resource_mut::<Assets<_>>().add(ShaderAsset::new(
                self.popup_block_rounded_rect().with_effect((self.fill_color(), {
                    Shadow::new(
                        self.shadow_color,
                        self.shadow_offset * 0.0,
                        self.shadow_margin * 1.0,
                        self.shadow_radius * 1.0,
                    )
                })),
            ));
            self.hollow_block_material = world
                .resource_mut::<Assets<_>>()
                .add(ShaderAsset::new(self.block_rounded_rect().with_effect(
                    Border::new(self.shadow_color, self.border_width),
                )));
            self.hightlight_hollow_block_material =
                world
                    .resource_mut::<Assets<_>>()
                    .add(ShaderAsset::new(self.block_rounded_rect().with_effect(
                        Border::new(self.main_color, self.border_width),
                    )));
            self.sunken_block_material = world.resource_mut::<Assets<_>>().add(ShaderAsset::new(
                self.block_rounded_rect()
                    .with_effect(self.inner_shadow(self.fill_color())),
            ));
            self.hightlight_hollow_block_material =
                world
                    .resource_mut::<Assets<_>>()
                    .add(ShaderAsset::new(self.block_rounded_rect().with_effect(
                        Border::new(self.main_color, self.border_width),
                    )));
        }

        {
            let mut button_material_assets = world.resource_mut::<Assets<_>>();
            self.button_material =
                button_material_assets.add(ShaderAsset::new(self.rounded_rect().with_effect((
                    self.invisible_inner_shadow(self.fill_color()),
                    self.fill_color(),
                    self.shadow(),
                ))));
            self.button_material_hover =
                button_material_assets.add(ShaderAsset::new(self.rounded_rect().with_effect((
                    self.invisible_inner_shadow(FillColor::new(self.fill_color * 0.95)),
                    FillColor::new(self.fill_color * 0.95),
                    self.shadow(),
                ))));
            self.button_material_clicked =
                button_material_assets.add(ShaderAsset::new(self.rounded_rect().with_effect((
                    self.inner_shadow(FillColor::new(self.fill_color * 0.95)),
                    FillColor::new(self.fill_color * 0.95),
                    self.invisible_shadow(),
                ))));
        }

        {
            let mut hightlight_button_material = world.resource_mut::<Assets<_>>();
            self.hightlight_button_material_hover = hightlight_button_material.add(
                ShaderAsset::new(self.rounded_rect().with_effect((
                    self.border(),
                    self.main_color.into(),
                    self.shadow(),
                ))),
            );
            self.hightlight_button_material = hightlight_button_material.add(ShaderAsset::new(
                self.rounded_rect().with_effect((
                    self.border(),
                    self.main_color.into(),
                    self.shadow(),
                )),
            ));
            self.hightlight_button_material_clicked = hightlight_button_material.add(
                ShaderAsset::new(self.rounded_rect().with_effect((
                    self.border(),
                    self.main_color.into(),
                    self.invisible_shadow(),
                ))),
            );
        }
        {
            let mut list_item_materials = world.resource_mut::<Assets<_>>();
            self.list_item_hightlight = list_item_materials.add(ShaderAsset::new(
                self.rounded_rect().with_effect(self.main_color.into()),
            ));
            self.list_item_hover = list_item_materials.add(ShaderAsset::new(
                self.rounded_rect().with_effect(self.fill_color3.into()),
            ));
            self.list_item = list_item_materials.add(ShaderAsset::new(
                self.rounded_rect().with_effect(self.fill_color2.into()),
            ));
        }

        {
            let mut checkbox_material_assets = world.resource_mut::<Assets<_>>();
            self.checkbox_material = checkbox_material_assets.add(ShaderAsset::new((
                Circle::new()
                    .with_effect(self.fill_color())
                    .with_transform(Margins::new(1.0, 32.0, 1.0, 1.0)),
                RoundedBar::new().with_effect((
                    FillColor::new(self.fill_color * 0.9),
                    self.invisible_shadow(),
                )),
            )));
            self.checkbox_material_hover = checkbox_material_assets.add(ShaderAsset::new((
                Circle::new()
                    .with_effect(self.fill_color())
                    .with_transform(Margins::new(1.0, 32.0, 1.0, 1.0)),
                RoundedBar::new().with_effect((
                    FillColor::new(self.fill_color * 0.93),
                    self.invisible_shadow(),
                )),
            )));
            self.checkbox_material_down = checkbox_material_assets.add(ShaderAsset::new((
                Circle::new()
                    .with_effect(self.fill_color())
                    .with_transform(Margins::new(32.0, 1.0, 1.0, 1.0)),
                RoundedBar::new().with_effect((self.main_color.into(), self.shadow())),
            )));
            self.checkbox_material_down_hover = checkbox_material_assets.add(ShaderAsset::new((
                Circle::new()
                    .with_effect(self.fill_color())
                    .with_transform(Margins::new(32.0, 1.0, 1.0, 1.0)),
                RoundedBar::new()
                    .with_effect((FillColor::new(self.main_color * 1.1), self.shadow())),
            )));
        }

        self.slider_material =
            world
                .resource_mut::<Assets<_>>()
                .add(ShaderAsset::new(RoundedBar::new().with_effect((
                    self.inner_shadow(FillColor::new(self.fill_color2)),
                    FillColor::new(self.fill_color2),
                ))));
        self.slider_hightlight_bar_material = world.resource_mut::<Assets<_>>().add(
            ShaderAsset::new(RoundedBar::new().with_effect(FillColor::new(self.main_color))),
        );
        self.slider_handler_material =
            world
                .resource_mut::<Assets<_>>()
                .add(ShaderAsset::new(Circle::new().with_effect((
                    self.border(),
                    self.fill_color(),
                    self.shadow(),
                ))));
        self.slider_handler_material_clicked =
            world
                .resource_mut::<Assets<_>>()
                .add(ShaderAsset::new(Circle::new().with_effect((
                    self.border(),
                    self.fill_color(),
                    self.shadow(),
                ))));

        self.inputbox_material = world.resource_mut::<Assets<_>>().add(ShaderAsset::new(
            RoundedRect::new(0.5 * self.cornor)
                .with_effect((self.inactive_border(), self.fill_color())),
        ));
        self.inputbox_material_focused = world.resource_mut::<Assets<_>>().add(ShaderAsset::new(
            RoundedRect::new(0.5 * self.cornor).with_effect((self.border(), self.fill_color())),
        ));
    }

    fn inactive_border(&self) -> Border<FillColor> {
        Border::new(self.fill_color3, self.border_width * 0.5)
    }
    fn invisible_border(&self) -> Border<FillColor> {
        Border::new(self.fill_color, 0.0)
    }
    fn border(&self) -> Border<FillColor> {
        Border::new(self.main_color, self.border_width)
    }
    fn invisible_shadow(&self) -> Shadow {
        Shadow::new(Color::NONE, Vec2::ZERO, Vec2::ZERO, 0.0)
    }
    fn shadow(&self) -> Shadow {
        Shadow::new(
            self.shadow_color,
            self.shadow_offset,
            self.shadow_margin,
            self.shadow_radius,
        )
    }
    fn popup_block_rounded_rect(&self) -> RoundedRect {
        RoundedRect::new(self.block_cornor)
    }
    fn block_rounded_rect(&self) -> RoundedRect {
        RoundedRect::new(self.block_cornor)
    }
    fn rounded_rect(&self) -> RoundedRect {
        RoundedRect::new(self.cornor)
    }
    fn fill_color(&self) -> FillColor {
        FillColor::new(self.fill_color)
    }
    fn invisible_inner_shadow<F: Fill>(&self, filler: F) -> InnerShadow<F> {
        InnerShadow {
            filler,
            color: Color::NONE,
            offset: Vec2::ZERO,
            radius: 0.0,
        }
    }
    fn inner_shadow<F: Fill>(&self, filler: F) -> InnerShadow<F> {
        InnerShadow {
            filler,
            color: self.inner_shadow_color,
            offset: self.inner_shadow_offset,
            radius: self.inner_shadow_radius,
        }
    }
    fn apply_material_animation<M: Asset + Interpolation>(
        &self,
        entity: Entity,
        commands: &mut Commands,
        material: Handle<M>,
    ) {
        let duration = self.animation_duration;
        let ease = self.animation_ease.clone();
        commands.add(move |world: &mut World| {
            insert_material_tween(world, entity, material, duration, ease)
        });
    }
}
impl ThemeDispatch for FlatTheme {
    fn apply(&self, entity: Entity, theme: &super::ThemeComponent, commands: &mut Commands) {
        let flag = theme.style_flags;
        let hover = flag.contains(StyleFlags::HOVERED);
        let clicked = flag.contains(StyleFlags::CLICKED);
        match &theme.widget_kind {
            super::WidgetKind::None => {}
            super::WidgetKind::Block => {
                if flag.contains(StyleFlags::HIGHLIGHT) {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.hightlight_hollow_block_material.clone(),
                    );
                } else if flag.contains(StyleFlags::HOLLOW) {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.hollow_block_material.clone(),
                    );
                } else if flag.contains(StyleFlags::SUNKEN) {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.sunken_block_material.clone(),
                    );
                } else {
                    self.apply_material_animation(entity, commands, self.block_material.clone());
                }
            }
            super::WidgetKind::Button => {
                if flag.contains(StyleFlags::HIGHLIGHT) {
                    if hover {
                        self.apply_material_animation(
                            entity,
                            commands,
                            self.hightlight_button_material_hover.clone(),
                        );
                    } else if clicked {
                        self.apply_material_animation(
                            entity,
                            commands,
                            self.hightlight_button_material_clicked.clone(),
                        );
                    } else {
                        self.apply_material_animation(
                            entity,
                            commands,
                            self.hightlight_button_material.clone(),
                        );
                    }
                } else if hover {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.button_material_hover.clone(),
                    );
                } else if clicked {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.button_material_clicked.clone(),
                    );
                } else {
                    self.apply_material_animation(entity, commands, self.button_material.clone());
                }
            }
            super::WidgetKind::Checkbox => match (flag.contains(StyleFlags::DOWNED), hover) {
                (true, true) => {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.checkbox_material_down_hover.clone(),
                    );
                }
                (true, false) => {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.checkbox_material_down.clone(),
                    );
                }
                (false, true) => {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.checkbox_material_hover.clone(),
                    );
                }
                (false, false) => {
                    self.apply_material_animation(entity, commands, self.checkbox_material.clone());
                }
            },
            super::WidgetKind::Inputbox => {
                if flag.contains(StyleFlags::FOCUSED) {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.inputbox_material_focused.clone(),
                    );
                } else {
                    self.apply_material_animation(entity, commands, self.inputbox_material.clone());
                }
            }
            super::WidgetKind::Slider => {
                self.apply_material_animation(entity, commands, self.slider_material.clone());
            }
            super::WidgetKind::SliderHandle => {
                if clicked {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.slider_handler_material_clicked.clone(),
                    );
                } else {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.slider_handler_material.clone(),
                    );
                }
            }
            super::WidgetKind::SliderHightlightBar => {
                self.apply_material_animation(
                    entity,
                    commands,
                    self.slider_hightlight_bar_material.clone(),
                );
            }
            super::WidgetKind::Other(_) => {}
            super::WidgetKind::ComboBox(super::ComboBoxNodeKind::Root) => {
                if clicked {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.hightlight_hollow_block_material.clone(),
                    );
                } else {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.hollow_block_material.clone(),
                    );
                }
            }
            super::WidgetKind::ComboBox(super::ComboBoxNodeKind::Popup) => {
                self.apply_material_animation(entity, commands, self.popup_block_material.clone());
            }
            super::WidgetKind::ComboBox(super::ComboBoxNodeKind::Item) => {
                if flag.contains(StyleFlags::HIGHLIGHT) {
                    self.apply_material_animation(
                        entity,
                        commands,
                        self.list_item_hightlight.clone(),
                    );
                } else if hover || clicked {
                    self.apply_material_animation(entity, commands, self.list_item_hover.clone());
                } else {
                    self.apply_material_animation(entity, commands, self.list_item.clone());
                }
            }
            super::WidgetKind::ComboBox(_) => {}
        }
    }
}

#[derive(Default)]
pub struct FlatThemePlugin {
    theme: FlatTheme,
}
impl Plugin for FlatThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ShaderPlugin::<BlockMaterial>::default(),
            ShaderPlugin::<HollowBlockMaterial>::default(),
            ShaderPlugin::<SunkenBlockMaterial>::default(),
            ShaderPlugin::<HightlightButtonMaterial>::default(),
            ShaderPlugin::<ListItemMaterial>::default(),
            ShaderPlugin::<ButtonMaterial>::default(),
            ShaderPlugin::<CheckboxMaterial>::default(),
            ShaderPlugin::<SliderMaterial>::default(),
            ShaderPlugin::<SliderHightlightBarMaterial>::default(),
            ShaderPlugin::<SliderHandlerMaterial>::default(),
            ShaderPlugin::<InputboxMaterial>::default(),
            ShaderPlugin::<ScrollBarMaterial>::default(),
        ));
        app.add_plugins((
            AssetAnimationPlugin::<ShaderAsset<BlockMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<HollowBlockMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<SunkenBlockMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<HightlightButtonMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<ListItemMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<ButtonMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<CheckboxMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<SliderMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<SliderHightlightBarMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<SliderHandlerMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<InputboxMaterial>>::default(),
            AssetAnimationPlugin::<ShaderAsset<ScrollBarMaterial>>::default(),
        ));
        let mut flat_theme = self.theme.clone();
        flat_theme.init(&mut app.world);
        let mut theme = app.world.resource_mut::<Theme>();
        theme.set_theme_dispatch(Some(std::sync::Arc::new(flat_theme)));
    }
}
