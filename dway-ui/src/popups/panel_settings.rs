use dway_client_core::controller::systemcontroller::SystemControllRequest;

use super::volume_control::VolumeControlBundle;
use crate::{panels::PanelButtonBundle, prelude::*};

#[derive(Default)]
#[dway_widget_prop]
pub struct PanelSettings {}

dway_widget! {
PanelSettings=>
@state_reflect()
@plugin{ app.register_callback(open_popup); }
@callback{[UiEvent<UiButtonEvent>]
    fn do_logout( event: UiEvent<UiButtonEvent>, mut event_writer: EventWriter<SystemControllRequest>) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(SystemControllRequest::Logout);
        }
    }
}
@callback{[UiEvent<UiButtonEvent>]
    fn do_reboot( event: UiEvent<UiButtonEvent>, mut event_writer: EventWriter<SystemControllRequest>) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(SystemControllRequest::Reboot);
        }
    }
}
@callback{[UiEvent<UiButtonEvent>]
    fn do_shutdown( event: UiEvent<UiButtonEvent>, mut event_writer: EventWriter<SystemControllRequest>) {
        if event.kind == UiButtonEventKind::Released {
            event_writer.send(SystemControllRequest::Shutdown);
        }
    }
}
@global(theme:Theme)
@global(asset_server: AssetServer)
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
<Node @style="flex-col">
    <VolumeControlBundle/>
    <Node @id="bottom_bar" @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
    >
        <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material, &[(this_entity, do_logout)]) )
            @style="w-32 h-32" @id="logout_button">
            <(UiSvg::new(theme.icon("logout", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material, &[(this_entity, do_reboot)]) )
            @style="w-32 h-32" @id="reboot_button">
            <(UiSvg::new(theme.icon("restart", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::with_callback(&theme,&mut assets_rounded_ui_rect_material, &[(this_entity, do_shutdown)]) )
            @style="w-32 h-32" @id="poweroff_button">
            <(UiSvg::new(theme.icon("power", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </Node>
</Node>
}

pub fn open_popup(event: UiEvent<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn((
                UiPopup::default(),
                UiTranslationAnimation::default(),
                AnimationTargetNodeState(style!("absolute top-36 align-self:end p-8 right-0")),
            ))
            .with_children(|c| {
                c.spawn((PanelSettings::default(), style!("h-auto w-auto")));
            })
            .set_parent(event.sender());
    }
}
