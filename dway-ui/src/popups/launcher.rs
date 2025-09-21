use dway_server::apps::{
    icon::LinuxIcon, launchapp::LaunchAppRequest, DesktopEntriesSet, DesktopEntry,
};
use dway_ui_framework::widgets::scroll::UiScroll;
use widgets::text::UiTextBundle;

use crate::{panels::PanelButtonBundle, prelude::*, widgets::icon::UiIcon};

#[derive(Component, Default)]
pub struct LauncherUI;

dway_widget! {
LauncherUI=>
@callback{[UiEvent<UiButtonEvent>]
    fn on_launch(
        event: UiEvent<UiButtonEvent>,
        mut event_writer: EventWriter<LaunchAppRequest>
    ) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(LaunchAppRequest::new(event.receiver()));
        }
    }
}
@global(theme:Theme)
@global(entries: DesktopEntriesSet)
@global(asset_server: AssetServer)
@plugin{{
    app.register_callback(open_popup);
}}
<Node
@style="flex-col p-4">
    <Node @style="min-h-600 w-full">
        <Node @id="left_bar" @style="w-34% m-4 min-h-600"
            @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
        >
        </Node>
        <Node @id="right_block" @style="m-4 w-full"
            @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
        >
            <UiScroll @style="max-h-600 m-4 w-full" @id="app_list_scroll">
                <Node @style="absolute flex-col w-full" @id="AppList"
                    @for_query(mut entry in Query<Ref<DesktopEntry>>::iter_many(&entries.list)=>[
                        entry=>{
                            state.set_name(entry.name().unwrap_or_default().to_string());
                            if let Some(icon_url) = entry.icon_url(32) {
                                state.set_icon(asset_server.load(icon_url));
                            }
                        }
                    ])>
                    <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material,&[
                        (widget.data_entity,on_launch)
                    ]) ) @style="m-4 p-4"
                        @use_state(pub name: String)
                        @use_state(pub icon: Handle<LinuxIcon>)
                    >
                        <UiIcon @style="w-24 h-24 align-self:center" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                        <(UiTextBundle::new(state.name(),24,&theme)) @id="app_name" @style="p-4 align-self:center"/>
                    </PanelButtonBundle>
                </Node>
            </UiScroll>
        </Node>
    </Node>
    <Node @id="bottom_bar" @style="p-4 min-w-512 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
    >
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="user_icon">
            <(UiSvg::new(theme.icon("user", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="lock_button">
            <(UiSvg::new(theme.icon("lock", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="logout_button">
            <(UiSvg::new(theme.icon("logout", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="reboot_button">
            <(UiSvg::new(theme.icon("restart", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="poweroff_button">
            <(UiSvg::new(theme.icon("power", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </Node>
</Node>
}

pub fn open_popup(event: UiEvent<UiButtonEvent>, theme: Res<Theme>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                UiPopup::default(),
                UiTranslationAnimation::default(),
                AnimationTargetNodeState(style!("absolute top-36 left-0")),
            ))
            .with_children(|c| {
                c.spawn((LauncherUI::default(), style!("h-auto w-auto")));
            })
            .set_parent(event.sender());
    }
}
