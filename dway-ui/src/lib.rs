#![feature(arc_unwrap_or_clone)]
pub mod assets;
pub mod framework;
pub mod panels;
pub mod popups;
pub mod prelude;
pub mod render;
pub mod sprite;
pub mod theme;
pub mod util;
pub mod widgets;

use crate::{prelude::*, widgets::applist::AppListUIBundle};
use bevy::{render::camera::RenderTarget, ui::FocusPolicy};
use bevy_svg::SvgPlugin;
use bevy_tweening::TweeningPlugin;
pub use bitflags::bitflags as __bitflags;
use dway_tty::{drm::surface::DrmSurface, seat::SeatState};
use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use widgets::clock::ClockBundle;

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        app.add_plugins((
            TweeningPlugin,
            assets::DWayAssetsPlugin,
            render::DWayUiMaterialPlugin,
            theme::ThemePlugin,
            framework::UiFrameworkPlugin,
            widgets::clock::ClockUiPlugin,
            widgets::window::WindowUIPlugin,
            widgets::popupwindow::PopupUIPlugin,
            widgets::applist::AppListUIPlugin,
            widgets::popup::PopupUiPlugin,
            popups::app_window_preview::AppWindowPreviewPopupPlugin,
        ));
        app.add_systems(Startup, (setup, init_ui));
    }
}

pub fn default_system_font() -> Option<String> {
    let source = SystemSource::new();
    let default_fonts = &[
        FamilyName::Title("arial".to_string()),
        FamilyName::SansSerif,
        FamilyName::Monospace,
        FamilyName::Fantasy,
    ];
    let font = source
        .select_best_match(
            default_fonts,
            Properties::new().style(font_kit::properties::Style::Normal),
        )
        .ok()?;
    let loaded = font.load().ok()?;
    Some(loaded.full_name())
}

fn setup(mut commands: Commands, seat: Option<NonSend<SeatState>>, surfaces: Query<&DrmSurface>) {
    if seat.is_none() {
        let camera = Camera2dBundle::default();
        commands.spawn(camera);
    } else {
        surfaces.for_each(|surface| {
            let image_handle = surface.image();
            commands.spawn((Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Image(image_handle),
                    ..default()
                },
                ..default()
            },));
        });
    }
}

fn init_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rect_material_set: ResMut<Assets<RoundedUiRectMaterial>>,
) {
    spawn! {&mut commands=>
    <(ImageBundle{style: style!("absolute full"),
        image: asset_server.load("background.jpg").into(),
        z_index: ZIndex::Global(-1024),
    ..default()})
    Name=(Name::new("background")) /> };

    spawn! { &mut commands=>
    <(MaterialNodeBundle { style: style!("absolute top-4 left-4 right-4 h-32"),
        material: rect_material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5),8.0)),
        z_index: ZIndex::Global(1024),
        ..Default::default()
    }) Name=(Name::new("panel"))>
        <(MaterialNodeBundle { style: style!("absolute flex-row m-4 left-4"),
            material: rect_material_set.add(RoundedUiRectMaterial::new((Color::BLUE*0.6).with_a(0.5),8.0,)),
            ..Default::default()
        }) @id="left">
            <ClockBundle/>
        </>
        <(MaterialNodeBundle { style: style!("absolute flex-row m-4 right-4"),
            material: rect_material_set.add(RoundedUiRectMaterial::new((Color::RED*0.6).with_a(0.5),8.0,)),
            ..Default::default()
        }) @id="right">
            <ClockBundle/>
        </>
        <NodeBundle @style="absolute w-full h-full justify-center items-center" @id="center">
            <(MaterialNodeBundle { style: style!("flex-row m-4"),
                material: rect_material_set.add(RoundedUiRectMaterial::new((Color::WHITE*0.6).with_a(0.5),8.0,)),
                ..Default::default()
            })>
                <ClockBundle/>
            </>
        </NodeBundle>
    </> };

    spawn! {&mut commands=>
    <(NodeBundle{style: style!("absolute bottom-4 w-full justify-center items-center"),
        focus_policy: FocusPolicy::Pass, z_index: ZIndex::Global(1024),..default()})
    Name=(Name::new("dock")) >
        <AppListUIBundle/>
    </NodeBundle> };
}
