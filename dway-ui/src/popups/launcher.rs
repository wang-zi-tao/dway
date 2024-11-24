use dway_server::apps::{
    icon::LinuxIcon, launchapp::LaunchAppRequest, DesktopEntriesSet, DesktopEntry,
};
use dway_ui_framework::animation::translation::UiTranslationAnimationExt;

use crate::{
    panels::PanelButtonBundle,
    prelude::*,
    widgets::icon::{UiIcon, UiIconBundle},
};

#[derive(Component, Default)]
pub struct LauncherUI;

dway_widget! {
LauncherUI=>
@callback{[UiEvent<UiButtonEvent>]
    fn on_launch(
        In(event): In<UiEvent<UiButtonEvent>>,
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
<MiniNodeBundle
@style="flex-col p-4">
    <MiniNodeBundle @style="min-h-600 w-full">
        <MiniNodeBundle @id="left_bar" @style="w-34% m-4 min-h-600"
            @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
        >
        </MiniNodeBundle>
        <MiniNodeBundle @id="right_block" @style="m-4 w-full"
            @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
        >
            <UiScrollBundle @style="max-h-600 m-4 w-full" @id="app_list_scroll">
                <MiniNodeBundle @style="absolute flex-col w-full" @id="AppList"
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
                        <UiIconBundle @style="w-24 h-24 align-self:center" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                        <(UiTextBundle::new(state.name(),24,&theme)) @id="app_name" @style="p-4 align-self:center"/>
                    </PanelButtonBundle>
                </MiniNodeBundle>
            </UiScrollBundle>
        </MiniNodeBundle>
    </MiniNodeBundle>
    <MiniNodeBundle @id="bottom_bar" @style="p-4 min-w-512 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
    >
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="user_icon">
            <(UiSvgBundle::new(theme.icon("user", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="lock_button">
            <(UiSvgBundle::new(theme.icon("lock", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="logout_button">
            <(UiSvgBundle::new(theme.icon("logout", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="reboot_button">
            <(UiSvgBundle::new(theme.icon("restart", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="poweroff_button">
            <(UiSvgBundle::new(theme.icon("power", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}

pub fn open_popup(In(event): In<UiEvent<UiButtonEvent>>, theme: Res<Theme>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                UiPopupBundle::default(),
                UiTranslationAnimationExt {
                    target_style: style!("absolute top-36 left-0").into(),
                    ..Default::default()
                },
            ))
            .with_children(|c| {
                c.spawn(LauncherUIBundle {
                    style: style!("h-auto w-auto"),
                    ..default()
                });
            })
            .set_parent(event.sender());
    }
}
