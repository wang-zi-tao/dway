use dway_client_core::controller::systemcontroller::SystemControllRequest;
use dway_ui_framework::render::layer_manager::{LayerKind, LayerRenderArea, RenderToLayer};

use super::volume_control::VolumeControl;
use crate::{
    panels::{PanelButtonBundle, PanelPopupBundle},
    prelude::*,
};

#[derive(Component, Default)]
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
    <VolumeControl/>
    <Node @id="bottom_bar" @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 16.0))
    >
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) )
            @on_event(do_logout->this_entity)
            @style="w-32 h-32" @id="logout_button">
            <(UiSvg::new(theme.icon("logout", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) )
            @on_event(do_reboot->this_entity)
            @style="w-32 h-32" @id="reboot_button">
            <(UiSvg::new(theme.icon("restart", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(&theme,&mut assets_rounded_ui_rect_material) )
            @on_event(do_shutdown->this_entity)
            @style="w-32 h-32" @id="poweroff_button">
            <(UiSvg::new(theme.icon("power", &asset_server))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </Node>
</Node>
}

pub fn open_popup(event: UiEvent<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        commands
            .spawn(PanelPopupBundle {
                anchor_policy: AnchorPolicy::new(PopupAnlign::InnerEnd, PopupAnlign::None),
                ..PanelPopupBundle::new(
                    event.receiver(),
                    style!("absolute top-42"),
                )
            })
            .with_children(|c| {
                c.spawn((PanelSettings::default(), style!("h-auto w-auto")));
            });
    }
}
