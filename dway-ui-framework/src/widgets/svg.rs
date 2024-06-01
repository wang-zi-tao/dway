use crate::{make_bundle, prelude::*, render::mesh::{UiMeshHandle, UiMeshTransform}};
use bevy::{ecs::entity::EntityHashSet, render::render_resource::{AsBindGroup, ShaderRef}, sprite::Material2d, utils::HashSet};
use bevy_svg::prelude::{Svg};

#[derive(Component, Default, Reflect, PartialEq, Eq, Hash)]
pub struct UiSvg {
    handle: Handle<Svg>,
}

impl std::ops::Deref for UiSvg {
    type Target = Handle<Svg>;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl From<Handle<Svg>> for UiSvg {
    fn from(value: Handle<Svg>) -> Self {
        Self { handle: value }
    }
}

impl UiSvg {
    pub fn new(handle: Handle<Svg>) -> Self {
        Self { handle }
    }
}

#[derive(AsBindGroup, Reflect, Debug, Clone, Asset)]
pub struct SvgMagerial {
    pub inner: Svg,
}

impl Material2d for SvgMagerial {
    fn fragment_shader() -> ShaderRef {
        Svg::fragment_shader()
    }
}

make_bundle! {
    @from svg: UiSvg,
    @addon UiSvgExt,
    UiSvgBundle{
        pub mesh: UiMeshHandle,
        pub material: Handle<SvgMagerial>,
        pub svg: UiSvg,
        pub layout: SvgLayout,
        pub mesh_transform: UiMeshTransform,

        pub focus_policy: FocusPolicy,
    }
}

impl UiSvgBundle {
    pub fn new(svg: Handle<Svg>) -> Self{
        Self{
            svg: UiSvg::new(svg),
            ..Default::default()
        }
    }
}

pub fn uisvg_update_system(
    mut query: Query<(
        Entity,
        Ref<Node>,
        Ref<UiSvg>,
        &mut UiMeshHandle,
        &mut Handle<SvgMagerial>,
        Ref<SvgLayout>,
        &mut UiMeshTransform,
        )>,
    assets: Res<Assets<Svg>>,
    mut materials: ResMut<Assets<SvgMagerial>>,
    mut padding_entity: Local<EntityHashSet>,
) {
    for (entity, node, svg, mut mesh, mut material, layout, mut transform) in &mut query {
        let not_init = mesh.id() == Handle::<Mesh>::default().id();
        let padding = padding_entity.is_empty() && padding_entity.remove(&entity);
        if not_init || padding || svg.is_changed() {
            if let Some(asset) = assets.get(&svg.handle) {
                *mesh = asset.mesh.clone().into();
                *material = materials.add( SvgMagerial{ inner:asset.clone() } );
            }
        }
        if not_init || padding || layout.is_changed() || node.is_changed(){
            if let Some(asset) = assets.get(&svg.handle) {
                let node_size = node.size();
                let mut size = Vec2::new( asset.view_box.w as f32, asset.view_box.h as f32 );
                let mut pos = Vec2::ZERO;
                if layout.scale{
                    let scala = f32::min(node_size.x/size.x,node_size.y/size.y);
                    size *= scala;
                    transform.scale = Vec3::new(scala, -scala,1.0);
                }
                match layout.horizontal_align{
                    SvgAlign::None => {},
                    SvgAlign::Begin => {
                        pos.x=0.0;
                    },
                    SvgAlign::Center => {
                        pos.x = 0.5*(node_size.x-size.x);
                    },
                    SvgAlign::End => {
                        pos.x = node_size.x - size.x;
                    },
                }
                match layout.horizontal_align{
                    SvgAlign::None => {},
                    SvgAlign::Begin => {
                        pos.y=0.0;
                    },
                    SvgAlign::Center => {
                        pos.y = 0.5*(node_size.y-size.y);
                    },
                    SvgAlign::End => {
                        pos.y = node_size.y - size.y;
                    },
                }
                match layout.horizontal_align{
                    SvgAlign::None=>{},
                    _=>{
                        transform.translation.x = ( pos.x + asset.view_box.x as f32 * transform.scale.x ) - node_size.x * 0.5;
                    }
                }
                match layout.vertical{
                    SvgAlign::None=>{},
                    _=>{
                        transform.translation.y = ( pos.y + asset.view_box.y as f32 * transform.scale.y ) - node_size.y * 0.5;
                    }
                }
            }else {
                padding_entity.insert(entity);
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum SvgAlign{
    None,
    Begin,
    #[default]
    Center,
    End,
}

#[derive(Clone, Component, Debug, SmartDefault, Reflect)]
pub struct SvgLayout{
    pub horizontal_align: SvgAlign,
    pub vertical: SvgAlign,
    #[default(true)]
    pub scale: bool,
}
