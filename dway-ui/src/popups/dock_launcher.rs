use animation::translation::UiTranslationAnimation;
use dway_server::apps::{
    icon::LinuxIcon, launchapp::LaunchAppRequest, DesktopEntriesSet, DesktopEntry,
};
use dway_ui_framework::widgets::scroll::UiScroll;
use regex::{Regex, RegexBuilder};
use util::DwayUiDirection;
use widgets::{
    inputbox::{UiInputBox, UiInputBoxState, UiInputboxEvent},
    text::UiTextBundle,
};

use crate::{panels::PanelButtonBundle, prelude::*, widgets::icon::UiIcon};

fn on_launch(
    event: UiEvent<UiButtonEvent>,
    widget_qeury: Query<(
        &DockLauncherUISubStateAppList,
        &DockLauncherUISubWidgetAppList,
    )>,
    mut popup_query: Query<&mut UiPopup>,
    mut event_writer: EventWriter<LaunchAppRequest>,
) {
    if event.kind == UiButtonEventKind::Released {
        let (state, widget) = widget_qeury.get(event.receiver()).unwrap();
        event_writer.send(LaunchAppRequest::new(widget.data_entity));

        if let Some(mut popup) = state.popup.and_then(|e| popup_query.get_mut(e).ok()) {
            popup.request_close();
        }
    }
}

pub fn open_popup(event: UiEvent<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                UiPopup::default(),
                style!("full"),
                DockLauncherUI,
                UiTranslationAnimation {
                    direction: DwayUiDirection::BOTTOM,
                    ..Default::default()
                },
                AnimationTargetNodeState(Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(10.0),
                    right: Val::Percent(10.0),
                    top: Val::Percent(10.0),
                    bottom: Val::Percent(10.0),
                    ..Default::default()
                }),
            ))
            .set_parent(event.receiver());
    }
}

fn on_text_changed(
    event: UiEvent<UiInputboxEvent>,
    mut widget_query: Query<&mut DockLauncherUIState>,
    inputbox_query: Query<&UiInputBoxState>,
) {
    let Ok(mut state) = widget_query.get_mut(event.receiver()) else {
        return;
    };
    let Ok(inputbox_state) = inputbox_query.get(event.sender()) else {
        return;
    };

    if matches!(&*event, UiInputboxEvent::Changed) {
        let filter_string = &inputbox_state.data;
        state.set_filter(
            RegexBuilder::new(filter_string)
                .case_insensitive(true)
                .build()
                .unwrap_or_else(|_| {
                    RegexBuilder::new(&regex::escape(filter_string))
                        .case_insensitive(true)
                        .build()
                        .unwrap()
                }),
        );
    }
}

#[derive(Component, Default)]
pub struct DockLauncherUI;

dway_widget! {
DockLauncherUI=>
@add_callback{[UiEvent<UiButtonEvent>]on_launch}
@add_callback{[UiEvent<UiInputboxEvent>]on_text_changed}
@global(theme:Theme)
@global(entries: DesktopEntriesSet)
@global(asset_server: AssetServer)
@plugin{{
    app.register_callback(open_popup);
}}
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
@use_state(pub filter: Regex = Regex::new(".*").unwrap())
<Node @style="full absolute" >
    <Node @style="full flex-col p-8">
        <UiInputBox @on_event(on_text_changed) @style="left-10% right-10% w-80% height-24"/>
        <UiScroll @style="m-4 w-full flex_grow:1.0" @id="app_list_scroll">
            <Node @style="absolute w-full min-h-full flex-row flex_wrap:FlexWrap::Wrap" @id="AppList"
                @for_query(mut entry in Query<Ref<DesktopEntry>>::iter_many(&entries.list)=>[
                    entry=>{
                        state.set_name(entry.name().unwrap_or_default().to_string());
                        if let Some(icon_url) = entry.icon_url(32) {
                            state.set_icon(asset_server.load(icon_url));
                        }
                    }
                ])>
                <Node @id="app_root"
                    @use_state(pub popup: Option<Entity><=Some(this_entity))
                    @use_state(pub name: String)
                    @use_state(pub enable: bool)
                    @use_state(pub icon: Handle<LinuxIcon>)
                    @before({
                        if root_state.filter_is_changed() || state.name_is_changed() {
                            let enable = root_state.filter().is_match(state.name());
                            state.set_enable(enable);
                        }
                    })
                    @if(*state.enable())
                >
                    <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material,&[
                        (node!(app_root),on_launch)
                    ]) )  @style="m-4 w-96 h-96 flex-col items-center justify-center"
                    >
                        <(UiIcon::from(state.icon().clone())) @style="absolute w-64 h-64 align-self:center" @id="app_icon" />
                        <(UiTextBundle::new(state.name(),16,&theme)) @id="app_name"
                            @style="absolute bottom-2 align-self:center"/>
                    </PanelButtonBundle>
                </Node>
            </Node>
        </UiScroll>
    </Node>
</Node>
}
