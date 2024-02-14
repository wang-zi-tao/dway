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
    framework::{gallary::WidgetGallaryBundle, svg::UiSvgBundle},
    panels::{PanelButtonBundle, WindowTitleBundle},
    prelude::*,
    widgets::{
        applist::AppListUIBundle, logger::LoggerUIBundle, popup::UiPopupAddonBundle, screen::{ScreenWindows, ScreenWindowsBundle}, workspacelist::WorkspaceListUIBundle
    },
};
use bevy::{render::camera::RenderTarget, ui::FocusPolicy, window::WindowRef};
use bevy_svg::SvgPlugin;
// use bevy_tweening::TweeningPlugin;
pub use bitflags::bitflags as __bitflags;
use dway_client_core::screen::Screen;
use dway_server::geometry::GlobalGeometry;
use dway_tty::{drm::surface::DrmSurface, seat::SeatState};
use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use widgets::clock::ClockBundle;

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        #[cfg(feature="css")]
        {
            app.add_plugins(bevy_ecss::EcssPlugin::with_hot_reload());
        }
        app.add_plugins((
            // TweeningPlugin,
            assets::DWayAssetsPlugin,
            render::DWayUiMaterialPlugin,
            theme::ThemePlugin,
            framework::UiFrameworkPlugin,
        ));
        app.add_plugins((
            // widgets::clock::ClockUiPlugin,
            // widgets::window::WindowUIPlugin,
            // widgets::popupwindow::PopupUIPlugin,
            // widgets::applist::AppListUIPlugin,
            // widgets::popup::PopupUiPlugin,
            // widgets::screen::ScreenWindowsPlugin,
            // widgets::workspacelist::WorkspaceListUIPlugin,
            // widgets::logger::LoggerUIPlugin,
            // ScreenUIPlugin,
        ));
        app.add_plugins((
            // panels::WindowTitlePlugin,
            // popups::app_window_preview::AppWindowPreviewPopupPlugin,
            // popups::launcher::LauncherUIPlugin,
            // popups::volume_control::VolumeControlPlugin,
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

fn setup(
    mut commands: Commands,
    seat: Option<NonSend<SeatState>>,
    surfaces: Query<(Entity, &DrmSurface)>,
) {
    if seat.is_none() {
        let camera = Camera2dBundle::default();
        commands.spawn(camera);
    } else {
        surfaces.for_each(|(entity, surface)| {
            let image_handle = surface.image();
            commands.spawn((Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Image(image_handle),
                    ..default()
                },
                ..default()
            },));
            commands.spawn((Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Window(WindowRef::Entity(entity)),
                    ..default()
                },
                ..default()
            },));
        });
    }
        spawn! {
            &mut commands=>
            <WidgetGallaryBundle @style="absolute right-32 bottom-32" UiPopupAddonBundle=(Default::default()) />
        }
}

#[derive(Component, SmartDefault)]
pub struct ScreenUI {
    #[default(Entity::PLACEHOLDER)]
    pub screen: Entity,
}
dway_widget! {
ScreenUI=>
@global(theme: Theme)
@global(asset_server: AssetServer)
@global(mut rect_material_set: Assets<RoundedUiRectMaterial>)
@bundle{{
    name:Name = Name::from("ScreenUI"),
}}
@world_query(style: &mut Style)
@query(screen_query: (screen,global_geo)<-Query<(Ref<Screen>,Ref<GlobalGeometry>)>[prop.screen] -> {
    if !widget.inited || global_geo.is_changed() {
        style.position_type = PositionType::Absolute;
        style.width = Val::Px(global_geo.width() as f32);
        style.height = Val::Px(global_geo.height() as f32);
    }
})
<NodeBundle Name=(Name::new("screen_ui"))
    // StyleSheet=(StyleSheet::new(asset_server.load("style/style.css")))
    @style="absolute full">
    <MiniNodeBundle Name=(Name::new("background")) @style="absolute full" @id="background">
        <ImageBundle UiImage=(asset_server.load("background.jpg").into()) ZIndex=(ZIndex::Global(-1024))/>
    </MiniNodeBundle>
    <ScreenWindowsBundle @style="absolute full" Name=(Name::new("windows")) @id="windows"
        ScreenWindows=(ScreenWindows{screen:prop.screen}) />
    <(MaterialNodeBundle { style: style!("absolute top-4 left-4 right-4 h-32"),
        material: rect_material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5),8.0)),
        z_index: ZIndex::Global(1024),
        ..Default::default()
    }) Name=(Name::new("panel")) @id="panel">
        <MiniNodeBundle @style="absolute flex-row m-4 left-4" @id="left">
            <(PanelButtonBundle::with_callback(prop.screen,&theme,&mut rect_material_set, &[
                (prop.screen,theme.system(popups::launcher::open_popup))
            ])) @style="flex-col">
                <(UiSvgBundle::new(theme.icon("dashboard"))) @style="w-24 h-24" @id="dashboard"/>
            </PanelButtonBundle>
            <WindowTitleBundle/>
        </MiniNodeBundle>
        <MiniNodeBundle @style="absolute flex-row m-4 right-4" @id="right">
            <(PanelButtonBundle::new(prop.screen,&theme,&mut rect_material_set))>
                <ClockBundle/>
            </PanelButtonBundle>
            <(PanelButtonBundle::with_callback(prop.screen,&theme,&mut rect_material_set, &[
                (prop.screen,theme.system(popups::volume_control::open_popup))
            ])) @style="flex-col">
                <(UiSvgBundle::new(theme.icon("volume_on"))) @style="w-24 h-24" @id="volume"/>
            </PanelButtonBundle>
            <(PanelButtonBundle::new(prop.screen,&theme,&mut rect_material_set))>
                <(UiSvgBundle::new(theme.icon("settings"))) @style="w-24 h-24" @id="settings"/>
            </PanelButtonBundle>
        </MiniNodeBundle>
        <MiniNodeBundle @style="absolute w-full h-full justify-center items-center" @id="center">
            <MiniNodeBundle @style="flex-row m-0 h-90%" >
                <WorkspaceListUIBundle @id="workspace_list" />
            </MiniNodeBundle>
        </MiniNodeBundle>
    </>
    <(NodeBundle{style: style!("absolute bottom-4 w-full justify-center items-center"),
        focus_policy: FocusPolicy::Pass, z_index: ZIndex::Global(1024),..default()})
        // Class=(Class::new("dock"))
        Name=(Name::new("dock")) @id="dock" >
        <MiniNodeBundle
            Handle<_>=(rect_material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5), 16.0)))>
            <AppListUIBundle/>
            <(PanelButtonBundle::new(prop.screen,&theme,&mut rect_material_set))>
                <(UiSvgBundle::new(theme.icon("apps"))) @style="w-48 h-48" @id="apps"/>
            </PanelButtonBundle>
        </MiniNodeBundle>
    </NodeBundle>
    // <LoggerUIBundle @style="bottom-64 left-32 w-80% absolute"/>
</NodeBundle>
}

fn init_screen_ui(screen_query: Query<Entity, Added<Screen>>, mut commands: Commands) {
    screen_query.for_each(|entity| {
        commands.spawn(ScreenUIBundle::from(ScreenUI { screen: entity }));
    });
}
