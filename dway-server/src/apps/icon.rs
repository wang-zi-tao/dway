use std::{
    any::type_name,
    io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use bevy::{
    asset::{
        io::{AssetReader, AssetReaderError, AssetSource, PathStream, Reader, VecReader},
        meta::{AssetAction, AssetMeta},
        AssetLoader,
    },
    tasks::ConditionalSendFuture,
};
use bevy_svg::prelude::Svg;
use dway_util::{asset_cache::AssetCachePlugin, try_or};
use futures::AsyncReadExt;
use thiserror::Error;
use winnow::{ascii::dec_uint, seq, token::take_while, PResult, Parser};

use crate::prelude::*;

const ICON_EXTENSIONS: &[&str] = &[".png", ".svg", ".jpg", ".jpeg", ".bmp", ".gif"];

#[derive(Error, Debug)]
pub enum LinuxIconError {
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("icon not found")]
    NotFound,
    #[error("failed to load an SVG")]
    SvgError(),
}

#[derive(Clone, Debug, Reflect, PartialEq, Eq)]
pub enum LinuxIconKind {
    Image(Handle<Image>),
    Svg(Handle<Svg>),
}
impl Default for LinuxIconKind {
    fn default() -> Self {
        Self::Image(default())
    }
}

#[derive(Debug, Reflect, Clone, Asset)]
pub struct LinuxIcon {
    pub id: String,
    pub handle: LinuxIconKind,
}

fn parse_icon_path(input: &mut &str) -> PResult<LinuxIconUrl> {
    seq!(
        LinuxIconUrl{
            name:take_while(0.., |c| c != '/').map(|s:&str|s.to_string()),
            _: '/',
            width:dec_uint,
            _:'x',
            height:dec_uint,
        }
    )
    .parse_next(input)
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinuxIconUrl {
    pub name: String,
    pub width: u16,
    pub height: u16,
}

impl FromStr for LinuxIconUrl {
    type Err = String;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        parse_icon_path.parse(s).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinuxIconSettings {
    pub icon: LinuxIconUrl,
}

#[derive(Debug)]
pub struct LinuxIconLoader {
    pub icon_loader: icon_loader::IconLoader,
}
impl Default for LinuxIconLoader {
    fn default() -> Self {
        let mut raw = icon_loader::IconLoader::default();
        raw.set_theme_name_provider(icon_loader::ThemeNameProvider::GTK);
        try_or! { raw.update_theme_name(), "failed to update theme name", () };
        info!("icon theme name: {}", raw.theme_name());
        Self { icon_loader: raw }
    }
}

impl AssetLoader for LinuxIconLoader {
    type Asset = LinuxIcon;
    type Error = LinuxIconError;
    type Settings = LinuxIconSettings;

    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        settings: &LinuxIconSettings,
        load_context: &mut bevy::asset::LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut path = String::new();
            reader.read_to_string(&mut path).await?;

            let path = PathBuf::from(path);

            debug!(
                "loading icon: {:?} {}x{}",
                &path, settings.icon.width, settings.icon.height
            );

            let icon = if path.extension().map(|e| e == "svg").unwrap_or(false) {
                LinuxIconKind::Svg(load_context.load(path))
            } else {
                LinuxIconKind::Image(load_context.load(path))
            };

            Ok(LinuxIcon {
                id: settings.icon.name.clone(),
                handle: icon,
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &[]
    }
}

pub struct LinuxIconReader {
    pub icon_loader: icon_loader::IconLoader,
}

impl Default for LinuxIconReader {
    fn default() -> Self {
        let mut raw = icon_loader::IconLoader::default();
        raw.set_theme_name_provider(icon_loader::ThemeNameProvider::GTK);
        try_or! { raw.update_theme_name(), "failed to update theme name", () };
        info!("icon theme name: {}", raw.theme_name());
        Self { icon_loader: raw }
    }
}

impl AssetReader for LinuxIconReader {
    async fn read<'a>(&'a self, uri: &'a Path) -> Result<VecReader, AssetReaderError> {
        use AssetReaderError::*;
        let icon_info = LinuxIconUrl::from_str(&uri.to_string_lossy())
            .map_err(|e| Io(Arc::new(io::Error::other(e))))?;

        let raw_icon = ICON_EXTENSIONS
            .iter()
            .find_map(|ext| {
                let mut path = icon_info.name.clone();
                path += ext;
                self.icon_loader.load_icon(path)
            })
            .ok_or(AssetReaderError::NotFound(uri.to_owned()))?;

        let icon_file = raw_icon.file_for_size(icon_info.width.max(icon_info.height));

        let data = icon_file.path().to_string_lossy().as_bytes().to_vec();
        Ok(VecReader::new(data))
    }

    async fn read_meta<'a>(&'a self, uri: &'a Path) -> Result<VecReader, AssetReaderError> {
        use AssetReaderError::*;
        let icon_info = LinuxIconUrl::from_str(&uri.to_string_lossy())
            .map_err(|e| Io(Arc::new(io::Error::other(e))))?;
        let data = ron::to_string(&AssetMeta::<LinuxIconLoader, ()> {
            meta_format_version: "1.0".to_string(),
            processed_info: None,
            asset: AssetAction::Load {
                loader: type_name::<LinuxIconLoader>().to_string(),
                settings: LinuxIconSettings { icon: icon_info },
            },
        })
        .map_err(|e| Io(Arc::new(io::Error::other(e))))?;

        Ok(VecReader::new(data.into_bytes()))
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async { Err(AssetReaderError::NotFound(path.to_owned())) })
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<bool, AssetReaderError>> {
        Box::pin(async { Ok(false) })
    }
}

pub struct LinuxIconSourcePlugin;
impl Plugin for LinuxIconSourcePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AssetCachePlugin::<LinuxIcon>::default());
        app.register_asset_source(
            "linuxicon",
            AssetSource::build().with_reader(|| Box::new(LinuxIconReader::default())),
        );
    }
}
