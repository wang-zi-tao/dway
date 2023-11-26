use std::collections::BTreeMap;

use dway_server::{
    apps::{icon::Icon, WindowList},
    geometry::GlobalGeometry,
    wl::surface::WlSurface,
    xdg::toplevel::DWayToplevel,
};

use crate::{
    framework::{button::UiButtonBundle, svg::UiSvgBundle},
    prelude::*,
    widgets::{
        popup::{PopupState, UiPopup, UiPopupBundle},
        window::create_window_material,
    },
};

#[derive(Event)]
pub struct OpenAppWindowPreviewPopup {
    pub app_entity: Entity,
    pub anchor: Entity,
}

#[derive(Component, Default)]
pub struct AppWindowPreviewPopup;

pub const PREVIEW_HIGHT: f32 = 128.0;

// dway_widget! {
// AppWindowPreviewPopup(
//     app_query: Query<(Ref<WindowList>,)>,
//     window_query: Query<(Entity,&WlSurface,&GlobalGeometry,&DWayToplevel)>,
//     mut events: EventReader<OpenAppWindowPreviewPopup>,
//     mut rect_materials: ResMut<Assets<RoundedUiRectMaterial>>,
//     mut image_materials: ResMut<Assets<RoundedUiImageMaterial>>,
//     asset_server: Res<AssetServer>,
// )
// #[derive(Debug,Reflect)]{
//     pub entity: Entity,
//     pub list: Vec<(Entity,Handle<RoundedUiImageMaterial>,Vec2,String)>,
// }=>
// (_time:Res<Time>, popup: &mut UiPopup){
//     let mut opening = false;
//     for OpenAppWindowPreviewPopup { app_entity, anchor } in events.read(){
//         set_state!(entity = *app_entity);
//         popup.state = PopupState::Open;
//         popup.anchor = Some(*anchor);
//         opening = true;
//     }
// 
//     if popup.state == PopupState::Closed {
//         update_state!(entity = Entity::PLACEHOLDER);
//         state_mut!(list).clear();
//         return;
//     }
// 
//     let app_entity = *state!(entity);
//     if app_entity == Entity::PLACEHOLDER { return; }
// 
//     let Ok((window_list,)) = app_query.get(app_entity) else { return };
//     if !opening && !window_list.is_changed() { return; }
// 
//     let list = state_mut!(list);
//     list.clear();
//     for (entity, wl_surface, geo, toplevel) in window_query.iter_many(window_list.iter()) {
//         let size = geo.size().as_vec2() * PREVIEW_HIGHT / geo.height() as f32;
//         list.push((entity,
//             image_materials.add(create_window_material(wl_surface, geo)),
//             size,
//             toplevel.title.clone().unwrap_or_default()
//         ));
//     }
// }
// <RounndedRectBundle @if(state.entity!=Entity::PLACEHOLDER) @style="flex-row m-4"
//     Handle<_>=(rect_materials.add(RoundedUiRectMaterial::new(Color::WHITE*0.2, 16.0))) >
//     <NodeBundle @for((entity,material,size,title) in state.list.iter()) @id="WindowPreview">
//         <RounndedRectBundle @style="flex-col m-4">
//             <NodeBundle @style="flex-row">
//                 <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="close" @style="m-2 w-20 h-20" >
//                     <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/close.svg").into())) />
//                 </ButtonBundle>
//                 <TextBundle @style="items-center justify-center m-auto"
//                     Text=(Text::from_section(
//                         &*title,
//                         TextStyle {
//                             font_size: 16.0,
//                             color: Color::WHITE,
//                             font: asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf"),
//                         },
//                     ).with_alignment(TextAlignment::Center))
//                 />
//             </NodeBundle>
//             <MaterialNodeBundle::<RoundedUiImageMaterial>
//                Handle<_>=(material.clone())
//                Style=(Style{
//                    width:Val::Px(size.x),
//                    height:Val::Px(size.y),
//                    ..default()
//                }) />
//         </RounndedRectBundle>
//     </NodeBundle>
// </RounndedRectBundle>
// }

pub struct AppWindowPreviewPopupPlugin;
impl Plugin for AppWindowPreviewPopupPlugin {
    fn build(&self, app: &mut App) {
        app.world.spawn((
            Name::new("AppWindowPreviewPopup"),
            UiPopupBundle {
                style: style!("absolute bottom-128"),
                z_index: ZIndex::Global(2048),
                ..Default::default()
            },
            AppWindowPreviewPopup,
            // AppWindowPreviewPopupState {
            //     entity: Entity::PLACEHOLDER,
            //     list: Default::default(),
            // },
            // AppWindowPreviewPopupWidget::default(),
        ));
        app.add_event::<OpenAppWindowPreviewPopup>();
        // app.register_type::<AppWindowPreviewPopupWidget>();
        // app.register_type::<AppWindowPreviewPopupSubWidgetWindowPreview>();
        // app.register_type::<AppWindowPreviewPopupState>();
        // app.add_systems(
        //     Update,
        //     appwindowpreviewpopup_render.in_set(AppWindowPreviewPopupSystems::Render),
        // );
    }
}
