use dway_server::apps::icon::{LinuxIcon, LinuxIconKind};

use crate::prelude::*;

use super::{canvas::UiCanvas, svg::UiSvg};

#[derive(Component, Reflect, Debug)]
pub struct UiIcon {
    pub handle: Handle<LinuxIcon>,
}
impl From<Handle<LinuxIcon>> for UiIcon {
    fn from(value: Handle<LinuxIcon>) -> Self {
        Self { handle: value }
    }
}

#[derive(Bundle)]
pub struct UiIconBundle {
    pub node: ImageBundle,
    pub icon: UiIcon,
}

pub fn uiicon_render(
    mut uiicon_query: Query<
        (Entity, &Node, &ViewVisibility, &UiIcon, &mut UiImage, Option<&mut UiSvg>),
        Changed<UiImage>,
    >,
    icons: Res<Assets<LinuxIcon>>,
    mut commands: Commands,
) {
    uiicon_query.for_each_mut(|(e, node, visibility, icon, mut image, svg)| {
        if node.size() == Vec2::ZERO || !visibility.get() {
            return;
        }
        if let Some(linux_icon) = icons.get(icon.handle.id()) {
            match &linux_icon.handle {
                LinuxIconKind::Image(h) => {
                    if &image.texture != h {
                        image.texture = h.clone();
                    }
                }
                LinuxIconKind::Svg(h) => {
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
        }
    });
}
