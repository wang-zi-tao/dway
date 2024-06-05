pub mod flat;

use std::{
    any::{type_name, Any, TypeId},
    hash::Hash,
    sync::Arc,
    fmt::Debug,
};

use bevy::{
    app::DynEq,
    ecs::system::Command,
    ui::UiSystem,
    utils::{label::DynHash, HashMap},
};
use bevy_svg::prelude::Svg;
use bitflags::bitflags;
use derive_more::From;
use downcast_rs::{impl_downcast, Downcast};

use crate::{
    animation::{apply_tween_asset, AnimationEaseMethod},
    prelude::*,
    shader::{
        effect::{InnerShadow, Shadow},
        fill::Fill,
        Material, ShaderAsset,
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

#[derive(Debug, Clone)]
pub enum ThemeIcon {
    Path(Arc<str>),
    InDirectory(Arc<str>),
    Handle(Handle<Svg>),
}
impl ThemeIcon {
    pub fn get_svg(&self, name: &str, asset_server: &AssetServer) -> Handle<Svg> {
        match self {
            ThemeIcon::Path(p) => asset_server.load(p.to_string()),
            ThemeIcon::InDirectory(p) => asset_server.load(format!("{}/{}.svg", p, name)),
            ThemeIcon::Handle(h) => h.clone(),
        }
    }
}

#[derive(Resource, Reflect)]
pub struct Theme {
    pub default_font: Handle<Font>,
    pub default_text_size: f32,
    pub default_text_color: Color,
    pub color_map: HashMap<String, Color>,
    pub style_map: HashMap<String, Style>,
    #[reflect(ignore)]
    pub callbacks: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    #[reflect(ignore)]
    pub icons: HashMap<Box<str>, ThemeIcon>,

    #[reflect(ignore)]
    pub material_shadow: Shadow,
    #[reflect(ignore)]
    pub material_inner_shadow: (Color, Vec2, f32),
    #[reflect(ignore)]
    pub theme_dispatch: Option<Arc<dyn ThemeDispatch>>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            default_font: Default::default(),
            default_text_size: 9.0,
            default_text_color: Color::BLACK,
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
            theme_dispatch: None,
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
    pub fn icon(&self, name: &str, asset_server: &AssetServer) -> Handle<Svg> {
        if let Some(icon) = self
            .icons
            .get(name)
            .map(|icon| icon.get_svg(name, asset_server))
        {
            icon
        } else {
            warn!(icon_name = %name,"icon not found in theme");
            Default::default()
        }
    }
    pub fn system<F, I, M>(&self, system: F) -> SystemId<I, ()>
    where
        F: IntoSystem<I, (), M> + 'static,
        I: 'static,
    {
        let Some(callback) = self.callbacks.get(&system.type_id()) else {
            panic!(
                "system is not registered: {system}
note: add code
```
use dway_ui::theme::ThemeAppExt;
app.register_system({system});
``` to the plugin to register the system",
                system = type_name::<F>()
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

    pub fn default_text_style(&self) -> TextStyle {
        TextStyle {
            font: self.default_font(),
            font_size: self.default_text_size,
            color: self.default_text_color,
        }
    }

    pub fn text_style(&self, size: f32, class_name: &str) -> TextStyle {
        TextStyle {
            font: self.default_font(),
            font_size: size,
            color: self.color(class_name),
        }
    }

    pub fn set_theme_dispatch(&mut self, theme_dispatch: Option<Arc<dyn ThemeDispatch>>) {
        self.theme_dispatch = theme_dispatch;
    }

    pub fn get_component(&self, flag: StyleFlags, widget_kind: WidgetKind) -> ThemeComponent {
        ThemeComponent {
            theme: self.theme_dispatch.clone(),
            style_flags: flag,
            old_style_flags: flag,
            widget_kind,
        }
    }

    pub fn register_icons_in_dictory(&mut self, dir: &str, icons: &[&str]) {
        let icon_info = ThemeIcon::InDirectory(Arc::from(dir));
        for icon in icons {
            self.icons.insert(Box::from(*icon), icon_info.clone());
        }
    }
}

#[derive(Resource, Default)]
pub struct SystemMap {
    pub map: HashMap<(TypeId, String), SystemId>,
}

pub fn apply_theme_system(
    mut query: Query<(Entity, &mut ThemeComponent), Changed<ThemeComponent>>,
    mut commands: Commands,
    theme: Res<Theme>,
) {
    for (entity, mut theme_component) in &mut query {
        if let Some(theme_dispatch) = theme_component
            .theme
            .as_ref()
            .or_else(|| theme.theme_dispatch.as_ref())
        {
            theme_dispatch.apply(entity, &theme_component, &mut commands);
        }
        theme_component.old_style_flags = theme_component.style_flags;
    }
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
                ("inputbox:text".to_string(), color!("#10171e")),
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
            .add_systems(
                PostUpdate,
                apply_theme_system.in_set(UiFrameworkSystems::UpdateTheme),
            )
            .register_type::<Theme>();
    }
}

pub trait ThemeAppExt {
    fn register_system<F, I, M>(&mut self, system: F) -> &mut App
    where
        F: IntoSystem<I, (), M> + 'static,
        I: 'static;
}
impl ThemeAppExt for App {
    fn register_system<F, I, M>(&mut self, system: F) -> &mut App
    where
        F: IntoSystem<I, (), M> + 'static,
        I: 'static,
    {
        let type_id = system.type_id();
        let system_id = self.world.register_system(system);
        let mut theme = self.world.resource_mut::<Theme>();
        theme.callbacks.insert(type_id, Box::new(system_id));
        self
    }
}

pub trait WidgetLabel: Sync + Send + DynHash + DynEq + std::fmt::Debug {}

structstruck::strike! {
    #[derive(Default, From)]
    #[strikethrough[derive(Debug, Clone)]]
    pub enum WidgetKind {
        #[default]
        None,
        Block,
        Button,
        Checkbox,
        Inputbox,
        #[from]
        ComboBox(
        pub enum ComboBoxNodeKind{
            Root,
            Handle,
            Popup,
            Item,
        }),
        Slider,
        SliderHandle,
        SliderHightlightBar,
        BlurBackground,
        Other(Arc<dyn WidgetLabel>),
    }
}

bitflags! {
    #[derive(Clone,Copy, Debug,Hash,PartialEq, Eq, PartialOrd, Ord, Default)]
    pub struct StyleFlags: u64 {
        const CLICKED = 1;
        const HOVERED = 1<<1;
        const DOWNED = 1<<2;
        const FOCUSED = 1<<3;
        const DISABLE = 1<<4;
        const HIGHLIGHT = 1<<5;

        const SUNKEN = 1<<8;
        const HOLLOW = 1<<9;
    }
}

#[derive(Component, Debug, Clone)]
pub struct ThemeComponent {
    pub theme: Option<Arc<dyn ThemeDispatch>>,
    pub style_flags: StyleFlags,
    pub old_style_flags: StyleFlags,
    pub widget_kind: WidgetKind,
}

impl ThemeComponent {
    pub fn new(style_flags: StyleFlags, widget_kind: WidgetKind) -> Self {
        Self {
            theme: None,
            style_flags,
            old_style_flags: style_flags,
            widget_kind,
        }
    }
    pub fn widget(kind: WidgetKind) -> Self {
        Self::new(StyleFlags::default(), kind)
    }
    pub fn none() -> Self {
        Self::new(StyleFlags::empty(), WidgetKind::None)
    }
    pub fn set_flag(&mut self, flag: StyleFlags, value: bool) {
        self.style_flags.set(flag, value);
    }
    pub fn with_flag(mut self, flag: StyleFlags) -> Self {
        self.style_flags = self.style_flags.union(flag);
        self
    }
    pub fn with_flag_value(mut self, flag: StyleFlags, value: bool) -> Self {
        self.style_flags.set(flag, value);
        self
    }
}

pub trait ThemeDispatch: Downcast + Debug + Sync + Send + 'static {
    fn apply(&self, entity: Entity, theme: &ThemeComponent, commands: &mut Commands);
}
impl_downcast!(ThemeDispatch);

pub fn insert_material_tween<M: Asset + Interpolation>(
    world: &mut World,
    entity: Entity,
    end_material: Handle<M>,
    duration: Duration,
    ease: AnimationEaseMethod,
) {
    let current_material = world.get::<Handle<M>>(entity).cloned();
    let mut entity_mut = world.entity_mut(entity);
    if let Some(current_material) = current_material {
        entity_mut.insert(Tween::new(current_material.clone(), end_material));
        if let Some(mut animation) = entity_mut.get_mut::<Animation>() {
            animation.set_duration(duration);
            animation.set_ease_method(ease);
            animation.replay();
        } else {
            let mut animation = Animation::new(duration, ease);
            {
                let theme = world.resource::<Theme>();
                animation
                    .callbacks
                    .push(theme.system(apply_tween_asset::<M>));
            }
            animation.replay();
            world.entity_mut(entity).insert(animation);
        }
    } else {
        entity_mut.insert(end_material);
    }
}

pub struct AnimationConfig {
    pub duration: Duration,
    pub ease: AnimationEaseMethod,
}

pub fn insert_material_command<M: Material>(entity: Entity, material: M) -> impl Command {
    move |world: &mut World| {
        let mut assets = world.resource_mut::<Assets<ShaderAsset<M>>>();
        let handle = assets.add(ShaderAsset::new(material));
        world.entity_mut(entity).insert(handle);
    }
}
