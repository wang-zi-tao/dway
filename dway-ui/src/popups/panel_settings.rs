use dway_client_core::controller::systemcontroller::SystemControllRequest;
use dway_ui_framework::animation::{
    interpolation::EaseFunction,
    translation::{UiTranslationAnimationBundle, UiTranslationAnimationExt},
    ui::{popup_open_close_up, popup_open_drop_down},
};

use super::volume_control::VolumeControlBundle;
use crate::{panels::PanelButtonBundle, prelude::*};

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
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
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

pub fn open_popup(In(event): In<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                UiPopupBundle::default(),
                UiTranslationAnimationExt {
                    target_style: style!("absolute top-36 align-self:end p-8 right-0").into(),
                    ..Default::default()
                },
            ))
            .with_children(|c| {
                c.spawn((PanelSettingsBundle {
                    style: style!("h-auto w-auto"),
                    ..default()
                },));
            })
            .set_parent(event.button);
    }
}
