use dway_client_core::{desktop::FocusedWindow, model::apps::AppListModel};
use dway_server::apps::{
    icon::LinuxIcon, launchapp::LaunchAppRequest, DesktopEntriesSet, DesktopEntry, WindowList,
};
use dway_ui_framework::{
    animation::translation::{UiTranslationAnimation, UiTranslationAnimationExt},
    util::DwayUiDirection,
};
use event::make_callback;
use indexmap::IndexSet;
use widgets::button::{UiButtonEventDispatcher, UiRawButtonBundle};

use super::icon::{UiIcon, UiIconBundle};
use crate::{
    popups::app_window_preview::{AppWindowPreviewPopup, AppWindowPreviewPopupBundle},
    prelude::*,
};

#[derive(Component, Reflect)]
pub struct AppEntryUI(pub Entity);

#[derive(Component, Reflect, Default)]
pub struct AppListUI {}

dway_widget! {
AppListUI=>
@plugin{
    app.register_type::<AppListUIState>();
}
@callback{ [UiEvent<UiButtonEvent>]
fn click_app(
    event: UiEvent<UiButtonEvent>,
    query: Query<(&AppListUISubStateList,&AppListUISubWidgetList)>,
    mut commands: Commands,
    mut launch_event: EventWriter<LaunchAppRequest>,
    key_input: Res<ButtonInput<KeyCode>>,
){
    let Ok((state,widget)) = query.get(event.receiver())else{return;};
    if widget.node_popup_entity == Entity::PLACEHOLDER {return;}
    if event.kind == UiButtonEventKind::Released{
        let ctrl = key_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        if *state.count() > 0 && !ctrl {
            commands.spawn((
                UiPopupBundle::default(),
                UiTranslationAnimationExt {
                    translation: UiTranslationAnimation::new(DwayUiDirection::BOTTOM),
                    target_style: style!("absolute bottom-52 align-self:center").clone().into(),
                    ..Default::default()
                },
            )).with_children(|c|{
                c.spawn(( AppWindowPreviewPopup{app:widget.data_entity}, style!("h-auto w-auto") ));
            }).set_parent(widget.node_popup_entity);
        } else {
            launch_event.send(LaunchAppRequest::new(widget.data_entity));
        }
    }
}}
@global(app_model: AppListModel)
@global(desktop_entrys: DesktopEntriesSet)
@state_component(#[derive(Reflect,serde::Serialize,serde::Deserialize)])
@use_state(app_entitys: Vec<Entity> )
@before_update{
    if !widget.inited || app_model.is_changed() || desktop_entrys.is_changed() {
        let favorite_apps = IndexSet::<Entity>::from_iter(app_model.favorite_apps.iter()
            .filter_map(|appid|desktop_entrys.by_id.get(&**appid).cloned()) );
        state.set_app_entitys(favorite_apps.iter().cloned()
            .chain(desktop_entrys.used_entries.iter().cloned()
                .filter(|entity|!favorite_apps.contains(entity)))
            .collect());
    }
}
@global(assets_server: AssetServer)
<MaterialNodeBundle::<RoundedUiRectMaterial> @id="List"
    @for_query(mut(window_list,entry) in Query<(Option<Ref<WindowList>>,Ref<DesktopEntry>)>::iter_many(state.app_entitys().iter().cloned())=>[
        entry=>{if let Some(icon_url)=entry.icon_url(48){ state.set_icon(assets_server.load(icon_url)); }},

        ]=>{
            if window_list.as_ref().map(|c|c.is_changed()).unwrap_or(false) {
                state.set_count(window_list.map(|l|l.len()).unwrap_or(0));
            }
        }) >
    <NodeBundle @id="app_root"
        @state_component(#[derive(Debug)])
        @use_state(pub count:usize) @use_state(pub icon:Handle<LinuxIcon>) @use_state(pub is_focused:bool)
        @arg(focused_window: ResMut<FocusedWindow> => { state.set_is_focused(focused_window.app_entity == Some(widget.data_entity)); }) >
        <MiniNodeBundle @style="w-48 h-48 m-4 flex-col" @id="app_rect"
            @handle(RoundedUiRectMaterial=>rounded_rect(Color::WHITE.with_alpha(0.4), 10.0)) >
            <UiRawButtonBundle @id="button" @style="absolute full flex-col"
                UiButtonEventDispatcher=(make_callback(node!(app_root), click_app)) >
                <UiIconBundle @id="app_icon" @style="w-full h-full" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                <NodeBundle @id="focus_mark" Node=(Node{
                        width:Val::Percent(((*state.count() as f32)/4.0).min(1.0)*80.0),
                    ..style!("absolute bottom-0 h-2 align-center")})
                    BackgroundColor=((if *state.is_focused() {color!("#0000ff")} else {Color::WHITE} ).into())
                />
            </UiRawButtonBundle>
            <MiniNodeBundle @id="popup" @style="absolute full flex-col" />
        </>
    </NodeBundle>
</>
}
