use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
};

use bevy::{
    ecs::system::{Command, EntityCommand, SystemId},
    render::render_resource::AsBindGroup,
    utils::{HashMap, HashSet},
};
use bevy_svg::prelude::Svg;

use crate::{
    animation::AnimationEaseMethod,
    prelude::*,
    shader::{
        effect::{InnerShadow, Shadow},
        fill::Fill,
        Material, ShaderAsset, ShaderPlugin,
    },
};

pub mod classname {
    pub const BACKGROUND: &str = "background";
    pub const BACKGROUND1: &str = "background1";
    pub const BACKGROUND2: &str = "background2";

    pub const FOREGROUND: &str = "foreground";
    pub const FOREGROUND1: &str = "foreground1";
    pub const FOREGROUND2: &str = "foreground2";

    pub const POPUP_BACKGROUND: &str = "popup-background";
}

pub mod iconname {
    pub const CLOSE: &str = "close";
    pub const MAXIMIZE: &str = "maximize";
    pub const MINIMIZE: &str = "minimize";
}
use classname::*;

#[derive(Resource, Reflect, Debug)]
pub struct Theme {
    pub default_font: Handle<Font>,
    pub color_map: HashMap<String, Color>,
    pub style_map: HashMap<String, Style>,
    pub icons: HashMap<String, Handle<Svg>>,
    #[reflect(ignore)]
    pub callbacks: HashMap<TypeId, Box<dyn Any + Send + Sync>>,

    #[reflect(ignore)]
    pub material_shadow: Shadow,
    #[reflect(ignore)]
    pub material_inner_shadow: (Color, Vec2, f32),
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            default_font: Default::default(),
            color_map: Default::default(),
            style_map: Default::default(),
            icons: Default::default(),
            callbacks: Default::default(),
            material_shadow: Shadow::new(
                color!("#888888"),
                Vec2::new(0.0, 1.0),
                Vec2::splat(1.0),
                2.0,
            ),
            material_inner_shadow: (color!("#888888"), Vec2::new(1.0, 1.0), 1.0),
        }
    }
}

impl Theme {
    pub fn default_font(&self) -> Handle<Font> {
        self.default_font.clone()
    }
    pub fn color(&self, color: &str) -> Color {
        self.color_map.get(color).cloned().unwrap_or(Color::NONE)
    }
    pub fn icon(&self, icon: &str) -> Handle<Svg> {
        self.icons.get(icon).cloned().unwrap_or_default()
    }
    pub fn system<T, M>(&self, system: T) -> SystemId<T::In, T::Out>
    where
        T: SystemParamFunction<M, Out = ()>,
        T::In: 'static,
        T::Out: 'static,
    {
        let Some(callback) = self.callbacks.get(&system.type_id()) else {
            panic!(
                "system is not registered: {system}
note: add code
```
use dway_ui::theme::ThemeAppExt;
app.register_system({system});
``` to the plugin to register the system",
                system = type_name::<T>()
            );
        };
        *callback.as_ref().downcast_ref().unwrap()
    }

    pub fn default_shadow_material(&self) -> Shadow {
        self.material_shadow.clone()
    }

    pub fn default_inner_shadow_material<F: Fill>(&self, filler: F) -> InnerShadow<F> {
        InnerShadow::new(
            filler,
            self.material_inner_shadow.0,
            self.material_inner_shadow.1,
            self.material_inner_shadow.2,
        )
    }

    pub fn text_style(&self, size: f32, class_name: &str) -> TextStyle {
        TextStyle {
            font: self.default_font(),
            font_size: size,
            color: self.color(class_name),
        }
    }
}

#[derive(Resource, Default)]
pub struct SystemMap {
    pub map: HashMap<(TypeId, String), SystemId>,
}

pub struct ThemePlugin;
impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        let asset_server = app.world.resource_mut::<AssetServer>();
        let theme = Theme {
            default_font: asset_server
                .load("embedded://dway_ui_framework/fonts/SmileySans-Oblique.ttf"),
            color_map: HashMap::from([
                ("foreground".to_string(), color!("#10171e")),
                ("foreground1".to_string(), color!("#1b1d1e")),
                ("foreground2".to_string(), color!("#2b2d2e")),
                ("background".to_string(), color!("#ffffff")),
                ("background1".to_string(), color!("#D8DEE9")),
                ("background2".to_string(), color!("#C8CED9")),
                ("black".to_string(), color!("#1C252C")),
                ("red".to_string(), color!("#DF5B61")),
                ("green".to_string(), color!("#78B892")),
                ("yellow".to_string(), color!("#e7c787")),
                ("orange".to_string(), color!("#DE8F78")),
                ("blue".to_string(), color!("#6791C9")),
                ("purple".to_string(), color!("#BC83E3")),
                ("magenta".to_string(), color!("#c678dd")),
                ("cyan".to_string(), color!("#008080")),
                ("sky".to_string(), color!("#67AFC1")),
                ("white".to_string(), color!("#D9D7D6")),
                ("gray".to_string(), color!("#484E5B")),
                ("slider:bar".to_string(), color!("#C8CED9")),
                ("slider:bar:highlight".to_string(), color!("#DE8F78")),
                ("slider:handle".to_string(), color!("#6791C9")),
                ("checkbox:bar".to_string(), color!("#6791C9")),
                ("checkbox:bar:highlight".to_string(), color!("#DE8F78")),
                ("checkbox:handle".to_string(), color!("#6791C9")),
                ("inputbox:cursor".to_string(), color!("#6791C9")),
                ("inputbox:placeholder".to_string(), color!("#C8CED9")),
                ("inputbox:text".to_string(), color!("#ffffff")),
                ("panel".to_string(), Color::WHITE.with_a(0.5)),
                ("panel:hover".to_string(), color!("#ffffff")),
                ("panel:clicked".to_string(), color!("#D8DEE9")),
                ("panel-popup".to_string(), Color::WHITE.with_a(0.5)),
                ("panel-popup:hover".to_string(), color!("#ffffff")),
                ("panel-popup:clicked".to_string(), color!("#D8DEE9")),
                ("panel-foreground".to_string(), color!("#1b1d1e")),
                ("scroll-bar".to_string(), color!("#6791C9").with_a(0.8)),
                ("shadow".to_string(), color!("#888888").with_a(0.5)),
                (POPUP_BACKGROUND.to_string(), color!("#D8DEE9")),
            ]),
            style_map: HashMap::from([("popup".to_string(), style!("m-4"))]),
            ..Default::default()
        };
        let systems = SystemMap {
            map: HashMap::from([]),
        };
        app.insert_resource(theme)
            .insert_resource(systems)
            .register_type::<Theme>();
    }
    fn finish(&self, app: &mut App) {
        let theme = app.world.resource::<Theme>();
        debug!("theme: {:?}", theme);
    }
}

pub trait ThemeAppExt {
    fn register_system<F, M: 'static>(&mut self, system: F) -> &mut App
    where
        F: SystemParamFunction<M, Out = ()> + 'static,
        F::In: 'static,
        F::Out: 'static;
}
impl ThemeAppExt for App {
    fn register_system<F, M: 'static>(&mut self, system: F) -> &mut App
    where
        F: SystemParamFunction<M, Out = ()> + 'static,
        F::In: 'static,
        F::Out: 'static,
    {
        let type_id = system.type_id();
        let system_id = self.world.register_system(system);
        let mut theme = self.world.resource_mut::<Theme>();
        theme.callbacks.insert(type_id, Box::new(system_id));
        self
    }
}

pub enum BlockVariant{ Normal, Hover }
pub enum ButtonVariant{ Normal, Hover, Clicked }
pub enum CheckboxVariant{ Up, UpHover, Down, DownHover }
pub enum InputboxVariant{ Normal, Hover, Focused }

pub enum ShaderThemeAnimationInfo {
    Enable{
        duration: Duration,
    },
    Disable{},
    Default,
}

pub trait ShaderTheme {
    type BlockMaterial: Material;
    type HollowBlockMaterial: Material;
    type SunkenBlockMaterial: Material;
    type HightlightButtonMaterial: Material;
    type ButtonMaterial: Material;
    type CheckboxMaterial: Material;
    type SliderMaterial: Material;
    type SliderHandlerMaterial: Material;
    type InputboxMaterial: Material;
    type ScrollBarMaterial: Material;

    fn block_material(&self, _variant: BlockVariant) -> Option<ShaderAsset<Self::BlockMaterial>> {None}
    fn hollow_block_material(&self, _variant: BlockVariant) -> Option<ShaderAsset<Self::HollowBlockMaterial>> {None}
    fn sunken_block_material(&self, _variant: BlockVariant) -> Option<ShaderAsset<Self::SunkenBlockMaterial>> {None}
    fn hightlight_button_material(&self, _variant: ButtonVariant) -> Option<ShaderAsset<Self::HightlightButtonMaterial>> {None}
    fn button_material(&self, _variant: ButtonVariant) -> Option<ShaderAsset<Self::ButtonMaterial>> {None}
    fn checkbox_material(&self,_variant: ButtonVariant) -> Option<ShaderAsset<Self::CheckboxMaterial>> {None}
    fn slider_material(&self) -> Option<ShaderAsset<Self::SliderMaterial>> {None}
    fn slider_handler_material(&self, _variant: ButtonVariant) -> Option<ShaderAsset<Self::SliderHandlerMaterial>> {None}
    fn inputbox_material(&self, _variant: InputboxVariant) -> Option<ShaderAsset<Self::InputboxMaterial>> {None}
    fn scroll_bar_material(&self, _variant: ButtonVariant) -> Option<ShaderAsset<Self::ScrollBarMaterial>> {None}

    fn block_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn hollow_block_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn sunken_block_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn hightlight_button_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn button_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn checkbox_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn slider_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn slider_handler_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn inputbox_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
    fn scroll_bar_material_animation(&self) -> ShaderThemeAnimationInfo { ShaderThemeAnimationInfo::Default }
}

pub struct ShaderThemePlugin<T: ShaderTheme + Send + Sync + 'static>(PhantomData<T>);
impl<T: ShaderTheme + ShaderTheme + Send + Sync + 'static> Plugin for ShaderThemePlugin<T> {
    fn build(&self, app: &mut App) {
        let mut types = HashSet::new();
        if types.insert(TypeId::of::<T::BlockMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::BlockMaterial>::default());
        }
        if types.insert(TypeId::of::<T::HollowBlockMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::HollowBlockMaterial>::default());
        }
        if types.insert(TypeId::of::<T::SunkenBlockMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::SunkenBlockMaterial>::default());
        }
        if types.insert(TypeId::of::<T::HightlightButtonMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::HightlightButtonMaterial>::default());
        }
        if types.insert(TypeId::of::<T::ButtonMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::ButtonMaterial>::default());
        }
        if types.insert(TypeId::of::<T::CheckboxMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::CheckboxMaterial>::default());
        }
        if types.insert(TypeId::of::<T::SliderMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::SliderMaterial>::default());
        }
        if types.insert(TypeId::of::<T::SliderHandlerMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::SliderHandlerMaterial>::default());
        }
        if types.insert(TypeId::of::<T::InputboxMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::InputboxMaterial>::default());
        }
        if types.insert(TypeId::of::<T::ScrollBarMaterial>()) {
            app.add_plugins(ShaderPlugin::<T::ScrollBarMaterial>::default());
        }
    }
}

fn insert_material_command<M: Material>(entity: Entity, material: M) -> impl Command {
    move |world: &mut World| {
        let mut assets = world.resource_mut::<Assets<ShaderAsset<M>>>();
        let handle = assets.add(ShaderAsset::new(material));
        world.entity_mut(entity).insert(handle);
    }
}

fn insert_material_tween_command<M: Material + Interpolation>(
    entity: Entity,
    duration: Duration,
    ease: AnimationEaseMethod,
    begin_material: M,
    end_material: M,
    theme: &Theme,
) -> impl Command {
    let asset = ShaderAsset::new(begin_material.clone());
    let mut animation = Animation::new(duration, ease);
    animation.pause();
    let animation_bundle = AssetTweenAddonBundle::new(
        animation,
        Tween::new(
            ShaderAsset::new(begin_material),
            ShaderAsset::new(end_material),
        ),
        theme,
    );
    move |world: &mut World| {
        let mut assets = world.resource_mut::<Assets<ShaderAsset<M>>>();
        let handle = assets.add(asset);
        let mut e = world.entity_mut(entity);
        e.insert((handle, animation_bundle));
    }
}
