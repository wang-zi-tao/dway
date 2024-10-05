use animation::translation::{UiTranslationAnimation, UiTranslationAnimationExt};
use dway_server::apps::{
    icon::LinuxIcon, launchapp::LaunchAppRequest, DesktopEntriesSet, DesktopEntry,
};
use util::DwayUiDirection;
use widgets::inputbox::{UiInputBox, UiInputBoxBundle, UiInputBoxState, UiInputboxEvent, UiInputboxEventKind};

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
        widget_qeury.get(event.receiver).unwrap();
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
                    popup: UiPopupBuilder::default().close_policy(PopupClosePolicy::None).build().unwrap(),
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

fn on_text_changed(In(event):In<UiInputboxEvent>, mut widget_query: Query<&mut DockLauncherUIState>, inputbox_query: Query<&UiInputBoxState>){
    let Ok(mut state) = widget_query.get_mut(event.receiver) else{
        return;
    };
    let Ok(inputbox_state) = inputbox_query.get(event.widget) else{
        return;
    };

    if UiInputboxEventKind::Changed == event.kind{
        state.set_filter(inputbox_state.data.clone());
    }
}

#[derive(Component, Default)]
pub struct DockLauncherUI;

dway_widget! {
DockLauncherUI=>
@add_callback{[UiButtonEvent]on_launch}
@add_callback{[UiInputboxEvent]on_text_changed}
@global(theme:Theme)
@global(entries: DesktopEntriesSet)
@global(asset_server: AssetServer)
@plugin{{
    app.register_callback(open_popup);
}}
@state_reflect()
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
@use_state(pub filter: String)
<MiniNodeBundle @style="full absolute" >
    <MiniNodeBundle @style="full flex-col p-8">
        <UiInputBoxBundle UiInputBox=(UiInputBox::default().with_callback((this_entity, on_text_changed))) 
            @style="left-10% right-10% w-80% height-24"/>
        <UiScrollBundle @style="m-4 w-full h-full" @id="app_list_scroll">
            <MiniNodeBundle @style="absolute flex-row flex_wrap:FlexWrap::Wrap flex_grow:1.0" @id="AppList"
                @for_query(mut entry in Query<Ref<DesktopEntry>>::iter_many(&entries.list)=>[
                    entry=>{
                        state.set_name(entry.name().unwrap_or_default().to_string());
                        if let Some(icon_url) = entry.icon_url(32) {
                            state.set_icon(asset_server.load(icon_url));
                        }
                    }
                ])>
                <MiniNodeBundle @id="app_root" 
                    @use_state(pub popup: Option<Entity><=Some(this_entity))
                    @use_state(pub name: String)
                    @use_state(pub icon: Handle<LinuxIcon>)
                    @if(state.name().contains(root_state.filter()))
                >
                    <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material,&[
                        (node!(app_root),on_launch)
                    ]) )  @style="m-4 w-96 h-96 flex-col items-center justify-center"
                    >
                        <UiIconBundle @style="absolute w-64 h-64 align-self:center"
                            UiIcon=(state.icon().clone().into()) @id="app_icon" />
                        <(UiTextBundle::new(state.name(),16,&theme)) @id="app_name"
                            @style="absolute bottom-2 align-self:center"/>
                    </PanelButtonBundle>
                </MiniNodeBundle>
            </MiniNodeBundle>
        </UiScrollBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
