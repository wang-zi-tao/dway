pub mod assets;
pub mod panels;
pub mod popups;
pub mod prelude;
pub mod sprite;
pub mod util;
pub mod widgets;

pub mod reexport {
    pub use bevy_relationship;
}

use crate::{
    panels::PanelButtonBundle,
    prelude::*,
    widgets::{
        applist::AppListUIBundle, cursor::{Cursor, CursorBundle}, notifys::NotifyButtonBundle, screen::{ScreenWindows, ScreenWindowsBundle}, system_monitor::PanelSystemMonitorBundle, windowtitle::WindowTitleBundle, workspacelist::WorkspaceListUIBundle
    },
};
use bevy::{render::camera::RenderTarget, window::WindowRef};
use bevy_svg::SvgPlugin;
pub use bitflags::bitflags as __bitflags;
use dway_client_core::{controller::systemcontroller::SystemControllRequest, screen::Screen};
use dway_server::geometry::GlobalGeometry;
use dway_tty::{
    drm::{camera::DrmCamera, surface::DrmSurface},
    seat::SeatState,
};
use dway_ui_framework::render::layer_manager::LayerManager;
use widgets::clock::ClockBundle;

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        #[cfg(feature = "css")]
        {
            app.add_plugins(bevy_ecss::EcssPlugin::with_hot_reload());
        }
        app.add_plugins((
            dway_ui_framework::UiFrameworkPlugin,
            assets::DWayAssetsPlugin,
        ));
        app.add_plugins((
            widgets::icon::UiIconPlugin,
            widgets::clock::ClockUiPlugin,
            widgets::window::WindowUIPlugin,
            widgets::popupwindow::PopupUIPlugin,
            widgets::applist::AppListUIPlugin,
            widgets::screen::ScreenWindowsPlugin,
            widgets::workspacelist::WorkspaceListUIPlugin,
            widgets::logger::LoggerUIPlugin,
            widgets::cursor::CursorPlugin,
            widgets::windowtitle::WindowTitlePlugin,
            widgets::system_monitor::PanelSystemMonitorPlugin,
            widgets::notifys::NotifyButtonPlugin,
            ScreenUIPlugin,
        ));
        app.add_plugins((
            popups::app_window_preview::AppWindowPreviewPopupPlugin,
            popups::launcher::LauncherUIPlugin,
            popups::volume_control::VolumeControlPlugin,
            popups::panel_settings::PanelSettingsPlugin,
        ));
        app.add_systems(PreUpdate, init_screen_ui.after(DWayClientSystem::CreateComponent));
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
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
@bundle{{
    name:Name = Name::from("ScreenUI"),
}}
@world_query(style: &mut Style)
@query(screen_query: (_screen,global_geo)<-Query<(Ref<Screen>,Ref<GlobalGeometry>)>[prop.screen] -> {
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
        material: assets!(RoundedUiRectMaterial).add(rounded_rect(Color::WHITE.with_a(0.5),8.0)),
        z_index: ZIndex::Global(1024),
        ..Default::default()
    }) Name=(Name::new("panel")) @id="panel">
        <MiniNodeBundle @style="absolute flex-row m-4 left-4" @id="left">
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (prop.screen,theme.system(popups::launcher::open_popup))
            ])) @style="flex-col">
                <(UiSvgBundle::new(theme.icon("dashboard", &asset_server))) @style="w-24 h-24" @id="dashboard"/>
            </PanelButtonBundle>
            <WindowTitleBundle/>
        </MiniNodeBundle>
        <MiniNodeBundle @style="absolute flex-row right-4 align-items:center" @id="right">
            <ClockBundle/>
            <PanelSystemMonitorBundle @id="system_monitor" @style="h-full"/>
            <NotifyButtonBundle @id="notify"/>
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (prop.screen,theme.system(popups::volume_control::open_popup))
            ])) @style="flex-col m-4">
                // <MiniNodeBundle @style="h-24 w-24" />
                <(UiSvgBundle::new(theme.icon("volume_on", &asset_server))) @style="w-24 h-24" @id="volume"/>
            </PanelButtonBundle>
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (prop.screen,theme.system(popups::panel_settings::open_popup))
            ])) @style="m-4">
                <(UiSvgBundle::new(theme.icon("settings", &asset_server))) @style="w-24 h-24" @id="settings"/>
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
        <MiniNodeBundle @material(RoundedUiRectMaterial=>rounded_rect(Color::WHITE.with_a(0.5), 16.0))>
            <AppListUIBundle/>
            <(PanelButtonBundle::new(&theme,&mut assets!(RoundedUiRectMaterial)))>
                <(UiSvgBundle::new(theme.icon("apps", &asset_server))) @style="w-48 h-48" @id="apps"/>
            </PanelButtonBundle>
        </MiniNodeBundle>
    </NodeBundle>
    <CursorBundle Cursor=(Cursor::new(asset_server.load("embedded://dway_ui/cursors/cursor-default.png"),Vec2::splat(32.0)))/>
    // <LoggerUIBundle @style="bottom-64 left-32 w-80% absolute"/>
</NodeBundle>
}

fn init_screen_ui(
    screen_query: Query<(Entity, Option<&DrmSurface>), Added<Screen>>,
    mut commands: Commands,
    mut camera_count: Local<usize>,
) {
    for (entity, drm_surface) in screen_query.iter() {
        let target = if let Some(drm_surface) = drm_surface {
            let image_handle = drm_surface.image();
            RenderTarget::Image(image_handle)
        } else {
            RenderTarget::Window(WindowRef::Entity(entity))
        };
        let mut camera_cmd = commands.spawn(Camera2dBundle {
            camera: Camera {
                target,
                ..default()
            },
            ..Default::default()
        });
        camera_cmd.add(LayerManager::create);
        let camera = camera_cmd.id();
        if *camera_count == 0 {
            // camera_cmd.insert(IsDefaultUiCamera);
        }
        if let Some(drm_surface) = drm_surface {
            camera_cmd.insert(DrmCamera::new(entity, drm_surface));
        }
        info!("init camera");

        commands.spawn((
            TargetCamera(camera),
            ScreenUIBundle::from(ScreenUI { screen: entity }),
        ));
        *camera_count += 1;
    }
}
