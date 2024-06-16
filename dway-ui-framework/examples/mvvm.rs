use bevy::core::FrameCount;
use bevy::prelude::*;
use dway_ui_derive::style;
use dway_ui_framework::mvvm::container::{ItemCell, ItemCellPlugin};
use dway_ui_framework::mvvm::list::ListViewModel;
use dway_ui_framework::mvvm::view::TextViewFactory;
use dway_ui_framework::mvvm::viewmode::{SimpleItemViewModel, ViewModelPlugin};
use dway_ui_framework::mvvm::ContainerViewModel;
use dway_ui_framework::theme::Theme;
use dway_ui_framework::widgets::bundles::{MiniNodeBundle, UiBlockBundle};

#[derive(Component)]
pub struct UpdateText;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            dway_ui_framework::UiFrameworkPlugin,
            ViewModelPlugin::<String>::default(),
            ItemCellPlugin::<String>::default(),
            // bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
        ))
        .add_systems(Update, update)
        .add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands, theme: Res<Theme>) {
    commands.spawn(Camera2dBundle::default());
    let section = TextStyle {
        font: theme.default_font(),
        font_size: 32.0,
        color: Color::BLACK,
    };
    commands
        .spawn(UiBlockBundle {
            style: style!("absolute w-256 h-64 align-items:center justify-content:center p-8"),
            ..Default::default()
        })
        .with_children(|c| {
            c.spawn((
                MiniNodeBundle::default(),
                TextViewFactory::new(section),
                SimpleItemViewModel("text view".to_string()),
                ItemCell::<String>::default(),
                UpdateText,
            ));
        });
}

fn update(
    mut query: Query<&mut SimpleItemViewModel<String>, With<UpdateText>>,
    frame: Res<FrameCount>,
) {
    for mut model in &mut query {
        model.0 = frame.0.to_string();
    }
}
