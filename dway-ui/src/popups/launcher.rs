use std::process;

use bevy_svg::prelude::Svg;
use dway_server::apps::{
    icon::{LinuxIcon, LinuxIconLoader, LinuxIconKind},
    DesktopEntriesSet, DesktopEntry, launchapp::LaunchAppRequest,
};

use crate::{
    animation,
    framework::{
        animation::despawn_animation,
        button::{UiButtonEvent, UiButtonEventKind},
        icon::UiIcon,
        scroll::UiScrollBundle,
        svg::UiSvgBundle,
        text::UiTextBundle,
    },
    panels::PanelButtonBundle,
    prelude::*,
    widgets::popup::{
        delay_destroy, delay_destroy_up, PopupEvent, PopupEventKind, UiPopup, UiPopupAddonBundle,
    },
};

#[derive(Component, Default)]
pub struct LauncherUI;

pub fn delay_destroy_launcher(In(event): In<PopupEvent>, mut commands: Commands) {
    if PopupEventKind::Closed == event.kind {
        commands.entity(event.entity).insert(despawn_animation(
            animation!(Tween 0.5 secs:BackIn->TransformScaleLens(Vec3::ONE=>Vec3::splat(0.5))),
        ));
    }
}

dway_widget! {
LauncherUI=>
@callback{[UiButtonEvent]
    fn on_launch(
        In(event): In<UiButtonEvent>,
        mut event_writer: EventWriter<LaunchAppRequest>
    ) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(LaunchAppRequest::new(event.receiver));
        }
    }
}
@global(theme:Theme)
@global(entries: DesktopEntriesSet)
@plugin{{
    app.register_system(open_popup);
    app.register_system(delay_destroy_launcher);
}}
@arg(mut svg_assets: ResMut<Assets<Svg>>)
@arg(mut mesh_assets: ResMut<Assets<Mesh>>)
@arg(mut assets_server: ResMut<AssetServer>)
<MiniNodeBundle
@material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup"), 16.0))
@style="flex-col p-4">
    <MiniNodeBundle @style="min-h-600 w-full">
        // <MiniNodeBundle @id="left_bar" @style="w-34% m-4 min-h-600"
        //     @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup")*0.9, 16.0))
        // >
        // </MiniNodeBundle>
        <MiniNodeBundle @id="right_block" @style="m-4 w-full"
            @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup")*0.9, 16.0))
        >
            <UiScrollBundle @style="max-h-600 m-4 w-full" @id="app_list_scroll">
                <MiniNodeBundle @style="absolute flex-col w-full" @id="AppList"
                    @for_query(mut entry in Query<Ref<DesktopEntry>>::iter_mut()=>[
                        entry=>{
                            state.set_name(entry.name().unwrap_or_default().to_string());
                            if let Some(mut icon_url) = entry.icon_url(32) {
                                state.set_icon(assets_server.load(&icon_url));
                            }
                        }
                    ])>
                    <( PanelButtonBundle::with_callback(this_entity,&theme,&mut assets_rounded_ui_rect_material,&[
                        (widget.data_entity,on_launch)
                    ]) ) @style="m-4 p-4"
                        @use_state(pub name: String)
                        @use_state(pub icon: Handle<LinuxIcon>)
                    >
                        <ImageBundle @style="w-24 h-24 align-self:center" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                        <(UiTextBundle::new(&state.name(),24,&theme)) @id="app_name" @style="p-4 align-self:center"/>
                    </PanelButtonBundle>
                </MiniNodeBundle>
            </UiScrollBundle>
        </MiniNodeBundle>
    </MiniNodeBundle>
    <MiniNodeBundle @id="bottom_bar" @style="p-4 min-w-512 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup")*0.9, 16.0))
    >
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="user_icon">
            <(UiSvgBundle::new(theme.icon("user"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="lock_button">
            <(UiSvgBundle::new(theme.icon("lock"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="logout_button">
            <(UiSvgBundle::new(theme.icon("logout"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="reboot_button">
            <(UiSvgBundle::new(theme.icon("restart"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="poweroff_button">
            <(UiSvgBundle::new(theme.icon("power"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}

pub fn open_popup(In(event): In<UiButtonEvent>, theme: Res<Theme>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                animation!(0.5 secs:BackOut->TransformScaleLens(Vec3::splat(0.5)=>Vec3::ONE)),
                LauncherUIBundle {
                    style: style!("absolute top-120% left-0"),
                    ..default()
                },
            ))
            .insert(UiPopupAddonBundle::from(UiPopup::new(Some(
                theme.system(delay_destroy_launcher),
            ))))
            .set_parent(event.button);
    }
}
