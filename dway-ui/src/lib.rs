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

use bevy::{render::camera::RenderTarget, window::WindowRef};
use bevy_svg::SvgPlugin;
pub use bitflags::bitflags as __bitflags;
use dway_client_core::{
    layout::{LayoutRect, LayoutStyle},
    screen::Screen,
    UiAttachData,
};
use dway_server::geometry::GlobalGeometry;
use dway_tty::drm::{connectors::Connector, surface::DrmSurface};
use dway_ui_framework::{
    render::layer_manager::LayerManager,
    theme::{ThemeComponent, WidgetKind},
};
use widgets::clock::ClockBundle;

use crate::{
    panels::PanelButtonBundle,
    prelude::*,
    widgets::{
        applist::AppListUIBundle,
        cursor::{Cursor, CursorBundle},
        notifys::NotifyButtonBundle,
        screen::{ScreenWindows, ScreenWindowsBundle},
        system_monitor::PanelSystemMonitorBundle,
        windowtitle::WindowTitleBundle,
        workspacelist::WorkspaceListUIBundle,
    },
};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        app.add_plugins((
            dway_ui_framework::UiFrameworkPlugin,
            assets::DWayAssetsPlugin,
        ));
        app.add_plugins((
            widgets::icon::UiIconPlugin,
            widgets::clock::ClockPlugin,
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
            popups::workspace_window_preview::WorkspaceWindowPreviewPopupPlugin,
            popups::dock_launcher::DockLauncherUIPlugin,
        ));
        app.add_observer(init_screen_ui);
        app.add_systems(Startup, setup);
    }
}

fn setup(commands: Commands) {
}

#[derive(Component, SmartDefault)]
pub struct ScreenUI {
    #[default(Entity::PLACEHOLDER)]
    pub screen: Entity,
}
dway_widget! {
ScreenUI=>
@global(theme: Theme)
@global(callbacks: CallbackTypeRegister)
@global(asset_server: AssetServer)
@bundle{{
    name:Name = Name::from("ScreenUI"),
}}
@world_query(style: &mut Node)
@query(screen_query: (_screen,global_geo)<-Query<(Ref<Screen>,Ref<GlobalGeometry>)>[prop.screen] -> {
    if !widget.inited || global_geo.is_changed() {
        style.position_type = PositionType::Absolute;
        style.width = Val::Px(global_geo.width() as f32);
        style.height = Val::Px(global_geo.height() as f32);
    }
})
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
<NodeBundle @id="screen_ui" Name=(Name::new("screen_ui"))
    // StyleSheet=(StyleSheet::new(asset_server.load("style/style.css")))
    @style="absolute full">
    <MiniNodeBundle Name=(Name::new("background")) @style="absolute full" @id="background">
        <ImageBundle ImageNode=(asset_server.load("background.jpg").into()) GlobalZIndex=(GlobalZIndex(-1024))/>
    </MiniNodeBundle>
    <ScreenWindowsBundle @style="absolute full" Name=(Name::new("windows")) @id="windows"
        ScreenWindows=(ScreenWindows{screen:prop.screen}) />
    <MiniNodeBundle @style="full absolute" @id="popup_parent" GlobalZIndex=(GlobalZIndex(1024)) />
    <MiniNodeBundle
        ThemeComponent=(ThemeComponent::widget(WidgetKind::BlurBackground))
        GlobalZIndex=(GlobalZIndex(1024))
        @style="absolute top-4 left-4 right-4 h-32"
        // @material(RoundedUiRectMaterial=>rounded_rect(Color::WHITE.with_a(0.5),8.0))
        Name=(Name::new("panel")) @id="panel">
        <MiniNodeBundle @style="absolute flex-row m-4 left-4" @id="left">
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (prop.screen,callbacks.system(popups::launcher::open_popup))
            ])) @style="flex-col">
                <(UiSvg::new(theme.icon("dashboard", &asset_server))) @style="w-24 h-24" @id="dashboard"/>
            </PanelButtonBundle>
            <WindowTitleBundle/>
        </MiniNodeBundle>
        <MiniNodeBundle @style="absolute flex-row right-4 align-items:center" @id="right">
            <ClockBundle/>
            <PanelSystemMonitorBundle @id="system_monitor" @style="h-full"/>
            <NotifyButtonBundle @id="notify"/>
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (prop.screen,callbacks.system(popups::volume_control::open_popup))
            ])) @style="flex-col m-4">
                // <MiniNodeBundle @style="h-24 w-24" />
                <(UiSvg::new(theme.icon("volume_on", &asset_server))) @style="w-24 h-24" @id="volume"/>
            </PanelButtonBundle>
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (prop.screen,callbacks.system(popups::panel_settings::open_popup))
            ])) @style="m-4">
                <(UiSvg::new(theme.icon("settings", &asset_server))) @style="w-24 h-24" @id="settings"/>
            </PanelButtonBundle>
        </MiniNodeBundle>
        <MiniNodeBundle @style="absolute w-full h-full justify-center items-center" @id="center">
            <MiniNodeBundle @style="flex-row m-0 h-90%" >
                <WorkspaceListUIBundle @id="workspace_list" />
            </MiniNodeBundle>
        </MiniNodeBundle>
    </>
    <(NodeBundle{node: style!("absolute bottom-4 w-full justify-center items-center"),
        focus_policy: FocusPolicy::Pass, ..default()})
        GlobalZIndex=(GlobalZIndex(1024))
        // Class=(Class::new("dock"))
        Name=(Name::new("dock")) @id="dock" >
        <MiniNodeBundle
            ThemeComponent=(ThemeComponent::widget(WidgetKind::BlurBackground))
            // @material(RoundedUiRectMaterial=>rounded_rect(Color::WHITE.with_a(0.5), 16.0))
            >
            <AppListUIBundle/>
            <(PanelButtonBundle::with_callback(&theme,&mut assets!(RoundedUiRectMaterial), &[
                (node!(popup_parent),callbacks.system(popups::dock_launcher::open_popup))
            ]))>
                <(UiSvg::new(theme.icon("apps", &asset_server))) @style="w-48 h-48" @id="apps"/>
            </PanelButtonBundle>
        </MiniNodeBundle>
    </NodeBundle>
    <CursorBundle Cursor=(Cursor::new(asset_server.load("embedded://dway_ui/cursors/cursor-default.png"),Vec2::splat(32.0)))/>
    // <LoggerUIBundle @style="bottom-64 left-32 w-80% absolute"/>
</NodeBundle>
}

fn init_screen_ui(
    trigger: Trigger<OnAdd, Screen>,
    screen_query: Query<(&DrmSurface, &Connector)>,
    window_query: Query<&Window>,
    mut commands: Commands,
) {
    let entity = trigger.entity();
    let (name, target) = if let Ok((drm_surface, connector)) = screen_query.get(entity) {
        let image_handle = drm_surface.image();
        (
            connector.name().to_string(),
            RenderTarget::Image(image_handle),
        )
    } else if let Ok(window) = window_query.get(entity) {
        (
            window
                .name
                .clone()
                .unwrap_or_else(|| "winit_screen_monitor".to_string()),
            RenderTarget::Window(WindowRef::Entity(entity)),
        )
    } else {
        error!(
            ?entity,
            "the screen entity has neigher a DrmSurface component nor a Window component"
        );
        return;
    };

    let camera = commands
        .spawn((
            Name::new(name),
            Msaa::Sample4,
            Camera2d,
            Camera {
                target: target.clone(),
                ..default()
            },
            LayerManager::default().with_window_target(entity),
        ))
        .id();

    commands.entity(entity).insert(LayoutStyle {
        padding: LayoutRect {
            top: 160,
            ..Default::default()
        },
        ..Default::default()
    });

    commands
        .spawn((
            TargetCamera(camera),
            ScreenUIBundle::from(ScreenUI { screen: entity }),
        ))
        .connect_to::<UiAttachData>(entity);
}
