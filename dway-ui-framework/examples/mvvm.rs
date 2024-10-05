use bevy::{core::FrameCount, prelude::*, utils::tracing::instrument::WithSubscriber};
use dway_ui_derive::style;
use dway_ui_framework::{
    mvvm::{
        container::{ItemCell, ItemCellPlugin},
        list::{ListViewLayout, ListViewModel, ListViewModelPlugin},
        view::{
            list::{ListView, ListViewBundle},
            TextViewFactory,
        },
        viewmodel::{SimpleItemViewModel, SimpleListViewModel, ViewModelPlugin},
        ContainerViewModel,
    },
    prelude::UiHollowBlockBundle,
    theme::Theme,
    widgets::bundles::{MiniNodeBundle, UiBlockBundle},
};

#[derive(Component)]
pub struct UpdateText;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            dway_ui_framework::UiFrameworkPlugin,
            ViewModelPlugin::<String>::default(),
            ListViewModelPlugin::<String>::default(),
            ItemCellPlugin::<String>::default(),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
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
            style: style!("align-items:center justify-content:center p-8"),
            ..Default::default()
        })
        .with_children(|c| {
            c.spawn(UiHollowBlockBundle {
                style: style!("w-256 h-128"),
                ..Default::default()
            })
            .with_children(|c| {
                c.spawn((
                    MiniNodeBundle::default(),
                    TextViewFactory::new(section.clone()),
                    SimpleItemViewModel("text view".to_string()),
                    ItemCell::<String>::default(),
                    UpdateText,
                ));
            });
            c.spawn(UiHollowBlockBundle {
                style: style!("w-256 h-256"),
                ..Default::default()
            })
            .with_children(|c| {
                c.spawn((
                    ListViewBundle::default(),
                    TextViewFactory::new(section),
                    SimpleListViewModel(vec![
                        "text view 1".to_string(),
                        "text view 2".to_string(),
                        "text view 3".to_string(),
                        "text view 4".to_string(),
                        "text view 5".to_string(),
                        "text view 6".to_string(),
                        "text view 7".to_string(),
                        "text view 8".to_string(),
                        "text view 9".to_string(),
                        "text view 10".to_string(),
                    ]),
                    ListViewLayout {
                        item_size: Vec2::new(256.0, 32.0),
                        ..Default::default()
                    },
                ));
            });
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
