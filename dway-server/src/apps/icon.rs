use std::{
    any::type_name,
    io::{self, SeekFrom},
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
    sync::Arc,
    task::Poll,
};

use bevy::{
    asset::{
        io::{
            AssetReader, AssetReaderError, AssetReaderFuture, AssetSource, AsyncSeekForward,
            PathStream, Reader,
        },
        meta::{AssetAction, AssetMeta},
        AssetLoader,
    },
    tasks::{BoxedFuture, ConditionalSendFuture},
};
use bevy_svg::prelude::Svg;
use dway_util::{asset_cache::AssetCachePlugin, try_or};
use futures::{ready, AsyncSeek, Stream};
use futures_lite::AsyncRead;
use thiserror::Error;
use winnow::{ascii::dec_uint, seq, token::take_while, PResult, Parser};

use crate::prelude::*;

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
        _reader: &mut dyn bevy::asset::io::Reader,
        settings: &LinuxIconSettings,
        load_context: &mut bevy::asset::LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let name = &settings.icon.name;
            let raw_icon = self
                .icon_loader
                .load_icon(format!("{}.svg", name))
                .ok_or(LinuxIconError::NotFound)?;
            let file = raw_icon.file_for_size(settings.icon.width.max(settings.icon.height));
            let path = file.path().to_owned();

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

#[derive(Default)]
struct DataReader {
    data: Vec<u8>,
    bytes_read: usize,
}

impl AsyncRead for DataReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        if self.bytes_read >= self.data.len() {
            Poll::Ready(Ok(0))
        } else {
            let n = ready!(Pin::new(&mut &self.data[self.bytes_read..]).poll_read(cx, buf))?;
            self.bytes_read += n;
            Poll::Ready(Ok(n))
        }
    }
}

impl AsyncSeekForward for DataReader {
    fn poll_seek_forward(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        offset: u64,
    ) -> Poll<std::io::Result<u64>> {
        let result = self
            .bytes_read
            .try_into()
            .map(|bytes_read: u64| bytes_read + offset);

        if let Ok(new_pos) = result {
            self.bytes_read = new_pos as _;

            Poll::Ready(Ok(new_pos as _))
        } else {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek position is out of range",
            )))
        }
    }
}

impl AsyncSeek for DataReader {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        let result = match pos {
            SeekFrom::Start(offset) => offset.try_into(),
            SeekFrom::End(offset) => self.data.len().try_into().map(|len: i64| len - offset),
            SeekFrom::Current(offset) => self
                .bytes_read
                .try_into()
                .map(|bytes_read: i64| bytes_read + offset),
        };

        if let Ok(new_pos) = result {
            if new_pos < 0 {
                Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "seek position is out of range",
                )))
            } else {
                self.bytes_read = new_pos as _;

                Poll::Ready(Ok(new_pos as _))
            }
        } else {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek position is out of range",
            )))
        }
    }
}

impl Reader for DataReader {
}

pub struct LinuxIconReader;

impl AssetReader for LinuxIconReader {
    async fn read<'a>(&'a self, _path: &'a Path) -> Result<DataReader, AssetReaderError> {
        Err(AssetReaderError::NotFound(_path.to_owned()))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<DataReader, AssetReaderError> {
        use AssetReaderError::*;
        let icon_info = LinuxIconUrl::from_str(&path.to_string_lossy())
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
        let reader = DataReader {
            data: data.into_bytes(),
            bytes_read: 0,
        };
        Ok(reader)
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
            AssetSource::build().with_reader(|| Box::new(LinuxIconReader)),
        );
    }
}
