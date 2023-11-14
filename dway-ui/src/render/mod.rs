use crate::prelude::*;
use bevy::{
    asset::{embedded_asset, load_internal_asset},
    render::render_resource::{AsBindGroup, ShaderType},
};

#[derive(AsBindGroup, Asset, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct UiCircleMaterial {
    #[uniform(0)]
    pub settings: UiCircleSettings,
}

impl UiCircleMaterial {
    pub fn new(color: Color, corner: f32) -> Self {
        Self {
            settings: UiCircleSettings { color, corner },
        }
    }
}

#[derive(Debug, Clone, Default, ShaderType, Reflect)]
pub struct UiCircleSettings {
    pub color: Color,
    pub corner: f32,
}

impl UiMaterial for UiCircleMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://dway_ui/render/shapes/circle.wgsl".into()
    }
}

#[derive(AsBindGroup, Asset, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct RoundedUiRectMaterial {
    #[uniform(0)]
    pub settings: RoundedUiRectSettings,
}

impl RoundedUiRectMaterial {
    pub fn new(color: Color, corner: f32) -> Self {
        Self {
            settings: RoundedUiRectSettings {
                color,
                corner,
                size: Vec2::new(1.0, 1.0),
            },
        }
    }
}

#[derive(Debug, Clone, Default, ShaderType, Reflect)]
pub struct RoundedUiRectSettings {
    pub color: Color,
    pub size: Vec2,
    pub corner: f32,
}

impl UiMaterial for RoundedUiRectMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://dway_ui/render/shapes/rounded_uirect.wgsl".into()
    }
}

pub fn update_material_size(
    ui_rect_query: Query<(&Node, &GlobalTransform, &Handle<RoundedUiRectMaterial>), Changed<Node>>,
    ui_image_query: Query<(&Node, &Handle<RoundedUiImageMaterial>), Changed<Node>>,
    mut rect_material_set: ResMut<Assets<RoundedUiRectMaterial>>,
    mut image_material_set: ResMut<Assets<RoundedUiImageMaterial>>,
) {
    ui_rect_query.for_each(|(node, trans, handle)| {
        let Some(group) = rect_material_set.get_mut(handle.id()) else {
            return;
        };
        group.settings.size = node.size();
    });
    ui_image_query.for_each(|(node, handle)| {
        let Some(group) = image_material_set.get_mut(handle.id()) else {
            return;
        };
        group.settings.size = node.size();
    });
}

#[derive(Debug, Clone, Default, ShaderType, Reflect)]
pub struct RoundedUiImageSettings {
    pub min_uv: Vec2,
    pub size_uv: Vec2,
    pub size: Vec2,
    pub corner: f32,
}

#[derive(AsBindGroup, Asset, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct RoundedUiImageMaterial {
    #[uniform(0)]
    pub settings: RoundedUiImageSettings,
    #[texture(1)]
    #[sampler(2)]
    pub image: Handle<Image>,
}

impl RoundedUiImageMaterial {
    pub fn new(
        size: Vec2,
        corner: f32,
        image_offset: Vec2,
        image_size: Vec2,
        image: Handle<Image>,
    ) -> Self {
        Self {
            settings: RoundedUiImageSettings {
                size,
                corner,
                min_uv: -image_offset / image_size,
                size_uv: size / image_size,
            },
            image,
        }
    }
}

impl UiMaterial for RoundedUiImageMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://dway_ui/render/shapes/rounded_uiimage.wgsl".into()
    }
}

#[derive(Debug, Clone, Default, ShaderType, Reflect)]
pub struct KwawaseRoundedRectMaterialSetting{
    pub size: Vec2,
    pub corner: f32,
}
#[derive(AsBindGroup, Asset, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct KwawaseRoundedRectMaterial {
    #[uniform(0)]
    pub settings: KwawaseRoundedRectMaterialSetting,
    #[texture(1)]
    #[sampler(2)]
    pub image: Handle<Image>,
}
impl UiMaterial for KwawaseRoundedRectMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://dway_ui/render/blur/kawase.wgsl".into()
    }
}

const SHAPES_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(121183247875205365928133316463072513415u128);

pub type RounndedRectBundle = MaterialNodeBundle<RoundedUiRectMaterial>;

pub struct DWayUiMaterialPlugin;
impl Plugin for DWayUiMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHAPES_HANDLE, "shapes/shapes.wgsl", Shader::from_wgsl);
        embedded_asset!(app, "shapes/rounded_uirect.wgsl");
        embedded_asset!(app, "shapes/rounded_uiimage.wgsl");
        embedded_asset!(app, "shapes/circle.wgsl");
        embedded_asset!(app, "blur/kawase.wgsl");
        app.add_plugins(UiMaterialPlugin::<RoundedUiRectMaterial>::default())
            .add_plugins(UiMaterialPlugin::<RoundedUiImageMaterial>::default())
            .add_plugins(UiMaterialPlugin::<UiCircleMaterial>::default())
            .register_asset_reflect::<RoundedUiRectMaterial>()
            .register_asset_reflect::<RoundedUiImageMaterial>()
            .register_asset_reflect::<UiCircleMaterial>()
            .add_systems(Last, update_material_size);
    }
}
