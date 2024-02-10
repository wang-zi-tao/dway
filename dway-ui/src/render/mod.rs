pub mod sharder_ml;
use crate::prelude::*;
use bevy::{
    asset::{embedded_asset, load_internal_asset},
    render::render_resource::{AsBindGroup, ShaderType},
};
use bevy_tweening::{asset_animator_system, component_animator_system, AnimationSystem};

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

impl From<(Color, f32)> for UiCircleMaterial {
    fn from((color, coner): (Color, f32)) -> Self {
        Self::new(color, coner)
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

impl From<(Color, f32)> for RoundedUiRectMaterial {
    fn from(value: (Color, f32)) -> Self {
        Self::new(value.0, value.1)
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
    ui_rect_query: Query<(&Node, &Handle<RoundedUiRectMaterial>), Changed<Node>>,
    ui_image_query: Query<(&Node, &Handle<RoundedUiImageMaterial>), Changed<Node>>,
    ui_circle_query: Query<(&Node, &Handle<UiCircleMaterial>), Changed<Node>>,
    ui_shadow_query: Query<(&Node, &Handle<ShadowUiRectMaterial>), Changed<Node>>,
    mut rect_material_set: ResMut<Assets<RoundedUiRectMaterial>>,
    mut image_material_set: ResMut<Assets<RoundedUiImageMaterial>>,
    mut circle_material_set: ResMut<Assets<UiCircleMaterial>>,
    mut shadow_material_set: ResMut<Assets<ShadowUiRectMaterial>>,
) {
    ui_rect_query.for_each(|(node, handle)| {
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
    ui_circle_query.for_each(|(node, handle)| {
        let Some(group) = circle_material_set.get_mut(handle.id()) else {
            return;
        };
        group.settings.corner = 0.5 * (node.size().x.min(node.size().y));
    });
    ui_shadow_query.for_each(|(node, handle)| {
        let Some(group) = shadow_material_set.get_mut(handle.id()) else {
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
pub struct KwawaseRoundedRectMaterialSetting {
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

structstruck::strike! {
#[derive(AsBindGroup, Asset, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct ShadowUiRectMaterial {
    #[uniform(0)]
    pub settings:
        #[derive(Debug, Clone, Default, ShaderType, Reflect)]
        struct ShadowUiRectSettings {
            pub color: Color,
            pub size: Vec2,
            pub corner: f32,
            pub shadow_color: Color,
            pub shadow_offset: Vec2,
            pub shadow_margin: Vec2,
            pub shadow_radius: f32,
        },
    }
}
impl ShadowUiRectMaterial {
    pub fn new(
        color: Color,
        corner: f32,
        shadow_color: Color,
        shadow_offset: Vec2,
        shadow_margin: Vec2,
        shadow_radius: f32,
    ) -> Self {
        Self {
            settings: ShadowUiRectSettings {
                color,
                size: Vec2::ONE,
                corner,
                shadow_color,
                shadow_offset,
                shadow_margin,
                shadow_radius,
            },
        }
    }
}
impl UiMaterial for ShadowUiRectMaterial {
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://dway_ui/render/ui/shadow.wgsl".into()
    }

    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://dway_ui/render/ui/shadow.wgsl".into()
    }

    fn specialize(
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        key: UiMaterialKey<Self>,
    ) {
    }
}

const SHAPES_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(121183247875205365928133316463072513415u128);

pub type RounndedRectBundle = MaterialNodeBundle<RoundedUiRectMaterial>;

pub struct DWayUiMaterialPlugin;
impl Plugin for DWayUiMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHAPES_HANDLE, "shapes/shapes.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, SHAPES_HANDLE, "shapes/shapes.wgsl", Shader::from_wgsl);
        embedded_asset!(app, "shapes/rounded_uirect.wgsl");
        embedded_asset!(app, "shapes/rounded_uiimage.wgsl");
        embedded_asset!(app, "shapes/circle.wgsl");
        embedded_asset!(app, "ui/shadow.wgsl");
        embedded_asset!(app, "blur/kawase.wgsl");
        app.add_plugins(UiMaterialPlugin::<RoundedUiRectMaterial>::default())
            .add_plugins(UiMaterialPlugin::<RoundedUiImageMaterial>::default())
            .add_plugins(UiMaterialPlugin::<UiCircleMaterial>::default())
            .add_plugins(UiMaterialPlugin::<ShadowUiRectMaterial>::default())
            .register_asset_reflect::<RoundedUiRectMaterial>()
            .register_asset_reflect::<RoundedUiImageMaterial>()
            .register_asset_reflect::<UiCircleMaterial>()
            .register_asset_reflect::<ShadowUiRectMaterial>()
            .add_systems(
                Update,
                (
                    asset_animator_system::<RoundedUiRectMaterial>,
                    asset_animator_system::<RoundedUiImageMaterial>,
                    asset_animator_system::<UiCircleMaterial>,
                    asset_animator_system::<ShadowUiRectMaterial>,
                )
                    .in_set(AnimationSystem::AnimationUpdate),
            )
            .add_systems(Last, update_material_size);
    }
}

impl Lens<RoundedUiRectMaterial> for ColorMaterialColorLens {
    fn lerp(&mut self, target: &mut RoundedUiRectMaterial, ratio: f32) {
        let start: Vec4 = self.start.into();
        let end: Vec4 = self.end.into();
        let value = start.lerp(end, ratio);
        target.settings.color = value.into();
    }
}

impl Lens<UiCircleMaterial> for ColorMaterialColorLens {
    fn lerp(&mut self, target: &mut UiCircleMaterial, ratio: f32) {
        let start: Vec4 = self.start.into();
        let end: Vec4 = self.end.into();
        let value = start.lerp(end, ratio);
        target.settings.color = value.into();
    }
}
