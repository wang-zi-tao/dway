use bevy::ecs::{entity::EntityHashSet, world::DeferredWorld};
use bevy_svg::prelude::Svg;
use imports::ComponentId;

use crate::{
    prelude::*,
    render::mesh::{UiMeshHandle, UiMeshTransform},
};

fn svg_on_insert(mut world: DeferredWorld, entity: Entity, _component_id: ComponentId) {
    let handle = world.get::<UiSvg>(entity).unwrap().handle.clone();
    world.commands().entity(entity).insert(MeshMaterial2d(handle));
}

#[derive(Component, Default, Reflect, PartialEq, Eq, Hash, Deref, DerefMut)]
#[require(UiMeshHandle, SvgLayout)]
#[component(on_insert = svg_on_insert)]
pub struct UiSvg {
    #[deref]
    handle: Handle<Svg>,
}

impl From<Handle<Svg>> for UiSvg {
    fn from(handle: Handle<Svg>) -> Self {
        Self { handle }
    }
}

impl UiSvg {
    pub fn new(handle: Handle<Svg>) -> Self {
        Self { handle }
    }
}

pub fn update_uisvg(
    mut query: Query<(
        Entity,
        Ref<ComputedNode>,
        Ref<UiSvg>,
        &mut UiMeshHandle,
        &mut MeshMaterial2d<Svg>,
        Ref<SvgLayout>,
        &mut UiMeshTransform,
    )>,
    assets: Res<Assets<Svg>>,
    mut padding_entity: Local<EntityHashSet>,
) {
    for (entity, computed_ndoe, svg, mut mesh, mut material, layout, mut transform) in &mut query {
        let not_init = mesh.id() == Handle::<Mesh>::default().id();
        let padding = padding_entity.is_empty() && padding_entity.remove(&entity);
        if not_init || padding || svg.is_changed() {
            if let Some(asset) = assets.get(&svg.handle) {
                *mesh = asset.mesh.clone().into();
                *material = MeshMaterial2d(svg.handle.clone())
            }
        }
        if not_init || padding || layout.is_changed() || computed_ndoe.is_changed() {
            if let Some(asset) = assets.get(&svg.handle) {
                let node_size = computed_ndoe.size();
                let mut size = Vec2::new(asset.view_box.w as f32, asset.view_box.h as f32);
                let mut pos = Vec2::ZERO;
                if layout.scale {
                    let scala = f32::min(node_size.x / size.x, node_size.y / size.y);
                    size *= scala;
                    transform.scale = Vec3::new(scala, -scala, 1.0);
                }
                match layout.horizontal_align {
                    SvgAlign::None => {}
                    SvgAlign::Begin => {
                        pos.x = 0.0;
                    }
                    SvgAlign::Center => {
                        pos.x = 0.5 * (node_size.x - size.x);
                    }
                    SvgAlign::End => {
                        pos.x = node_size.x - size.x;
                    }
                }
                match layout.horizontal_align {
                    SvgAlign::None => {}
                    SvgAlign::Begin => {
                        pos.y = 0.0;
                    }
                    SvgAlign::Center => {
                        pos.y = 0.5 * (node_size.y - size.y);
                    }
                    SvgAlign::End => {
                        pos.y = node_size.y - size.y;
                    }
                }
                match layout.horizontal_align {
                    SvgAlign::None => {}
                    _ => {
                        transform.translation.x = (pos.x
                            + asset.view_box.x as f32 * transform.scale.x)
                            - node_size.x * 0.5;
                    }
                }
                match layout.vertical {
                    SvgAlign::None => {}
                    _ => {
                        transform.translation.y = (pos.y
                            + asset.view_box.y as f32 * transform.scale.y)
                            - node_size.y * 0.5;
                    }
                }
            } else {
                padding_entity.insert(entity);
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum SvgAlign {
    None,
    Begin,
    #[default]
    Center,
    End,
}

#[derive(Clone, Component, Debug, SmartDefault, Reflect)]
pub struct SvgLayout {
    pub horizontal_align: SvgAlign,
    pub vertical: SvgAlign,
    #[default(true)]
    pub scale: bool,
}
