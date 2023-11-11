use std::num::NonZeroUsize;

use bevy::{
    asset::LoadState,
    ui::{widget::UiImageSize, ContentSize},
};
use bevy_svg::prelude::{Origin, Svg, Svg2dBundle};
use dway_util::temporary::TemporaryEntity;
use lru::LruCache;

use crate::prelude::*;

use super::canvas::{UiCanvas, UiCanvasRenderCommand};

#[derive(Component, Debug, Clone, Reflect, Default)]
pub struct UiSvg {
    pub svg: Handle<bevy_svg::prelude::Svg>,
}

impl From<Handle<Svg>> for UiSvg {
    fn from(value: Handle<Svg>) -> Self {
        UiSvg { svg: value }
    }
}

pub fn uisvg_render(
    mut svg_query: Query<(
        &UiSvg,
        &Node,
        &mut UiCanvasRenderCommand,
        &mut UiCanvas,
        &mut UiImage,
    )>,
    mut commands: Commands,
    mut cache: ResMut<SvgImageCache>,
    svgs: Res<Assets<Svg>>,
    asset_server: Res<AssetServer>,
) {
    svg_query.for_each_mut(
        |(ui_svg, node, mut render_command, mut ui_canvas, mut ui_image)| {
            if let Some(image) = cache.cache.get(&(ui_svg.svg.id(), node.size().as_ivec2())) {
                ui_image.texture = image.clone();
                ui_canvas.set_image(image.clone());
                return;
            }
            let svg_size = svgs
                .get(&ui_svg.svg)
                .map(|svg| svg.size)
                .unwrap_or(node.size());
            commands.spawn((
                Svg2dBundle {
                    svg: ui_svg.svg.clone(),
                    origin: Origin::Center,
                    transform: render_command.transform()
                        * Transform::default().with_scale(Vec3::new(
                            node.size().x / svg_size.x,
                            node.size().y / svg_size.y,
                            1.0,
                        )),
                    ..default()
                },
                TemporaryEntity,
            ));
            match asset_server.load_state(&ui_svg.svg) {
                LoadState::Loading => {
                    dbg!(&ui_svg);
                    render_command.continue_rending();
                }
                _ => {
                    cache.cache.put(
                        (ui_svg.svg.id(), node.size().as_ivec2()),
                        ui_image.texture.clone(),
                    );
                }
            }
        },
    )
}

#[derive(Bundle, Default)]
pub struct UiSvgBundle {
    pub svg: UiSvg,
    pub canvas: UiCanvas,
    pub node: ImageBundle,
}
impl UiSvgBundle {
    pub fn new(svg: Handle<Svg>) -> Self {
        Self {
            svg: UiSvg { svg },
            ..Default::default()
        }
    }
}

#[derive(Resource)]
pub struct SvgImageCache {
    pub cache: LruCache<(AssetId<Svg>, IVec2), Handle<Image>>,
}
impl Default for SvgImageCache {
    fn default() -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(1024).unwrap()),
        }
    }
}

#[derive(Bundle, Default)]
pub struct UiSvgAddonBundle {
    pub svg: UiSvg,
    pub canvas: UiCanvas,
    pub image: UiImage,
    pub image_size: UiImageSize,
    pub calculated_size: ContentSize,
}
impl UiSvgAddonBundle {
    pub fn new(svg: Handle<Svg>) -> Self {
        Self {
            svg: UiSvg { svg },
            ..Default::default()
        }
    }
}

impl From<Handle<Svg>> for UiSvgAddonBundle {
    fn from(value: Handle<Svg>) -> Self {
        Self::new(value)
    }
}
