use bevy::sprite::MaterialMesh2dBundle;
use bevy_svg::prelude::{Origin, Svg, Svg2dBundle};
use dway_util::temporary::{TemporaryEntity, TemporaryTree};

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
    mut svg_query: Query<(&UiSvg, &Node, &UiCanvasRenderCommand)>,
    mut commands: Commands,
    svgs: Res<Assets<Svg>>,
) {
    svg_query.for_each_mut(|(ui_svg, node, render_command)| {
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
    })
}

#[derive(Bundle, Default)]
pub struct UiSvgBundle {
    pub svg: UiSvg,
    pub canvas: UiCanvas,
    pub node: ImageBundle,
}
