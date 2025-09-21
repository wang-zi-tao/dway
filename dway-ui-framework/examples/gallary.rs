use std::{path::PathBuf, time::Duration};

use bevy::{
    asset::io::embedded::EmbeddedAssetRegistry,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_prototype_lyon::{
    draw::{Fill, Stroke},
    entity::Shape,
    geometry::LyonPathBuilderExt,
    path::ShapePath,
    prelude::{tess::path::traits::SvgPathBuilder, ShapeBuilder, ShapeBuilderBase},
    shapes::{self, RegularPolygonFeature},
};
use bevy_svg::prelude::{LineJoin, StrokeOptions};
use dway_ui_derive::dway_widget;
use dway_ui_framework::{
    prelude::*,
    render::mesh::{UiMeshHandle, UiMeshTransform},
    widgets::{
        inputbox::UiInputBox, scroll::UiScroll, shader::{
            arc_material, button_material, checkbox_material, clicked_fake3d_button_material,
            fake3d_button_material, hollow_block, rainbow_block, rounded_block,
            rounded_border_block, rounded_inner_shadow_block, rounded_rect, ArcMaterial,
            CheckboxMaterial, Fake3dButton, HollowBlockMaterial, RoundedBlockMaterial,
            RoundedBorderBlockMaterial, RoundedInnerShadowBlockMaterial,
            RoundedRainbowBlockMaterial, RoundedUiImageMaterial, RoundedUiRectMaterial,
            UiCircleMaterial,
        }, shape::UiShapeMaterial, text::UiTextBundle
    },
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
            dway_ui_framework::UiFrameworkPlugin,
            GallaryPlugin,
        ))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_plugins((
            RoundedUiRectMaterial::plugin(),
            UiCircleMaterial::plugin(),
            RoundedUiImageMaterial::plugin(),
            RoundedBlockMaterial::plugin(),
            RoundedBorderBlockMaterial::plugin(),
            HollowBlockMaterial::plugin(),
            // ButtonMaterial::plugin(),
            RoundedRainbowBlockMaterial::plugin(),
            Fake3dButton::plugin(),
            CheckboxMaterial::plugin(),
            RoundedInnerShadowBlockMaterial::plugin(),
            ArcMaterial::plugin(),
            AssetAnimationPlugin::<CheckboxMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(Update, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .insert_resource(ClearColor(Color::WHITE));

    {
        let embedded = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
        embedded.insert_asset(std::path::PathBuf::new(), &PathBuf::from("dway_ui_framework/examples/gallary/power.svg"), r###"
<svg width="800px" height="800px" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12.5 6L8.5 12H14.5L10.5 18M21 13V11M7.7 6H6.2C5.0799 6 4.51984 6 4.09202 6.21799C3.71569 6.40973 3.40973 6.71569 3.21799 7.09202C3 7.51984 3 8.0799 3 9.2V14.8C3 15.9201 3 16.4802 3.21799 16.908C3.40973 17.2843 3.71569 17.5903 4.09202 17.782C4.51984 18 5.0799 18 6.2 18H6.5M16.5 6H16.8C17.9201 6 18.4802 6 18.908 6.21799C19.2843 6.40973 19.5903 6.71569 19.782 7.09202C20 7.51984 20 8.0799 20 9.2V14.8C20 15.9201 20 16.4802 19.782 16.908C19.5903 17.2843 19.2843 17.5903 18.908 17.782C18.4802 18 17.9201 18 16.8 18H15.31" stroke="#6791C9" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"###.to_string().into_bytes());
    }

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d::default(), Msaa::Sample4));
    commands.spawn(Gallary);
}

fn grid_style() -> Node {
    Node {
        height: Val::Percent(100.0),
        aspect_ratio: Some(1.0),
        display: Display::Grid,
        padding: UiRect::all(Val::Px(24.0)),
        grid_template_columns: RepeatedGridTrack::flex(4, 1.0),
        grid_template_rows: RepeatedGridTrack::flex(6, 1.0),
        row_gap: Val::Px(12.0),
        column_gap: Val::Px(12.0),
        margin: UiRect::all(Val::Px(8.0)),
        ..default()
    }
}

fn cell_style() -> Node {
    Node {
        margin: UiRect::all(Val::Px(8.0)),
        min_width: Val::Px(128.0),
        min_height: Val::Px(64.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn button_style() -> Node {
    Node {
        margin: UiRect::all(Val::Px(8.0)),
        min_width: Val::Px(64.0),
        min_height: Val::Px(32.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..style!("p-8 m-8")
    }
}

fn checkbox_style() -> Node {
    Node {
        margin: UiRect::all(Val::Px(8.0)),
        width: Val::Px(64.0),
        height: Val::Px(32.0),
        ..default()
    }
}

#[derive(Component, Default)]
pub struct Gallary;

dway_widget! {
Gallary=>
@global(theme: Theme)
@global(asset_server: AssetServer)
@global(mut meshes: Assets<Mesh>)
@global(mut mesh2d_materials: Assets<ColorMaterial>)
<Node Node=(grid_style())
    @material(RoundedBlockMaterial=>rounded_block(color!("#dddddd"), 16.0, &theme))
>
    <Node Node=(cell_style())
        @material(RoundedUiRectMaterial=>rounded_rect(color!("#ffffff"), 16.0)) >
        <(UiTextBundle::new("block", 24, &theme))/>
    </>
    <Node Node=(cell_style())
        @material(RoundedBlockMaterial=>rounded_block(color!("#ffffff"), 16.0, &theme)) >
        <(UiTextBundle::new("block with shadow", 24, &theme))/>
    </>
    <Node Node=(cell_style())
        @material(HollowBlockMaterial=>hollow_block(theme.color("blue"), 16.0, 2.0)) >
        <(UiTextBundle::new("hollow block", 24, &theme))/>
    </>
    <Node Node=(cell_style())
        @material(RoundedRainbowBlockMaterial=>rainbow_block(16.0, 2.0)) >
        <(UiTextBundle::new("rainbow block", 24, &theme))/>
    </>
    <Node Node=(cell_style())>
        <Node Node=(button_style())
            @material(RoundedBlockMaterial=>button_material(theme.color("blue"), 8.0, &theme)) >
            <((Text::new("button"), theme.text_font(24.0), TextColor(Color::WHITE)))/>
        </>
    </>
    <Node Node=(cell_style())>
        <Node Node=(button_style())
            @material(RoundedBlockMaterial=>button_material(color!("#ffffff"), 8.0, &theme)) >
            <((Text::new("button"), theme.text_font(24.0), TextColor(color!("#0000ff"))))/>
        </>
    </>
    <Node Node=(cell_style())>
        <Node Node=(button_style())
            @material(Fake3dButton=>fake3d_button_material(color!("#ffffff"), 4.0)) >
            <(UiTextBundle::new("3d button", 24, &theme))/>
        </>
    </>
    <Node Node=(cell_style())>
        <Node Node=(button_style())
            @material(Fake3dButton=>clicked_fake3d_button_material(color!("#ffffff"), 4.0)) >
            <(UiTextBundle::new("3d button", 24, &theme))/>
        </>
    </>
    <Node Node=(cell_style())>
        <Node Node=(checkbox_style())
            @material(CheckboxMaterial=>checkbox_material(false, Vec2::new(64.0,32.0), &theme))
        >
        </>
    </Node>
    <Node Node=(cell_style())>
        <Node @style="p-8 w-full m-8" @material(RoundedBorderBlockMaterial=>rounded_border_block(Color::WHITE,theme.color("blue"), 8.0, 2.0)) >
            <UiInputBox UiInputBox=(UiInputBox{
                placeholder: "input box...".into(),
                ..Default::default()
            })/>
        </>
    </Node>
    <Node Node=(cell_style())>
        <Node @style="p-8 w-full m-8" @material(RoundedInnerShadowBlockMaterial=>rounded_inner_shadow_block(Color::WHITE, 8.0, &theme)) >
            <UiInputBox UiInputBox=(UiInputBox{
                placeholder: "input box...".into(),
                ..Default::default()
            })/>
        </>
    </Node>
    <Node Node=(cell_style())>
        <UiSlider @style="w-full" />
    </Node>
    <Node Node=(cell_style())>
        <Node @style="w-128 h-128 align-items:center justify-content:center"
            @material(ArcMaterial=>arc_material(color!("#00ff00"), Color::WHITE, 8.0, [0.0,5.28]))
        >
            <(( UiSvg::from(asset_server.load("embedded://dway_ui_framework/examples/gallary/power.svg")),
                UiMeshTransform::new(Transform::default()
                                                    .with_translation(Vec3::new(-24.0,-24.0,0.0))
                                                    .with_scale(Vec3::new(2.0,-2.0,1.0))),
                style!("w-64 h-64"), )
            )/>
        </>
    </Node>
    <Node Node=(cell_style())
        @material(HollowBlockMaterial=>hollow_block(theme.color("blue"), 16.0, 2.0)) >
        <(UiMeshHandle::from(meshes.add(RegularPolygon::new(48.0, 6))))
        UiShapeMaterial=(mesh2d_materials.add(color!("#ff0000")).into())
        @style="w-64 h-64"
    />
    </Node>
    <Node Node=(cell_style())
        @material(HollowBlockMaterial=>hollow_block(theme.color("blue"), 16.0, 2.0)) >
        <UiShape @style="w-120 h-120 m-8"
        UiMeshTransform=(Transform::default().with_translation(Vec3::new(-64.0,-64.0,0.0)).with_scale(Vec3::splat(1.0/8.0)).into())
        Shape=(ShapeBuilder::with(&shapes::SvgPathShape {
                svg_doc_size_in_px: Vec2::splat(0.0),
                svg_path_string: "M280-240q-100 0-170-70T40-480q0-100 70-170t170-70h400q100 0 170 70t70 170q0 100-70 170t-170 70H280Zm0-80h400q66 0 113-47t47-113q0-66-47-113t-113-47H280q-66 0-113 47t-47 113q0 66 47 113t113 47Zm400-40q50 0 85-35t35-85q0-50-35-85t-85-35q-50 0-85 35t-35 85q0 50 35 85t85 35ZM480-480Z".to_string()
            }).fill(Fill::color(color!("#0000ff"))).stroke(Stroke::new(Color::BLACK,8.0) ).build())/>
    </Node>
    <Node Node=(cell_style())
        @material(HollowBlockMaterial=>hollow_block(theme.color("blue"), 16.0, 2.0)) >
        <UiShape @style="w-120 h-120 m-8"
        Shape=(ShapeBuilder::with(&shapes::RegularPolygon {
            sides: 8,
            feature: RegularPolygonFeature::Radius(48.0),
            ..default()
        }).fill(Fill::color(color!("#ffff00"))).stroke(Stroke{
            color: Color::BLACK,
            options: StrokeOptions::default().with_line_join(LineJoin::Round).with_line_width(16.0)
        }).build()) />
    </Node>
    <Node Node=(cell_style())
        @material(RoundedInnerShadowBlockMaterial=>rounded_inner_shadow_block(Color::WHITE, 8.0, &theme)) >
        <UiScroll @style="w-120 h-120 m-8">
            <(UiTextBundle::new("scroll\nscroll\nscroll\nscroll\nscroll\nscroll\nscroll", 24, &theme)) @style="w-256 h-256 left-4"/>
        </UiScroll>
    </Node>
</Node>
}
