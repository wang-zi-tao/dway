use bevy_svg::prelude::Svg;
use dway_client_core::desktop::FocusedWindow;
use dway_server::apps::{
    icon::{Icon, IconLoader, IconResorce},
    WindowList,
};
use super::popup::{delay_destroy, UiPopup};
use crate::{
    framework::{
        button::{UiButton, UiButtonAddonBundle, UiButtonBundle, UiButtonEvent, UiButtonEventKind},
        icon::UiIcon,
    },
    popups::app_window_preview::{AppWindowPreviewPopup, AppWindowPreviewPopupBundle},
    prelude::*,
    theme::Theme,
    widgets::popup::UiPopupAddonBundle,
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
@callback{ [UiButtonEvent]
fn open_popup(
    In(event): In<UiButtonEvent>,
    prop_query: Query<&AppListUISubWidgetList>,
    theme: Res<Theme>,
    mut commands: Commands,
){
    let Ok(widget) = prop_query.get(event.receiver)else{return;};
    if widget.node_popup_entity == Entity::PLACEHOLDER {return;}
    if event.kind == UiButtonEventKind::Pressed{
        commands.spawn(AppWindowPreviewPopupBundle{
            prop:AppWindowPreviewPopup{app:widget.data_entity},
            popup: UiPopupAddonBundle{
                popup: UiPopup{
                    callback: Some(theme.system(delay_destroy)),
                    ..default()
                },
                ..default()
            },
            style: style!("absolute bottom-110% align-self:center"),
            ..default()
        }).set_parent(widget.node_popup_entity);
    }
}}
@state_component(#[derive(Reflect,serde::Serialize,serde::Deserialize)])
@arg(mut icon_loader: ResMut<IconLoader>)
@arg(mut svg_assets: ResMut<Assets<Svg>>)
@arg(mut mesh_assets: ResMut<Assets<Mesh>>)
@arg(mut icon_loader: ResMut<IconLoader>)
@arg(mut assets_server: ResMut<AssetServer>)
<MaterialNodeBundle::<RoundedUiRectMaterial> @id="List"
    @for_query(mut(window_list,mut icon) in Query<(Ref<WindowList>,&mut Icon)>::iter_mut()=>[
        icon=>{state.set_icon(icon_loader.load(&mut icon, 48, &mut assets_server, &mut svg_assets, &mut mesh_assets).unwrap_or_default());},
        window_list=>{ state.set_count(window_list.len()); },
    ]) >
    <NodeBundle @id="app_root"
        @state_component(#[derive(Debug)])
        @use_state(pub count:usize) @use_state(pub icon:IconResorce) @use_state(pub is_focused:bool)
        @arg(focused_window: ResMut<FocusedWindow> => { state.set_is_focused(focused_window.app_entity == Some(widget.data_entity)); }) >
        <MiniNodeBundle @if(*state.count()>0)  >
            <RounndedRectBundle @style="w-48 h-48 m-4 flex-col" @id="app_rect"
                @handle(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(Color::WHITE.with_a(0.4), 10.0)) >
                <UiButtonBundle @id="button" @style="absolute full flex-col"
                    UiButtonAddonBundle=(UiButton::new(node!(app_root), open_popup).into()) >
                    <ImageBundle @style="w-full h-full" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                    <NodeBundle @id="focus_mark" Style=(Style{
                            width:Val::Percent(((*state.count() as f32)/4.0).min(1.0)*80.0),
                        ..style!("absolute bottom-0 h-2 align-center")})
                        BackgroundColor=((if *state.is_focused() {Color::BLUE} else {Color::WHITE} ).into())
                    />
                </UiButtonBundle>
                <MiniNodeBundle @id="popup" @style="absolute full flex-col" />
            </>
        </MiniNodeBundle>
    </NodeBundle>
</>
}

