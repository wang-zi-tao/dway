use bevy::{asset::AssetLoader, prelude::*};
use kayak_ui::{
    prelude::*,
    widgets::{ElementBundle, KImage, KImageBundle},
    KayakUIPlugin,
};

#[derive(Default)]
pub struct DWayBackgroundPlugin {}
impl Plugin for DWayBackgroundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(add_background);
    }
}

impl KayakUIPlugin for DWayBackgroundPlugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        // context.add_widget_data::<DWayBackgroundProps, DWayBackgroundStates>();
        // context.add_widget_system(
        //     DWayBackgroundProps::default().get_name(),
        //     widget_update::<DWayBackgroundProps, DWayBackgroundStates>,
        //     render,
        // );
    }
}

#[derive(Component, Clone, PartialEq, Default)]
pub struct DWayBackgroundProps {}
impl Widget for DWayBackgroundProps {}
#[derive(Component, Clone, PartialEq, Default)]
pub struct DWayBackgroundStates {}

#[derive(Bundle)]
pub struct DWayBackgroundBundle {
    pub props: DWayBackgroundProps,
    pub styles: KStyle,
    pub computed_styles: ComputedStyles,
    pub widget_name: WidgetName,
}
impl Default for DWayBackgroundBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: DWayBackgroundProps::default().get_name(),
        }
    }
}
pub fn add_background(mut commands: Commands, assets: ResMut<AssetServer>) {
    let image = assets.load("background.jpg");
    commands.spawn(ImageBundle {
        style: Style {
            position_type:PositionType::Absolute,
            position:UiRect::all(Val::Px(0.0)),
            ..Default::default()
        },
        image_mode: bevy::ui::widget::ImageMode::KeepAspect,
        image: UiImage(image),
        focus_policy: bevy::ui::FocusPolicy::Pass,
        ..Default::default()
    });
}
// pub fn render(
//     In((widget_context, entity)): In<(KayakWidgetContext, Entity)>,
//     mut commands: Commands,
//     query: Query<&DWayBackgroundStates>,
//     assets: ResMut<AssetServer>,
// ) -> bool {
//     let state_entity =
//         widget_context.use_state(&mut commands, entity, DWayBackgroundStates::default());
//     let image = assets.load("background.jpg");
//     if let Ok(status) = query.get(state_entity) {
//         let parent_id = Some(entity);
//
//         rsx! {
//             // <ElementBundle
//             // styles={KStyle {
//             //     layout_type:LayoutType::Column.into(),
//             //     position_type: KPositionType::SelfDirected.into(),
//             //     ..Default::default()
//             // }}
//             // >
//                 <KImageBundle
//                 image={KImage(image)}
//                 styles={KStyle{
//                     left:Units::Pixels(0.0).into(),
//                     right:Units::Pixels(0.0).into(),
//                     top:Units::Pixels(0.0).into(),
//                     bottom:Units::Pixels(0.0).into(),
//                 position_type: KPositionType::SelfDirected.into(),
//                     // z_index: (-1024).into(),
//                     ..Default::default()
//                 }}
//                 />
//             // </ElementBundle>
//         };
//     }
//     true
// }
