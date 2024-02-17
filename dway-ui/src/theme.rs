use std::{any::{type_name, Any, TypeId}, marker::PhantomData};

use bevy::{ecs::system::SystemId, utils::{HashMap, HashSet}};
use bevy_svg::prelude::Svg;

use crate::prelude::*;

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

#[derive(Resource, Reflect, Default, Debug)]
pub struct Theme {
    pub default_font: Handle<Font>,
    pub color_map: HashMap<String, Color>,
    pub style_map: HashMap<String, Style>,
    pub icons: HashMap<String, Handle<Svg>>,
    #[reflect(ignore)]
    pub callbacks: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
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
}

#[derive(Resource, Default)]
pub struct SystemMap {
    pub map: HashMap<(TypeId, String), SystemId>,
}

pub struct ThemePlugin;
impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        let asset_server = app.world.resource_mut::<AssetServer>();
        let icons = HashMap::from_iter(
            [
                "apps",
                "close",
                "dashboard",
                "lock",
                "logout",
                "maximize",
                "minimize",
                "power",
                "restart",
                "settings",
                "user",
                "volume_off",
                "volume_on",
            ]
            .iter()
            .map(|name| {
                (
                    name.to_string(),
                    asset_server.load(format!("embedded://dway_ui/icons/{name}.svg")),
                )
            }),
        );
        let theme = Theme {
            default_font: asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf"),
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
                ("panel".to_string(), Color::WHITE.with_a(0.5)),
                ("panel:hover".to_string(), color!("#ffffff")),
                ("panel:clicked".to_string(), color!("#D8DEE9")),
                ("panel-popup".to_string(), Color::WHITE.with_a(0.5)),
                ("panel-popup:hover".to_string(), color!("#ffffff")),
                ("panel-popup:clicked".to_string(), color!("#D8DEE9")),
                ("panel-foreground".to_string(), color!("#1b1d1e")),
                ("scroll-bar".to_string(), color!("#6791C9").with_a(0.5)),
                ("shadow".to_string(), color!("#888888").with_a(0.5)),
                (POPUP_BACKGROUND.to_string(), color!("#D8DEE9")),
            ]),
            icons,
            style_map: HashMap::from([("popup".to_string(), style!("m-4"))]),
            callbacks: default(),
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

pub trait ShaderTheme {
    type BlockMaterial: UiMaterial;
    type HollowBlockMaterial: UiMaterial;
    type SunkenBlockMaterial: UiMaterial;
    type HightlightButtonMaterial: UiMaterial;
    type ButtonMaterial: UiMaterial;
    type CheckboxMaterial: UiMaterial;
    type SliderMaterial: UiMaterial;
    type SliderHandlerMaterial: UiMaterial;
    type InputboxMaterial: UiMaterial;
    type ScrollBarMaterial: UiMaterial;
}
pub struct ShaderThemePlugin<T: ShaderTheme + Send + Sync + 'static>(PhantomData<T>);
impl<T: ShaderTheme + ShaderTheme + Send + Sync + 'static> Plugin for ShaderThemePlugin<T>

where
    T::BlockMaterial::Data: PartialEq + Eq + std::hash::Hash + Clone,
{
    fn build(&self, app: &mut App) {
        let mut types = HashSet::new();
        if !types.contains(&TypeId::of::<T::BlockMaterial>()){
            app.add_plugins(UiMaterialPlugin::<T::BlockMaterial>::default());
        }
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
