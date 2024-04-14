use super::icon::{UiIcon, UiIconBundle};
use crate::popups::app_window_preview::{AppWindowPreviewPopup, AppWindowPreviewPopupBundle};
use crate::prelude::*;
use dway_client_core::desktop::FocusedWindow;
use dway_server::apps::{icon::LinuxIcon, DesktopEntry, WindowList};
use dway_ui_framework::widgets::button::UiRawButtonExt;

#[derive(Component, Reflect)]
pub struct AppEntryUI(pub Entity);

#[derive(Component, Reflect, Default)]
pub struct AppListUI {}

dway_widget! {
AppListUI=>
@plugin{
    app.register_type::<AppListUIState>();
}
@callback{ [UiButtonEvent]
fn open_popup(
    In(event): In<UiButtonEvent>,
    prop_query: Query<&AppListUISubWidgetList>,
    theme: Res<Theme>,
    mut commands: Commands,
){
    let Ok(widget) = prop_query.get(event.receiver)else{return;};
    if widget.node_popup_entity == Entity::PLACEHOLDER {return;}
    if event.kind == UiButtonEventKind::Released{
        commands.spawn(AppWindowPreviewPopupBundle{
            prop:AppWindowPreviewPopup{app:widget.data_entity},
            style: style!("absolute bottom-110% align-self:center"),
            ..default()
        })
        .insert(UiPopupExt::from( UiPopup::default().with_callback(event.receiver, theme.system(delay_destroy))))
        .set_parent(widget.node_popup_entity);
    }
}}
@state_component(#[derive(Reflect,serde::Serialize,serde::Deserialize)])
@arg(assets_server: ResMut<AssetServer>)
<MaterialNodeBundle::<RoundedUiRectMaterial> @id="List"
    @for_query(mut(window_list,entry) in Query<(Ref<WindowList>,Ref<DesktopEntry>)>::iter_mut()=>[
        entry=>{if let Some(icon_url)=entry.icon_url(48){ state.set_icon(assets_server.load(icon_url)); }},
        window_list=>{ state.set_count(window_list.len()); },
    ]) >
    <NodeBundle @id="app_root"
        @state_component(#[derive(Debug)])
        @use_state(pub count:usize) @use_state(pub icon:Handle<LinuxIcon>) @use_state(pub is_focused:bool)
        @arg(focused_window: ResMut<FocusedWindow> => { state.set_is_focused(focused_window.app_entity == Some(widget.data_entity)); }) >
        <MiniNodeBundle @if(*state.count()>0)  >
            <MiniNodeBundle @style="w-48 h-48 m-4 flex-col" @id="app_rect"
                @handle(RoundedUiRectMaterial=>rounded_rect(Color::WHITE.with_a(0.4), 10.0)) >
                <MiniNodeBundle @id="button" @style="absolute full flex-col"
                    UiRawButtonExt=(UiButton::new(node!(app_root), open_popup).into()) >
                    <UiIconBundle @id="app_icon" @style="w-full h-full" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                    <NodeBundle @id="focus_mark" Style=(Style{
                            width:Val::Percent(((*state.count() as f32)/4.0).min(1.0)*80.0),
                        ..style!("absolute bottom-0 h-2 align-center")})
                        BackgroundColor=((if *state.is_focused() {Color::BLUE} else {Color::WHITE} ).into())
                    />
                </MiniNodeBundle>
                <MiniNodeBundle @id="popup" @style="absolute full flex-col" />
            </>
        </MiniNodeBundle>
    </NodeBundle>
</>
}
