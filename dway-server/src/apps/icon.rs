use std::sync::Arc;

use bevy::utils::HashMap;
use bevy_svg::prelude::Svg;
use dway_util::try_or;
use icon_loader::ThemeNameProvider;

use crate::prelude::*;

#[derive(Clone, Debug, Reflect,FromReflect)]
pub enum IconResorce {
    Image(Handle<Image>),
    Svg(Handle<Svg>),
}

#[derive(Component, Debug, Reflect, Clone)]
pub struct Icon {
    pub id: String,
    pub loaded: bool,
    #[reflect(ignore)]
    pub icon: Option<Arc<icon_loader::Icon>>,
    pub cache: HashMap<u16, IconResorce>,
}

impl Icon {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            icon: Default::default(),
            cache: Default::default(),
            loaded: false,
        }
    }
}

#[derive(Resource, Debug)]
pub struct IconLoader {
    pub icon_loader: icon_loader::IconLoader,
}
impl Default for IconLoader {
    fn default() -> Self {
        let mut raw = icon_loader::IconLoader::default();
        // raw.set_theme_name_provider(ThemeNameProvider::GTK);
        try_or! { raw.update_theme_name(), "failed to update theme name", () };
        info!("icon theme name: {}", raw.theme_name());
        Self { icon_loader: raw }
    }
}

impl IconLoader {
    pub fn load(
        &mut self,
        icon: &mut Icon,
        size: u16,
        asset_server: &mut AssetServer,
    ) -> Option<IconResorce> {
        if !icon.loaded {
            let icon_id = icon.id.to_string() + ".svg";
            let raw_icon = self.icon_loader.load_icon(&icon_id);
            if raw_icon.is_none() {
                warn!("icon not found: {}", &icon.id);
            }
            icon.icon = raw_icon;
        }
        if let Some(image) = icon.cache.get(&size) {
            return Some(match image {
                IconResorce::Image(h) => IconResorce::Image(asset_server.get_handle::<Image, _>(h)),
                IconResorce::Svg(h) => IconResorce::Svg(asset_server.get_handle::<Svg, _>(h)),
            });
        }
        if let Some(raw_icon) = &icon.icon {
            let file = raw_icon.file_for_size(size);
            debug!(icon=%&icon.id,"loading icon file: {:?}",file.path());
            if file.path().extension().is_some_and(|e| e == "svg") {
                let image = asset_server.load(file.path());
                icon.cache
                    .insert(size, IconResorce::Svg(image.clone_weak()));
                return Some(IconResorce::Svg(image.clone()));
            } else {
                let image = asset_server.load(file.path());
                icon.cache
                    .insert(size, IconResorce::Image(image.clone_weak()));
                return Some(IconResorce::Image(image.clone()));
            }
        };
        None
    }
}
