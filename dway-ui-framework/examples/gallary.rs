use std::path::PathBuf;
use std::time::Duration;

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::asset::load_internal_asset;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use dway_ui::animation::AssetAnimationPlugin;
use dway_ui::render::mesh::UiMeshBundle;
use dway_ui::render::mesh::UiMeshHandle;
use dway_ui::render::mesh::UiMeshTransform;
use dway_ui::shader::ShaderPlugin;
use dway_ui::widgets::button::UiButtonAddonBundle;
use dway_ui::widgets::button::UiButtonBundle;
use dway_ui::widgets::inputbox::UiInputBox;
use dway_ui::widgets::inputbox::UiInputBoxBundle;
use dway_ui::widgets::scroll::UiScrollBundle;
use dway_ui::widgets::slider::UiSliderBundle;
use dway_ui::widgets::svg::UiSvg;
use dway_ui::widgets::svg::UiSvgBundle;
use dway_ui::widgets::text::UiTextAddonBundle;
use dway_ui::widgets::text::UiTextBundle;
use dway_ui_derive::color;
use dway_ui_derive::dway_widget;
use dway_ui_framework::prelude::*;
use interpolation::EaseFunction;

const SVG_HANDLE: Handle<Shader> = Handle::weak_from_u128(15628284168829255748903736059973599232);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
            dway_ui_framework::UiFrameworkPlugin,
            GallaryPlugin,
            RoundedUiRectMaterial::plugin(),
            UiCircleMaterial::plugin(),
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
        let embedded = app.world.resource_mut::<EmbeddedAssetRegistry>();
        embedded.insert_asset(std::path::PathBuf::new(), &PathBuf::from("dway_ui_framework/examples/gallary/power.svg"), r###"
<svg width="800px" height="800px" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12.5 6L8.5 12H14.5L10.5 18M21 13V11M7.7 6H6.2C5.0799 6 4.51984 6 4.09202 6.21799C3.71569 6.40973 3.40973 6.71569 3.21799 7.09202C3 7.51984 3 8.0799 3 9.2V14.8C3 15.9201 3 16.4802 3.21799 16.908C3.40973 17.2843 3.71569 17.5903 4.09202 17.782C4.51984 18 5.0799 18 6.2 18H6.5M16.5 6H16.8C17.9201 6 18.4802 6 18.908 6.21799C19.2843 6.40973 19.5903 6.71569 19.782 7.09202C20 7.51984 20 8.0799 20 9.2V14.8C20 15.9201 20 16.4802 19.782 16.908C19.5903 17.2843 19.2843 17.5903 18.908 17.782C18.4802 18 17.9201 18 16.8 18H15.31" stroke="#6791C9" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"###.to_string().into_bytes());
    }

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(GallaryBundle::default());
}

fn grid_style() -> Style {
    Style {
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

fn cell_style() -> Style {
    Style {
        margin: UiRect::all(Val::Px(8.0)),
        min_width: Val::Px(128.0),
        min_height: Val::Px(64.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

fn button_style() -> Style {
    Style {
        margin: UiRect::all(Val::Px(8.0)),
        min_width: Val::Px(64.0),
        min_height: Val::Px(32.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..style!("p-8 m-8")
    }
}

fn checkbox_style() -> Style {
    Style {
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
<MiniNodeBundle Style=(grid_style())
    @material(RoundedBlockMaterial=>rounded_block(color!("#dddddd"), 16.0, &theme))
>
    <MiniButtonBundle Style=(cell_style())
        @material(RoundedUiRectMaterial=>rounded_rect(color!("#ffffff"), 16.0)) >
        <(UiTextBundle::new("block", 24, &theme))/>
    </>
    <MiniButtonBundle Style=(cell_style())
        @material(RoundedBlockMaterial=>rounded_block(color!("#ffffff"), 16.0, &theme)) >
        <(UiTextBundle::new("block with shadow", 24, &theme))/>
    </>
    <MiniButtonBundle Style=(cell_style())
        @material(HollowBlockMaterial=>hollow_block(theme.color("blue"), 16.0, 2.0)) >
        <(UiTextBundle::new("hollow block", 24, &theme))/>
    </>
    <MiniButtonBundle Style=(cell_style())
        @material(RoundedRainbowBlockMaterial=>rainbow_block(16.0, 2.0)) >
        <(UiTextBundle::new("rainbow block", 24, &theme))/>
    </>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle Style=(button_style())
            @material(RoundedBlockMaterial=>button_material(theme.color("blue"), 8.0, &theme)) >
            <(UiTextBundle::from( Text::from_section(
                    "button",
                    TextStyle {
                        font: theme.default_font(),
                        font_size: 24 as f32,
                        color: Color::WHITE,
                    },
            ) ))/>
        </>
    </>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle Style=(button_style())
            @material(RoundedBlockMaterial=>button_material(color!("#ffffff"), 8.0, &theme)) >
            <(UiTextBundle::from( Text::from_section(
                    "button",
                    TextStyle {
                        font: theme.default_font(),
                        font_size: 24 as f32,
                        color: Color::BLUE,
                    },
            ) ))/>
        </>
    </>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle Style=(button_style())
            @material(Fake3dButton=>fake3d_button_material(color!("#ffffff"), 4.0)) >
            <(UiTextBundle::new("3d button", 24, &theme))/>
        </>
    </>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle Style=(button_style())
            @material(Fake3dButton=>clicked_fake3d_button_material(color!("#ffffff"), 4.0)) >
            <(UiTextBundle::new("3d button", 24, &theme))/>
        </>
    </>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle Style=(checkbox_style())
            @material(CheckboxMaterial=>checkbox_material(false, Vec2::new(64.0,32.0), &theme))
        >
        </>
    </MiniNodeBundle>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle @style="p-8 w-full m-8" @material(RoundedBorderBlockMaterial=>rounded_border_block(Color::WHITE,theme.color("blue"), 8.0, 2.0)) >
            <UiInputBoxBundle UiInputBox=(UiInputBox{
                placeholder: "input box...".into(),
                ..Default::default()
            })/>
        </>
    </MiniNodeBundle>
    <MiniNodeBundle Style=(cell_style())>
        <MiniButtonBundle @style="p-8 w-full m-8" @material(RoundedInnerShadowBlockMaterial=>rounded_inner_shadow_block(Color::WHITE, 8.0, &theme)) >
            <UiInputBoxBundle UiInputBox=(UiInputBox{
                placeholder: "input box...".into(),
                ..Default::default()
            })/>
        </>
    </MiniNodeBundle>
    <MiniNodeBundle Style=(cell_style())>
        <UiSliderBundle @style="w-full" />
    </MiniNodeBundle>
    <MiniNodeBundle Style=(cell_style())>
        <UiButtonBundle @style="w-128 h-128 align-items:center justify-content:center"
            @material(ArcMaterial=>arc_material(Color::GREEN, Color::WHITE, 8.0, [0.0,5.28]))
        >
            <( UiSvgBundle{
                svg: UiSvg::from(asset_server.load("embedded://dway_ui_framework/examples/gallary/power.svg")),
                mesh_transform: UiMeshTransform::new(Transform::default()
                                                    .with_translation(Vec3::new(-24.0,-24.0,0.0))
                                                    .with_scale(Vec3::new(2.0,-2.0,1.0))),
                style: style!("w-64 h-64"),
                ..default()
            })/>
        </>
    </MiniNodeBundle>
    <MiniNodeBundle Style=(cell_style())
        @material(HollowBlockMaterial=>hollow_block(theme.color("blue"), 16.0, 2.0)) >
        <( UiMeshBundle{
            mesh: UiMeshHandle::from(meshes.add(RegularPolygon::new(48.0, 6))),
            material: mesh2d_materials.add(Color::RED),
            style: style!("w-64 h-64"),
            ..default()
        })/>
    </MiniNodeBundle>
    <MiniNodeBundle Style=(cell_style())
        @material(RoundedInnerShadowBlockMaterial=>rounded_inner_shadow_block(Color::WHITE, 8.0, &theme)) >
        <UiScrollBundle @style="w-120 h-120 m-8">
            <(UiTextBundle::new("scroll\nscroll\nscroll\nscroll\nscroll\nscroll\nscroll", 24, &theme)) @style="w-256 h-256 left-4"/>
        </UiScrollBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
