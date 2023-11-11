use crate::prelude::*;
use bevy::utils::HashMap;
use bevy_svg::prelude::Svg;
use dway_util::try_or;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Reflect, PartialEq, Eq)]
pub enum IconResorce {
    Image(Handle<Image>),
    Svg(Handle<Svg>),
}
impl Default for IconResorce{
    fn default() -> Self {
        Self::Image(default())
    }
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
        svg_assets: &mut Assets<Svg>,
        mesh_assets: &mut Assets<Mesh>,
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
                IconResorce::Image(h) => IconResorce::Image(asset_server.get_id_handle(h.id())?),
                IconResorce::Svg(h) => IconResorce::Svg(asset_server.get_id_handle(h.id())?),
            });
        }
        if let Some(raw_icon) = &icon.icon {
            let file = raw_icon.file_for_size(size);
            let path = file.path().to_owned();
            if path.extension().is_some_and(|e| e == "svg") {
                let data = std::fs::read(&path).ok()?;
                let mut svg = Svg::from_bytes(&data, file.path(), Option::<PathBuf>::None).ok()?;
                svg.mesh = mesh_assets.add(svg.tessellate());
                let image = svg_assets.add(svg);
                icon.cache
                    .insert(size, IconResorce::Svg(image.clone_weak()));
                debug!(icon=%icon.id,"loading svg icon file: {:?}",file.path());
                return Some(IconResorce::Svg(image.clone()));
            } else {
                let image = asset_server.load(path.clone());
                icon.cache
                    .insert(size, IconResorce::Image(image.clone_weak()));
                debug!(icon=%icon.id,"loading pixmap icon file: {:?}",path);
                return Some(IconResorce::Image(image.clone()));
            }
        };
        None
    }
}
