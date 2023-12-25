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

pub mod reexport {
    pub use bevy_relationship;
}

use crate::{
    framework::{
        button::{ButtonColor, RoundedButtonAddonBundle, UiButton},
        slider::UiSliderBundle,
        svg::UiSvgBundle,
    },
    panels::{PanelButtonBundle, WindowTitleBundle},
    popups::volume_control::VolumeControlBundle,
    prelude::*,
    widgets::{
        applist::AppListUIBundle,
        screen::{ScreenWindows, ScreenWindowsBundle},
    },
};
use bevy::{render::camera::RenderTarget, ui::FocusPolicy};
use bevy_ecss::{EcssPlugin, StyleSheet, Class};
use bevy_svg::SvgPlugin;
use bevy_tweening::TweeningPlugin;
pub use bitflags::bitflags as __bitflags;
use dway_client_core::screen::Screen;
use dway_tty::{drm::surface::DrmSurface, seat::SeatState};
use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use widgets::clock::ClockBundle;

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        app.add_plugins(EcssPlugin::with_hot_reload());
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
            widgets::screen::ScreenWindowsPlugin,
            panels::WindowTitlePlugin,
            popups::app_window_preview::AppWindowPreviewPopupPlugin,
            popups::launcher::LauncherUIPlugin,
            popups::volume_control::VolumeControlPlugin,
        ));
        app.add_systems(PreUpdate, init_screen_ui);
        app.add_systems(Startup, setup);
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

fn init_screen_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rect_material_set: ResMut<Assets<RoundedUiRectMaterial>>,
    mut screen_query: Query<(Entity, &Screen), Added<Screen>>,
    theme: Res<Theme>,
) {
    screen_query.for_each(|(entity,screen)|{
        spawn! {&mut commands=>
        <NodeBundle Name=(Name::new("screen_ui"))
            StyleSheet=(StyleSheet::new(asset_server.load("style/style.css")))
            @style="absolute full">
            <MiniNodeBundle Name=(Name::new("background")) @style="absolute full">
                <ImageBundle UiImage=(asset_server.load("background.jpg").into()) ZIndex=(ZIndex::Global(-1024))/>
            </MiniNodeBundle> 
            <ScreenWindowsBundle @style="absolute full" Name=(Name::new("windows"))
                ScreenWindows=(ScreenWindows{screen:entity}) />
            <(MaterialNodeBundle { style: style!("absolute top-4 left-4 right-4 h-32"),
                material: rect_material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5),8.0)),
                z_index: ZIndex::Global(1024),
                ..Default::default()
            }) Name=(Name::new("panel"))>
                <MiniNodeBundle @style="absolute flex-row m-4 left-4" @id="left">
                    <(PanelButtonBundle::with_callback(entity,&theme,&mut rect_material_set, &[
                        (entity,theme.system(popups::launcher::open_popup))
                    ])) @style="flex-col">
                        <(UiSvgBundle::new(theme.icon("dashboard"))) @style="w-24 h-24"/>
                    </PanelButtonBundle>
                    <WindowTitleBundle/>
                </MiniNodeBundle>
                <MiniNodeBundle @style="absolute flex-row m-4 right-4" @id="right">
                    <(PanelButtonBundle::with_callback(entity,&theme,&mut rect_material_set, &[
                        (entity,theme.system(popups::volume_control::open_popup))
                    ])) @style="flex-col">
                        <(UiSvgBundle::new(theme.icon("volume_on"))) @style="w-24 h-24"/>
                    </PanelButtonBundle>
                    <(PanelButtonBundle::new(entity,&theme,&mut rect_material_set))>
                        <(UiSvgBundle::new(theme.icon("settings"))) @style="w-24 h-24"/>
                    </PanelButtonBundle>
                </MiniNodeBundle>
                <MiniNodeBundle @style="absolute w-full h-full justify-center items-center" @id="center">
                    <MiniNodeBundle @style="flex-row m-0 h-90%" >
                        <(PanelButtonBundle::new(entity,&theme,&mut rect_material_set))>
                            <ClockBundle/>
                        </PanelButtonBundle>
                    </MiniNodeBundle>
                </MiniNodeBundle>
            </> 
            <(NodeBundle{style: style!("absolute bottom-4 w-full justify-center items-center"),
                focus_policy: FocusPolicy::Pass, z_index: ZIndex::Global(1024),..default()})
                Name=(Name::new("dock")) Class=(Class::new("dock")) >
                <MiniNodeBundle 
                    Handle<_>=(rect_material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5), 16.0)))>
                    <AppListUIBundle/>
                    <(PanelButtonBundle::new(entity,&theme,&mut rect_material_set))>
                        <(UiSvgBundle::new(theme.icon("apps"))) @style="w-48 h-48"/>
                    </PanelButtonBundle>
                </MiniNodeBundle>
            </NodeBundle>
        </NodeBundle>
        };
    });
}
