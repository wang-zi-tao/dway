use crate::{make_bundle, prelude::*, render::mesh::{UiMeshHandle, UiMeshTransform}};
use bevy::{render::render_resource::{AsBindGroup, ShaderRef}, sprite::Material2d};
use bevy_svg::prelude::{Origin, Svg};

#[derive(Component, Default, Reflect)]
pub struct UiSvg {
    handle: Handle<Svg>,
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
    UiSvgBundle{
        pub mesh: UiMeshHandle,
        pub material: Handle<SvgMagerial>,
        pub svg: UiSvg,
        pub layout: SvgLayout,
        pub mesh_transform: UiMeshTransform,
    }
}

pub fn uisvg_update_system(
    mut query: Query<(
        Ref<Node>,
        Ref<UiSvg>,
        &mut UiMeshHandle,
        &mut Handle<SvgMagerial>,
        Ref<SvgLayout>,
        &mut UiMeshTransform,
        )>,
    assets: Res<Assets<Svg>>,
    mut materials: ResMut<Assets<SvgMagerial>>,
) {
    for (node, svg, mut mesh, mut material, layout, mut transform) in &mut query {
        let not_init = mesh.id() == Handle::<Mesh>::default().id();
        if not_init || svg.is_changed() {
            if let Some(asset) = assets.get(&svg.handle) {
                *mesh = asset.mesh.clone().into();
                *material = materials.add( SvgMagerial{ inner:asset.clone() } );
            }
        }
        if not_init || layout.is_changed() || node.is_changed(){
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
                        transform.translation.x = pos.x - node_size.x * 0.5;
                    }
                }
                match layout.vertical{
                    SvgAlign::None=>{},
                    _=>{
                        transform.translation.y = pos.y - node_size.y * 0.5;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SvgAlign{
    None,
    Begin,
    #[default]
    Center,
    End,
}

#[derive(Clone, Component, Debug, SmartDefault)]
pub struct SvgLayout{
    pub horizontal_align: SvgAlign,
    pub vertical: SvgAlign,
    #[default(true)]
    pub scale: bool,
}
