use bevy::{
    prelude::*,
};
use dway_ui_derive::spawn;
use dway_ui_framework::{
    shader::{
        fill::{BlurImage, KawaseLevel2Blur},
        shape::{RoundedRect, Shape},
        ShaderAsset, ShaderPlugin, ShapeRender,
    },
    widgets::bundles::{MiniNodeBundle},
    UiFrameworkPlugin,
};

type BlurImageMaterial = ShapeRender<RoundedRect, BlurImage<KawaseLevel2Blur>>;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            UiFrameworkPlugin,
            ShaderPlugin::<BlurImageMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut assets_blur_image_material: ResMut<Assets<ShaderAsset<BlurImageMaterial>>>,
) {
    commands.spawn((Camera2d::default(), Msaa::Sample4));
    let image = asset_server.load("../../dway/assets/background.jpg");

    spawn!(&mut commands=>
        <MiniNodeBundle @style="full absolute"
            @material(BlurImageMaterial=>RoundedRect::new(16.0).with_effect(BlurImage::new(0.5, image)))
        />
    );
}
