use std::any::type_name;

use bevy::ecs::{
    query::{QueryData, QueryItem},
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};

use super::{
    adapter::{EventObserver, GlobalThemePlugin, ThemeTrait, WidgetInsertObserver},
    insert_material_tween, BlockStyle, DefaultTextTheme, StyleFlags, ThemeComponent, ThemeDispatch,
    ThemeHightlight,
};
use crate::{
    animation::{
        apply_tween_asset, ease::AnimationEaseMethod, play_asset_animation,
        AnimationEventDispatcher, MaterialAnimationQueryData,
    },
    prelude::*,
    render::layer_manager::{FillWithLayer, RenderToLayer},
    shader::{
        effect::{Border, InnerShadow, Shadow},
        fill::{AddColor, Fill, FillColor},
        shape::{Circle, RoundedBar, RoundedRect, Shape},
        transform::Margins,
        ShaderAsset, ShaderPlugin, ShapeRender, Transformed,
    },
    util::{modify_component_or_insert, set_component_or_insert},
    widgets::{
        button::UiButtonEventDispatcher,
        checkbox::UiCheckBoxEventDispatcher,
        inputbox::{UiInputBox, UiInputBoxEventDispatcher, UiInputBoxWidget},
        slider::{UiSliderEventDispatcher, UiSliderInited, UiSliderWidget},
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
type BlurMaterial = ShapeRender<RoundedRect, AddColor<FillWithLayer>>;

#[derive(Component)]
pub struct FlatThemeComponent;

#[derive(SmartDefault, Clone, Debug, Component)]
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
    #[default(0.75)]
    pub blur_brightness: f32,
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
    pub slider_handler_material_hoverd: Handle<ShaderAsset<SliderHandlerMaterial>>,
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
                self.block_rounded_rect().with_effect((self.fill_color(), {
                    Shadow::new(
                        self.shadow_color,
                        self.shadow_offset,
                        self.shadow_margin,
                        self.shadow_radius * 2.0,
                    )
                })),
            ));
            self.popup_block_material = world.resource_mut::<Assets<_>>().add(ShaderAsset::new(
                self.popup_block_rounded_rect()
                    .with_effect((self.fill_color(), {
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
                    self.invisible_inner_shadow(FillColor::new(
                        (self.fill_color.to_srgba() * 0.95).into(),
                    )),
                    FillColor::new((self.fill_color.to_srgba() * 0.95).into()),
                    self.shadow(),
                ))));
            self.button_material_clicked =
                button_material_assets.add(ShaderAsset::new(self.rounded_rect().with_effect((
                    self.inner_shadow(FillColor::new((self.fill_color.to_srgba() * 0.95).into())),
                    FillColor::new((self.fill_color.to_srgba() * 0.95).into()),
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
                    FillColor::new(self.fill_color2.into()),
                    self.invisible_shadow(),
                )),
            )));
            self.checkbox_material_hover = checkbox_material_assets.add(ShaderAsset::new((
                Circle::new()
                    .with_effect(self.fill_color())
                    .with_transform(Margins::new(1.0, 32.0, 1.0, 1.0)),
                RoundedBar::new().with_effect((
                    FillColor::new(self.fill_color2.into()),
                    self.highlight_shadow(),
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
                RoundedBar::new().with_effect((
                    FillColor::new(self.main_color.into()),
                    self.highlight_shadow(),
                )),
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
        self.slider_handler_material_hoverd =
            world
                .resource_mut::<Assets<_>>()
                .add(ShaderAsset::new(Circle::new().with_effect((
                    self.border(),
                    self.fill_color(),
                    self.highlight_shadow(),
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

    fn highlight_shadow(&self) -> Shadow {
        Shadow::new(
            self.main_color,
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
            color: Color::NONE.to_linear(),
            offset: Vec2::ZERO,
            radius: 0.0,
        }
    }

    fn inner_shadow<F: Fill>(&self, filler: F) -> InnerShadow<F> {
        InnerShadow {
            filler,
            color: self.inner_shadow_color.to_linear(),
            offset: self.inner_shadow_offset,
            radius: self.inner_shadow_radius,
        }
    }

    fn apply_material_animation<M: UiMaterial + Asset + Interpolation>(
        &self,
        entity: Entity,
        commands: &mut Commands,
        material: Handle<M>,
    ) {
        let duration = self.animation_duration;
        let ease = self.animation_ease.clone();
        commands.queue(move |world: &mut World| {
            insert_material_tween(world, entity, material, duration, ease)
        });
    }
}

// TODO blur theme


impl WidgetInsertObserver<Text> for FlatTheme {
    type Filter = Without<DefaultTextTheme>;
    type ItemQuery = (&'static mut TextFont,);
    type Params = (SRes<Theme>,);

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        (mut text_font,): QueryItem<Self::ItemQuery>,
        (theme,): SystemParamItem<Self::Params>,
        _: EntityCommands,
    ) {
        if text_font.font == Handle::default() {
            text_font.font = theme.default_font();
        }
    }
}

impl WidgetInsertObserver<BlockStyle> for FlatTheme {
    type Filter = ();
    type ItemQuery = (&'static BlockStyle, Has<ThemeHightlight>);
    type Params = ();

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        (block_style, hightlight): QueryItem<Self::ItemQuery>,
        _: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        if hightlight {
            commands.insert(MaterialNode(self.hightlight_hollow_block_material.clone()));
        } else {
            match block_style {
                BlockStyle::Normal => {
                    commands.insert(MaterialNode(self.block_material.clone()));
                }
                BlockStyle::Hollow => {
                    commands.insert(MaterialNode(self.hollow_block_material.clone()));
                }
                BlockStyle::Sunken => {
                    commands.insert(MaterialNode(self.sunken_block_material.clone()));
                }
            }
        }
    }
}

impl WidgetInsertObserver<UiButton> for FlatTheme {
    type Filter = ();
    type ItemQuery = (&'static mut UiButtonEventDispatcher, Has<ThemeHightlight>);
    type Params = SResMut<CallbackTypeRegister>;

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        (mut event_dispatcher, hightlight): QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        if hightlight {
            commands.insert(MaterialNode(self.hightlight_button_material.clone()));
            let widget_entity = commands.id();
            callback_register.add_to_observer(
                <Self as EventObserver<UiButtonEvent, ThemeHightlight>>::trigger,
                commands.commands_mut(),
                widget_entity,
            );
        } else {
            commands.insert(MaterialNode(self.button_material.clone()));
            let widget_entity = commands.id();
            callback_register.add_to_observer(
                <Self as EventObserver<UiButtonEvent>>::trigger,
                commands.commands_mut(),
                widget_entity,
            );
        }
    }
}

impl EventObserver<UiButtonEvent> for FlatTheme {
    type ItemQuery = MaterialAnimationQueryData<ShaderAsset<ButtonMaterial>>;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_event(
        &self,
        event: Trigger<UiEvent<UiButtonEvent>>,
        theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        match event.kind {
            UiButtonEventKind::Pressed => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.button_material_clicked.clone(),
                    self.animation_duration.mul_f32(0.5),
                    self.animation_ease.clone(),
                    commands,
                );
            }
            UiButtonEventKind::Released | UiButtonEventKind::Hovered => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.button_material_hover.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            UiButtonEventKind::Leaved => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.button_material.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
        }
    }
}

impl EventObserver<UiButtonEvent, ThemeHightlight> for FlatTheme {
    type ItemQuery = MaterialAnimationQueryData<ShaderAsset<HightlightButtonMaterial>>;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_event(
        &self,
        event: Trigger<UiEvent<UiButtonEvent>>,
        theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        match event.kind {
            UiButtonEventKind::Pressed => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.hightlight_button_material_clicked.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            UiButtonEventKind::Released | UiButtonEventKind::Hovered => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.hightlight_button_material_hover.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            UiButtonEventKind::Leaved => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.hightlight_button_material.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
        }
    }
}

impl WidgetInsertObserver<UiCheckBox> for FlatTheme {
    type Filter = ();
    type ItemQuery = &'static mut UiCheckBoxEventDispatcher;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        mut event_dispatcher: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        commands.insert(MaterialNode(self.checkbox_material.clone()));
        let widget_entity = commands.id();
        callback_register.add_to_observer(
            <Self as EventObserver<UiCheckBoxEvent>>::trigger,
            commands.commands_mut(),
            widget_entity,
        );
    }
}

impl EventObserver<UiCheckBoxEvent> for FlatTheme {
    type ItemQuery = (
        &'static UiCheckBox,
        MaterialAnimationQueryData<ShaderAsset<CheckboxMaterial>>,
    );
    type Params = SResMut<CallbackTypeRegister>;

    fn on_event(
        &self,
        event: Trigger<UiEvent<UiCheckBoxEvent>>,
        theme_entity: Entity,
        (checkbox, query_items): QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        match (event.value, checkbox.state) {
            (true, Interaction::Hovered | Interaction::Pressed) => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.checkbox_material_down_hover.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            (false, Interaction::None) => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.checkbox_material.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            (false, Interaction::Hovered | Interaction::Pressed) => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.checkbox_material_hover.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            (true, Interaction::None) => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.checkbox_material_down.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
        }
    }
}

impl WidgetInsertObserver<UiSliderInited> for FlatTheme {
    type Filter = ();
    type ItemQuery = (
        &'static UiSlider,
        &'static UiSliderWidget,
        &'static mut UiSliderEventDispatcher,
    );
    type Params = SResMut<CallbackTypeRegister>;

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        (prop, widget, mut event_dispatcher): QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        commands
            .commands()
            .entity(widget.node_bar_entity)
            .insert(MaterialNode(self.slider_material.clone()));
        commands
            .commands()
            .entity(widget.node_bar_highlight_entity)
            .insert(MaterialNode(self.slider_hightlight_bar_material.clone()));

        let mut commands = commands.commands();
        let mut handle_entity_commands = commands.entity(widget.node_handle_entity);
        handle_entity_commands.insert((MaterialNode(self.slider_handler_material.clone()),));
        handle_entity_commands.entry::<UiInput>().or_default();
        handle_entity_commands
            .entry::<ThemeComponent>()
            .or_default()
            .and_modify(move |mut t| {
                t.theme_entity = theme_entity;
            });

        let handle_entity = widget.node_handle_entity;
        callback_register.add_to_observer(
            <Self as EventObserver<UiInputEvent, UiSliderHandle>>::trigger,
            &mut commands,
            handle_entity,
        );
    }
}

pub struct UiSliderHandle;

impl EventObserver<UiInputEvent, UiSliderHandle> for FlatTheme {
    type ItemQuery = MaterialAnimationQueryData<ShaderAsset<SliderHandlerMaterial>>;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_event(
        &self,
        event: Trigger<UiEvent<UiInputEvent>>,
        theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        match &**event {
            UiInputEvent::MouseEnter => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.slider_handler_material_hoverd.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            UiInputEvent::MouseLeave => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.slider_handler_material.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            _ => {}
        }
    }
}

impl WidgetInsertObserver<UiInputBox> for FlatTheme {
    type Filter = ();
    type ItemQuery = (
        &'static UiInputBox,
        &'static UiInputBoxWidget,
        &'static mut UiInputBoxEventDispatcher,
    );
    type Params = SResMut<CallbackTypeRegister>;

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        (prop, widget, mut event_dispatcher): QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        commands.insert(MaterialNode(self.inputbox_material.clone()));
        let widget_entity = commands.id();
        callback_register.add_to_observer(
            <Self as EventObserver<UiInputEvent, UiInputBox>>::trigger,
            commands.commands_mut(),
            widget_entity,
        );
    }
}

impl EventObserver<UiInputEvent, UiInputBox> for FlatTheme {
    type ItemQuery = MaterialAnimationQueryData<ShaderAsset<InputboxMaterial>>;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_event(
        &self,
        event: Trigger<UiEvent<UiInputEvent>>,
        theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        match &**event {
            UiInputEvent::KeyboardEnter => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.inputbox_material_focused.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            UiInputEvent::KeyboardLeave => {
                play_asset_animation(
                    query_items,
                    &mut callback_register,
                    self.inputbox_material.clone(),
                    self.animation_duration,
                    self.animation_ease.clone(),
                    commands,
                );
            }
            _ => {}
        }
    }
}

impl ThemeTrait for FlatTheme {
    fn register_to_global(theme_entity: Entity, world: &mut World) {
        <FlatTheme as WidgetInsertObserver<Text>>::register(theme_entity, world);
        <FlatTheme as WidgetInsertObserver<BlockStyle>>::register(theme_entity, world);
        <FlatTheme as WidgetInsertObserver<UiButton>>::register(theme_entity, world);
        <FlatTheme as WidgetInsertObserver<UiCheckBox>>::register(theme_entity, world);
        <FlatTheme as WidgetInsertObserver<UiSliderInited>>::register(theme_entity, world);
        <FlatTheme as WidgetInsertObserver<UiInputBox>>::register(theme_entity, world);
    }

    fn unregister(&self, theme_entity: Entity, world: &mut World) {
        todo!()
    }
}

#[derive(SmartDefault)]
pub struct FlatThemePlugin {
    theme: FlatTheme,
    #[default(true)]
    register_to_global: bool,
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
            ShaderPlugin::<BlurMaterial>::default(),
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
        if self.register_to_global {
            let mut flat_theme = self.theme.clone();
            flat_theme.init(app.world_mut());
            app.add_plugins(GlobalThemePlugin::new(flat_theme));
            // let mut theme = app.world_mut().resource_mut::<Theme>();
            // theme.set_theme_dispatch(Some(std::sync::Arc::new(flat_theme)));
        }
    }
}
