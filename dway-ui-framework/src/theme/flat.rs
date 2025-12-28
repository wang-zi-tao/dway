
use bevy::ecs::{
    query::QueryItem,
    system::{
        lifetimeless::{SRes, SResMut},
        SystemParamItem,
    },
};

use super::{
    adapter::{
        ApplyMaterialAnimation, EventObserver, FocusMaterialSet, GlobalThemePlugin,
        InteractionMaterialSet, MaterialApplyMethod, ThemeTrait, WidgetInsertObserver,
    },
    insert_material_tween, BlockStyle, DefaultTextTheme, ThemeComponent, ThemeDispatch,
    ThemeHightlight,
};
use crate::{
    animation::{
        ease::AnimationEaseMethod, MaterialAnimationQueryData,
    },
    prelude::*,
    render::layer_manager::{FillWithLayer, LayerCamera},
    shader::{
        effect::{Border, InnerShadow, Shadow},
        fill::{AddColor, Fill, FillColor},
        shape::{Circle, RoundedBar, RoundedRect, Shape},
        transform::Margins,
        ShaderAsset, ShaderPlugin, ShapeRender, Transformed,
    },
    widgets::{
        button::UiButtonEventDispatcher,
        checkbox::UiCheckBoxEventDispatcher,
        inputbox::{UiInputBox, UiInputBoxEventDispatcher, UiInputBoxWidget},
        slider::{UiSliderEventDispatcher, UiSliderInited, UiSliderWidget},
    },
    UiFrameworkSystems,
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
#[require(ThemeComponent)]
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
    #[default(0.25)]
    pub blur_opacity: f32,
    #[default(Duration::from_secs_f32(0.2))]
    pub animation_duration: Duration,
    // #[default(AnimationEaseMethod::EaseFunction(EaseFunction::QuadraticIn))]
    #[default(AnimationEaseMethod::Linear)]
    pub animation_ease: AnimationEaseMethod,

    #[default(ApplyMaterialAnimation{duration:Duration::from_secs_f32(0.1),ease:AnimationEaseMethod::Linear})]
    pub animation_player: ApplyMaterialAnimation,
    pub button_material_set: InteractionMaterialSet<ShaderAsset<ButtonMaterial>>,
    pub hightlight_button_material_set:
        InteractionMaterialSet<ShaderAsset<HightlightButtonMaterial>>,
    pub checkbox_material_set: InteractionMaterialSet<ShaderAsset<CheckboxMaterial>>,
    pub checkbox_material_down_set: InteractionMaterialSet<ShaderAsset<CheckboxMaterial>>,
    pub slider_handler_material_set: InteractionMaterialSet<ShaderAsset<SliderHandlerMaterial>>,
    pub scroll_bar_material_set: InteractionMaterialSet<ShaderAsset<ScrollBarMaterial>>,
    pub inputbox_material_set: FocusMaterialSet<ShaderAsset<InputboxMaterial>>,
    pub list_item_hightlight_set: InteractionMaterialSet<ShaderAsset<ListItemMaterial>>,

    pub block_material: Handle<ShaderAsset<BlockMaterial>>,
    pub popup_block_material: Handle<ShaderAsset<BlockMaterial>>,
    pub hollow_block_material: Handle<ShaderAsset<HollowBlockMaterial>>,
    pub hightlight_hollow_block_material: Handle<ShaderAsset<HollowBlockMaterial>>,
    pub sunken_block_material: Handle<ShaderAsset<SunkenBlockMaterial>>,
    pub hightlight_block_material: Handle<ShaderAsset<BlockMaterial>>,
    pub slider_material: Handle<ShaderAsset<SliderMaterial>>,
    pub slider_hightlight_bar_material: Handle<ShaderAsset<SliderHightlightBarMaterial>>,
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
            self.hightlight_block_material = world.resource_mut::<Assets<_>>().add(
                ShaderAsset::new(self.block_rounded_rect().with_effect((
                    FillColor::new(self.main_color),
                    Shadow::new(
                        self.shadow_color,
                        self.shadow_offset,
                        self.shadow_margin,
                        self.shadow_radius * 2.0,
                    ),
                ))),
            );
        }

        self.button_material_set = InteractionMaterialSet::new(
            &mut world.resource_mut::<Assets<_>>(),
            ShaderAsset::new(self.rounded_rect().with_effect((
                self.invisible_inner_shadow(self.fill_color()),
                self.fill_color(),
                self.shadow(),
            ))),
            Some(ShaderAsset::new(self.rounded_rect().with_effect((
                self.invisible_inner_shadow(FillColor::new(
                    (self.fill_color.to_srgba() * 0.95).into(),
                )),
                FillColor::new((self.fill_color.to_srgba() * 0.95).into()),
                self.shadow(),
            )))),
            ShaderAsset::new(self.rounded_rect().with_effect((
                self.inner_shadow(FillColor::new((self.fill_color.to_srgba() * 0.95).into())),
                FillColor::new((self.fill_color.to_srgba() * 0.95).into()),
                self.invisible_shadow(),
            ))),
        );

        self.hightlight_button_material_set = InteractionMaterialSet::new(
            &mut world.resource_mut::<Assets<_>>(),
            ShaderAsset::new(self.rounded_rect().with_effect((
                self.border(),
                self.main_color.into(),
                self.shadow(),
            ))),
            Some(ShaderAsset::new(self.rounded_rect().with_effect((
                self.border(),
                self.main_color.into(),
                self.shadow(),
            )))),
            ShaderAsset::new(self.rounded_rect().with_effect((
                self.border(),
                self.main_color.into(),
                self.invisible_shadow(),
            ))),
        );

        self.list_item_hightlight_set = InteractionMaterialSet::new(
            &mut world.resource_mut(),
            ShaderAsset::new(self.rounded_rect().with_effect(self.main_color.into())),
            Some(ShaderAsset::new(
                self.rounded_rect().with_effect(self.fill_color3.into()),
            )),
            ShaderAsset::new(self.rounded_rect().with_effect(self.fill_color2.into())),
        );

        {
            let up_inner = Circle::new()
                .with_effect(self.fill_color())
                .with_transform(Margins::new(1.0, 32.0, 1.0, 1.0));
            let down_inner = Circle::new()
                .with_effect(self.fill_color())
                .with_transform(Margins::new(32.0, 1.0, 1.0, 1.0));
            self.checkbox_material_set = InteractionMaterialSet::new(
                &mut world.resource_mut::<Assets<_>>(),
                ShaderAsset::new((
                    up_inner.clone(),
                    RoundedBar::new().with_effect((
                        FillColor::new(self.fill_color2),
                        self.invisible_shadow(),
                    )),
                )),
                None,
                ShaderAsset::new((
                    up_inner,
                    RoundedBar::new().with_effect((
                        FillColor::new(self.fill_color2),
                        self.highlight_shadow(),
                    )),
                )),
            );
            self.checkbox_material_down_set = InteractionMaterialSet::new(
                &mut world.resource_mut::<Assets<_>>(),
                ShaderAsset::new((
                    down_inner.clone(),
                    RoundedBar::new().with_effect((self.main_color.into(), self.shadow())),
                )),
                None,
                ShaderAsset::new((
                    down_inner,
                    RoundedBar::new().with_effect((
                        FillColor::new(self.main_color),
                        self.highlight_shadow(),
                    )),
                )),
            );
        }

        self.slider_handler_material_set = InteractionMaterialSet::new(
            &mut world.resource_mut(),
            ShaderAsset::new(Circle::new().with_effect((
                self.border(),
                self.fill_color(),
                self.shadow(),
            ))),
            None,
            ShaderAsset::new(Circle::new().with_effect((
                self.border(),
                self.fill_color(),
                self.highlight_shadow(),
            ))),
        );
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

        self.inputbox_material_set = FocusMaterialSet::new(
            &mut world.resource_mut(),
            ShaderAsset::new(
                RoundedRect::new(0.5 * self.cornor)
                    .with_effect((self.inactive_border(), self.fill_color())),
            ),
            ShaderAsset::new(
                RoundedRect::new(0.5 * self.cornor).with_effect((self.border(), self.fill_color())),
            ),
        );
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
        _theme_entity: Entity,
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
        _theme_entity: Entity,
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
                BlockStyle::Blur => {
                    commands.insert((
                        FlatThemeComponent,
                        MaterialNode::<ShaderAsset<BlurMaterial>>::default(),
                    ));
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
        _theme_entity: Entity,
        (_event_dispatcher, hightlight): QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        if hightlight {
            commands.insert(self.hightlight_button_material_set.normal.clone());
            let widget_entity = commands.id();
            callback_register.add_to_observer(
                <Self as EventObserver<UiButtonEvent, ThemeHightlight>>::trigger,
                commands.commands_mut(),
                widget_entity,
            );
        } else {
            commands.insert(self.button_material_set.normal.clone());
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
        _theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        let interaction = match event.kind {
            UiButtonEventKind::Pressed => Interaction::Pressed,
            UiButtonEventKind::Released | UiButtonEventKind::Hovered => Interaction::Hovered,
            UiButtonEventKind::Leaved => Interaction::None,
        };
        self.animation_player.apply(
            self.button_material_set.get_material(interaction).clone(),
            commands,
            query_items,
            callback_register,
        );
    }
}

impl EventObserver<UiButtonEvent, ThemeHightlight> for FlatTheme {
    type ItemQuery = MaterialAnimationQueryData<ShaderAsset<HightlightButtonMaterial>>;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_event(
        &self,
        event: Trigger<UiEvent<UiButtonEvent>>,
        _theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        let interaction = match event.kind {
            UiButtonEventKind::Pressed => Interaction::Pressed,
            UiButtonEventKind::Released | UiButtonEventKind::Hovered => Interaction::Hovered,
            UiButtonEventKind::Leaved => Interaction::None,
        };
        self.animation_player.apply(
            self.hightlight_button_material_set
                .get_material(interaction)
                .clone(),
            commands,
            query_items,
            callback_register,
        );
    }
}

impl WidgetInsertObserver<UiCheckBox> for FlatTheme {
    type Filter = ();
    type ItemQuery = &'static mut UiCheckBoxEventDispatcher;
    type Params = SResMut<CallbackTypeRegister>;

    fn on_widget_insert(
        &self,
        _theme_entity: Entity,
        _: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        commands.insert(self.checkbox_material_set.normal.clone());
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
        _theme_entity: Entity,
        (checkbox, query_items): QueryItem<Self::ItemQuery>,
        callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        let material_set = if event.value {
            &self.checkbox_material_down_set
        } else {
            &self.checkbox_material_set
        };
        self.animation_player.apply(
            material_set.get_material(checkbox.state).clone(),
            commands,
            query_items,
            callback_register,
        );
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
        (_prop, widget, _event_dispatcher): QueryItem<Self::ItemQuery>,
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
        handle_entity_commands.insert(self.slider_handler_material_set.normal.clone());
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
        _theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        if let Some(interaction) = event.interaction() {
            self.animation_player.apply(
                self.slider_handler_material_set
                    .get_material(interaction)
                    .clone(),
                commands,
                query_items,
                callback_register,
            );
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
        _theme_entity: Entity,
        (_prop, _widget, _event_dispatcher): QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        commands.insert(self.inputbox_material_set.normal.clone());
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
        _theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        callback_register: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    ) {
        if let Some(focus) = event.key_focus() {
            self.animation_player.apply(
                self.inputbox_material_set.get_material(focus).clone(),
                commands,
                query_items,
                callback_register,
            );
        }
    }
}

impl WidgetInsertObserver<UiPopup> for FlatTheme {
    type Filter = ();
    type ItemQuery = ();
    type Params = ();

    fn on_widget_insert(
        &self,
        _theme_entity: Entity,
        _: QueryItem<Self::ItemQuery>,
        _: SystemParamItem<Self::Params>,
        mut commands: EntityCommands,
    ) {
        commands.insert((
            FlatThemeComponent,
            MaterialNode::<ShaderAsset<BlurMaterial>>::default(),
        ));
    }
}

pub fn update_ui_blur_material(
    mut query: Query<
        (
            &ComputedUiTargetCamera,
            &ThemeComponent,
            &mut MaterialNode<ShaderAsset<BlurMaterial>>,
        ),
        (
            With<FlatThemeComponent>,
            Or<(
                Changed<ComputedUiTargetCamera>,
                Added<MaterialNode<ShaderAsset<BlurMaterial>>>,
            )>,
        ),
    >,
    ui_root_query: Query<&LayerCamera>,
    theme_query: Query<&FlatTheme>,
    mut material_assets: ResMut<Assets<ShaderAsset<BlurMaterial>>>,
) {
    for (node_target, theme_component, mut shader_handle) in &mut query {
        let Ok(theme) = theme_query.get(theme_component.theme_entity) else {
            continue;
        };

        let Some(layer) = node_target.get().and_then(|e| ui_root_query.get(e).ok()) else {
            continue;
        };

        let material = RoundedRect::new(theme.block_cornor).with_effect(AddColor::new(
            FillWithLayer {
                texture: layer.ui_background().clone(),
                texture_size: layer.background_size,
            },
            theme.fill_color.with_alpha(theme.blur_opacity),
        ));
        *shader_handle = material_assets.add(material).into();
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
        <FlatTheme as WidgetInsertObserver<UiPopup>>::register(theme_entity, world);
    }

    fn unregister(&self, _theme_entity: Entity, _world: &mut World) {
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

        app.add_systems(
            Last,
            update_ui_blur_material.in_set(UiFrameworkSystems::UpdateLayersMaterial),
        );
    }
}
