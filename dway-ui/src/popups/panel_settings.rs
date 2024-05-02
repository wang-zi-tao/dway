use super::volume_control::VolumeControlBundle;
use crate::{panels::PanelButtonBundle, prelude::*};
use dway_client_core::controller::systemcontroller::SystemControllRequest;
use dway_ui_framework::animation::{
    interpolation::EaseFunction,
    ui::{popup_open_close_up, popup_open_drop_down},
};

#[derive(Component, Default)]
pub struct PanelSettings {}

dway_widget! {
PanelSettings=>
@state_reflect()
@plugin{ app.register_system(open_popup).register_system(delay_destroy); }
@callback{[UiButtonEvent]
    fn do_logout( In(event): In<UiButtonEvent>, mut event_writer: EventWriter<SystemControllRequest>) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(SystemControllRequest::Logout);
        }
    }
}
@callback{[UiButtonEvent]
    fn do_reboot( In(event): In<UiButtonEvent>, mut event_writer: EventWriter<SystemControllRequest>) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(SystemControllRequest::Reboot);
        }
    }
}
@callback{[UiButtonEvent]
    fn do_shutdown( In(event): In<UiButtonEvent>, mut event_writer: EventWriter<SystemControllRequest>) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(SystemControllRequest::Shutdown);
        }
    }
}
@global(theme:Theme)
@global(asset_server: AssetServer)
<MiniNodeBundle @style="flex-col">
    <VolumeControlBundle/>
    <MiniNodeBundle @id="bottom_bar" @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup")*0.9, 16.0))
    >
        <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material, &[(this_entity, do_logout)]) ) 
            @style="w-32 h-32" @id="logout_button">
            <(UiSvgBundle::new(theme.icon("logout", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material, &[(this_entity, do_reboot)]) ) 
            @style="w-32 h-32" @id="reboot_button">
            <(UiSvgBundle::new(theme.icon("restart", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material, &[(this_entity, do_shutdown)]) )
            @style="w-32 h-32" @id="poweroff_button">
            <(UiSvgBundle::new(theme.icon("power", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}

pub fn delay_destroy(In(event): In<PopupEvent>, mut commands: Commands, theme: Res<Theme>) {
    if PopupEventKind::Closed == event.kind {
        commands.entity(event.entity).insert(
            Animation::new(Duration::from_secs_f32(0.4), EaseFunction::CubicOut)
                .with_callback(theme.system(popup_open_close_up)),
        );
    }
}

pub fn open_popup(
    In(event): In<UiButtonEvent>,
    theme: Res<Theme>,
    mut commands: Commands,
    mut rect_material_set: ResMut<Assets<RoundedUiRectMaterial>>,
) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                Animation::new(Duration::from_secs_f32(0.5), EaseFunction::CubicIn)
                    .with_callback(theme.system(popup_open_drop_down)),
                rect_material_set.add(rounded_rect(theme.color("panel-popup"), 16.0)),
                PanelSettingsBundle {
                    style: style!("absolute top-120% right-0 align-self:end p-8"),
                    ..default()
                },
            ))
            .insert(UiPopupExt::from(
                UiPopup::default().with_callback(event.receiver, theme.system(delay_destroy)),
            ))
            .set_parent(event.button);
    }
}
