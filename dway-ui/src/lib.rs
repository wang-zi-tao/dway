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

use bevy::{
    math::FloatOrd,
    render::camera::{ImageRenderTarget, RenderTarget},
    window::WindowRef,
};
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
    render::layer_manager::{LayerKind, LayerManager, RenderToLayer},
    theme::{ThemeComponent, WidgetKind},
};
use widgets::clock::Clock;

use crate::{
    panels::{dock::Dock, top_panel::Panel},
    prelude::*,
    widgets::{
        cursor::Cursor,
        screen::ScreenWindows,
    },
};

pub mod zindex {
    use bevy::ui::GlobalZIndex;

    pub const BACKGROUND: GlobalZIndex = GlobalZIndex(-1024);
    pub const WINDOWS: GlobalZIndex = GlobalZIndex(0);
    pub const PANEL: GlobalZIndex = GlobalZIndex(1024);
    pub const DOCK: GlobalZIndex = GlobalZIndex(1024);
    pub const POPUP: GlobalZIndex = GlobalZIndex(2048);
    pub const CURSOR: GlobalZIndex = GlobalZIndex(8192);
}

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
        app.add_plugins((
            panels::top_panel::PanelPlugin,
            panels::dock::DockPlugin,
        ));
        app.add_observer(init_screen_ui);
        app.add_systems(Startup, setup);
    }
}

fn setup(commands: Commands) {
}

#[derive(Component)]
#[require(Name = Name::from("ScreenUI"))]
pub struct ScreenUI {
    pub screen: Entity,
}

impl ScreenUI {
    pub fn new(screen: Entity) -> Self {
        Self { screen }
    }
}

dway_widget! {
ScreenUI=>
@global(theme: Theme)
@global(callbacks: CallbackTypeRegister)
@global(asset_server: AssetServer)
@world_query(style: &mut Node)
@query(screen_query: (_screen,global_geo)<-Query<(Ref<Screen>,Ref<GlobalGeometry>)>[prop.screen] -> {
    if !widget.inited || global_geo.is_changed() {
        style.position_type = PositionType::Absolute;
        style.width = Val::Px(global_geo.width() as f32);
        style.height = Val::Px(global_geo.height() as f32);
    }
})
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
<Node @id="screen_ui" Name=(Name::new("screen_ui"))
    @style="absolute full">
    <Node Name=(Name::new("background")) @style="absolute full" @id="background">
        <(ImageNode::from(asset_server.load("background.jpg"))) GlobalZIndex=(GlobalZIndex(-1024))/>
    </Node>
    <(ScreenWindows{screen:prop.screen}) @style="absolute full" Name=(Name::new("windows")) @id="windows" />
</Node>
}

fn init_screen_ui(
    trigger: Trigger<OnAdd, Screen>,
    screen_query: Query<(&DrmSurface, &Connector)>,
    window_query: Query<&Window>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let entity = trigger.target();
    let (name, target) = if let Ok((drm_surface, connector)) = screen_query.get(entity) {
        let image_handle = drm_surface.image();
        (
            connector.name().to_string(),
            RenderTarget::Image(ImageRenderTarget {
                handle: image_handle,
                scale_factor: FloatOrd(1.0),
            }),
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
            top: 48,
            ..Default::default()
        },
        ..Default::default()
    });

    commands
        .spawn((
            Name::new("ScreenUI"),
            UiTargetCamera(camera),
            ScreenUI::new(entity),
            style!("absolute full"),
        ))
        .connect_to::<UiAttachData>(entity);

    commands
        .spawn((
            Name::new("panel"),
            Panel::new(entity),
            style!("absolute top-4 left-4 right-4 h-32"),
            RenderToLayer::new(camera, LayerKind::Blur),
            zindex::PANEL,
        ))
        .connect_to::<UiAttachData>(entity);

    commands
        .spawn((
            Name::new("dock"),
            UiTargetCamera(camera),
            Dock::new(entity),
            style!("absolute bottom-4 justify-self:center justify-center items-center"),
            RenderToLayer::new(camera, LayerKind::Blur),
            zindex::DOCK,
        ))
        .connect_to::<UiAttachData>(entity);

    commands
        .spawn((
            Name::new("cursor"),
            UiTargetCamera(camera),
            Cursor::new(asset_server.load("embedded://dway_ui/cursors/cursor-default.png"),Vec2::splat(32.0)),
            RenderToLayer::new(camera, LayerKind::Blur),
            zindex::CURSOR,
        ))
        .connect_to::<UiAttachData>(entity);
}
