use dway_server::apps::icon::IconResorce;

use crate::prelude::*;

use super::{canvas::UiCanvas, svg::UiSvg};

#[derive(Component, Reflect, Debug)]
pub struct UiIcon {
    pub handle: IconResorce,
}
impl From<IconResorce> for UiIcon {
    fn from(value: IconResorce) -> Self {
        Self { handle: value }
    }
}

impl Default for UiIcon {
    fn default() -> Self {
        Self {
            handle: IconResorce::Image(default()),
        }
    }
}

#[derive(Bundle)]
pub struct UiIconBundle {
    pub node: ImageBundle,
    pub icon: UiIcon,
}

pub fn uiicon_render(
    mut uiicon_query: Query<
        (Entity, &Node, &UiIcon, &mut UiImage, Option<&mut UiSvg>),
        Changed<UiImage>,
    >,
    mut commands: Commands,
) {
    uiicon_query.for_each_mut(|(e, node, icon, mut image, svg)| {
        match &icon.handle {
            IconResorce::Image(h) => {
                if &image.texture != h {
                    image.texture = h.clone();
                }
            }
            IconResorce::Svg(h) => {
                if let Some(mut svg) = svg {
                    if &svg.svg != h {
                        svg.svg = h.clone();
                    }
                } else {
                    commands
                        .entity(e)
                        .insert((UiSvg { svg: h.clone() }, UiCanvas::default()));
                }
            }
        };
    });
}
