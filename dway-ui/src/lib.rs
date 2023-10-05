#![feature(arc_unwrap_or_clone)]
pub mod background;
pub mod context;
pub mod contexts;
pub mod dock;
pub mod lock;
pub mod overview;
pub mod panel;
pub mod title_bar;
pub mod util;
pub mod widgets;
pub mod windows_area;
pub mod logger;

use std::{ptr::NonNull, sync::Arc};

use dway_client_core::DWayClientSystem;
use dway_udev::{drm::surface::DrmSurface, seat::SeatState};
use failure::Fallible;
pub use kayak_ui;

use bevy::{prelude::*, render::camera::RenderTarget};
use font_kit::{
    error::SelectionError,
    family_name::FamilyName,
    handle::Handle,
    properties::{Properties, Style},
    source::SystemSource,
};
use kayak_ui::{prelude::*, widgets::*};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(KayakContextPlugin);
        app.add_plugin(KayakWidgets);
        app.init_resource::<FontMapping>();
        app.add_startup_system(setup.after(DWayClientSystem::Init));
        app.add_plugin(panel::DWayPanelPlugin::default());
        app.add_plugin(widgets::DWayWidgetsPlugin::default());
        // app.add_plugin(background::DWayBackgroundPlugin::default());
    }
}
pub fn default_system_font() -> Result<Handle, SelectionError> {
    let source = SystemSource::new();
    let default_fonts = &[
        FamilyName::Title("arial".to_string()),
        FamilyName::SansSerif,
        FamilyName::Monospace,
        FamilyName::Fantasy,
    ];
    source.select_best_match(default_fonts, Properties::new().style(Style::Normal))
}

fn setup(
    mut commands: Commands,
    mut font_mapping: ResMut<FontMapping>,
    _camera_query: Query<Entity, With<Camera2d>>,
    _font_resource: ResMut<Assets<Font>>,
    asset_server: Res<AssetServer>,

    seat: Option<NonSend<SeatState>>,
    surfaces: Query<(Entity, &DrmSurface)>,
) {
    info!("setup kayak");
    match default_system_font()
        .map_err(|e| e.into())
        .and_then(|font| font.load().map_err(|e| e.into()))
    {
        Fallible::Ok(font) => {
            let font: font_kit::font::Font = font;
            let font_data_list = font.copy_font_data();
            if let Some(font_data) = font_data_list {
                if let Ok(bevy_font) = Font::try_from_bytes(Arc::unwrap_or_clone(font_data)) {
                    let _e = bevy_font.font;
                    // let kayak_ui_frot=KayakFontLoader.load(font, load_context);
                }
            }
            // font_mapping.set_default();
        }
        Err(e) => {
            error!("failed to load system font :{e}");
            font_mapping.set_default(asset_server.load("roboto.kayak_font"));
        }
    }
    // font_mapping.set_default(asset_server.load("roboto.kayak_font"));
    // let font=asset_server.load("fonts/FiraSans-Bold.ttf");
    font_mapping.set_default(asset_server.load("roboto.kttf"));

    if seat.is_none() {
        info!("create kayak ui camera");
        let camera = Camera2dBundle::default();
        let camera_entity = commands.spawn((camera, CameraUIKayak)).id();
        setup_kayak(camera_entity, &asset_server, &mut commands);
    } else {
        surfaces.for_each(|(entity, surface)| {
            let image_handle = surface.image();
            let camera_entity=commands.spawn((
                Camera2dBundle {
                    camera: Camera {
                        target: RenderTarget::Image(image_handle),
                        ..default()
                    },
                    ..default()
                },
                CameraUIKayak,
            )).id();
            setup_kayak(camera_entity, &asset_server, &mut commands);
            info!(surface=?entity,"create kayak ui camera on drm surface");
        });
    }
}

pub fn setup_kayak(camera_entity: Entity, asset_server: &AssetServer, mut commands: &mut Commands) {
    let mut widget_context = KayakRootContext::new(camera_entity);
    widget_context.add_plugin(KayakWidgetsContextPlugin);
    widget_context.add_plugin(panel::DWayPanelPlugin::default());
    widget_context.add_plugin(widgets::DWayWidgetsPlugin::default());
    widget_context.add_plugin(background::DWayBackgroundPlugin::default());
    widget_context.add_plugin(windows_area::WindowAreaPlugin);
    let _panel_style = KStyle {
        top: StyleProp::Value(Units::Pixels(0.0)),
        height: Units::Percentage(100.0).into(),
        // z_index: 256.into(),
        position_type: KPositionType::SelfDirected.into(),
        ..Default::default()
    };
    let windows_area_style = KStyle {
        // top:StyleProp::Value(Units::Pixels(0.0)),
        // height: Units::Percentage(100.0).into(),
        // z_index: (-1024).into(),
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
        position_type: KPositionType::SelfDirected.into(),
        ..Default::default()
    };
    let background_style = KStyle {
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
        position_type: KPositionType::SelfDirected.into(),
        // z_index: (-256).into(),
        background_color: Color::rgba_u8(0, 0, 0, 0).into(),
        ..Default::default()
    };
    let root_styles = KStyle {
        ..Default::default()
    };
    let parent_id = None;
    // let image = asset_server.load("background.jpg");
    rsx! {
        <KayakAppBundle styles={root_styles} >
            <KImageBundle
            image={KImage(asset_server.load("background.jpg"))}
            styles={background_style.clone()}
            />
            <windows_area::WindowAreaBundle styles={windows_area_style}/>
            <ElementBundle
            styles={KStyle {
                left: StyleProp::Inherit,
                right: StyleProp::Inherit,
                top: StyleProp::Inherit,
                height: StyleProp::Value(Units::Auto),
                position_type: KPositionType::SelfDirected.into(),
                background_color: Color::rgba_u8(0, 0, 0, 0).into(),
                ..Default::default()
            }} >
                <panel::DWayPanelBundle />
            </ElementBundle>
        </KayakAppBundle>

    };
    commands.spawn((widget_context, EventDispatcher::default()));
}
