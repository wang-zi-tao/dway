use animation::translation::{UiTranslationAnimation, UiTranslationAnimationExt};
use dway_server::apps::{
    icon::LinuxIcon, launchapp::LaunchAppRequest, DesktopEntriesSet, DesktopEntry,
};
use util::DwayUiDirection;

use crate::{
    panels::PanelButtonBundle,
    prelude::*,
    widgets::icon::{UiIcon, UiIconBundle},
};

fn on_launch(
    In(event): In<UiButtonEvent>,
    widget_qeury: Query<(
        &DockLauncherUISubStateAppList,
        &DockLauncherUISubWidgetAppList,
    )>,
    mut popup_query: Query<&mut UiPopup>,
    mut event_writer: EventWriter<LaunchAppRequest>,
) {
    if event.kind == UiButtonEventKind::Released {
        let (state,widget) = 
        widget_qeury.get(event.button).unwrap();
        event_writer.send(LaunchAppRequest::new(widget.data_entity));

        if let Some(mut popup) = state.popup.and_then(|e|popup_query.get_mut(e).ok()){
            popup.request_close();
        }
    }
}

pub fn open_popup(In(event): In<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                UiPopupBundle{
                    style: style!("full"),
                    ..Default::default()
                },
                DockLauncherUI::default(),
                DockLauncherUIState::default(),
                DockLauncherUIWidget::default(),
                UiTranslationAnimationExt {
                    translation: UiTranslationAnimation {
                        direction: DwayUiDirection::BOTTOM,
                        ..Default::default()
                    },
                    target_style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(10.0),
                        right: Val::Percent(10.0),
                        top: Val::Percent(10.0),
                        bottom: Val::Percent(10.0),
                        ..Default::default()
                    }
                    .into(),
                    ..Default::default()
                },
            ))
            .set_parent(event.receiver);
    }
}

#[derive(Component, Default)]
pub struct DockLauncherUI;

dway_widget! {
DockLauncherUI=>
@add_callback{[UiButtonEvent]on_launch}
@global(theme:Theme)
@global(entries: DesktopEntriesSet)
@global(asset_server: AssetServer)
@plugin{{
    app.register_callback(open_popup);
}}
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
<MiniNodeBundle @style="full absolute" >
    <MiniNodeBundle @style="full">
        <UiScrollBundle @style="m-4 w-full" @id="app_list_scroll">
            <MiniNodeBundle @style="absolute flex-row flex_wrap:FlexWrap::Wrap" @id="AppList"
                @for_query(mut entry in Query<Ref<DesktopEntry>>::iter_many(&entries.list)=>[
                    entry=>{
                        state.set_name(entry.name().unwrap_or_default().to_string());
                        if let Some(icon_url) = entry.icon_url(32) {
                            state.set_icon(asset_server.load(icon_url));
                        }
                    }
                ])>
                <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material,&[
                    (this_entity,on_launch)
                ]) ) @style="m-4 w-96 h-96 flex-col items-center justify-center"
                    @use_state(pub popup: Option<Entity><=Some(this_entity))
                    @use_state(pub name: String)
                    @use_state(pub icon: Handle<LinuxIcon>)
                >
                    <UiIconBundle @style="absolute w-64 h-64 align-self:center"
                        UiIcon=(state.icon().clone().into()) @id="app_icon" />
                    <(UiTextBundle::new(state.name(),16,&theme)) @id="app_name"
                        @style="absolute bottom-2 align-self:center"/>
                </PanelButtonBundle>
            </MiniNodeBundle>
        </UiScrollBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
