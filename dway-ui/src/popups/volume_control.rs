use dway_client_core::controller::volume::VolumeController;
use dway_ui_framework::{
    animation::translation::UiTranslationAnimationExt,
    widgets::checkbox::UiCheckBoxBundle,
};
use event::make_callback;
use widgets::{checkbox::UiCheckBoxEventDispatcher, slider::UiSliderEventDispatcher};

use crate::prelude::*;

#[derive(Component, Default)]
pub struct VolumeControl;

dway_widget! {
VolumeControl=>
@state_reflect()
@callback{[UiEvent<UiSliderEvent>]
    fn on_slider_event(
        event: UiEvent<UiSliderEvent>,
        mut volume_control: NonSendMut<VolumeController>,
    ) {
        if let Err(e) = volume_control.set_volume(event.value) {
            error!("failed to set volume: {e}");
        }
    }
}
@callback{[UiEvent<UiCheckBoxEvent>]
    fn on_mute_event(
        event: UiEvent<UiCheckBoxEvent>,
        mut volume_control: NonSendMut<VolumeController>,
    ) {
        if let Err(e) = volume_control.set_mute(event.value) {
            error!("failed to set mute: {e}");
        }
    }
}
@plugin{ app.register_callback(open_popup); }
@use_state(volume:f32)
@use_state(mute:bool)
@arg(volume_control: NonSend<VolumeController> => {
    let volume = volume_control.volume();
    if *state.volume() != volume {
        state.set_volume(volume);
    }
    let mute = volume_control.is_mute();
    if *state.mute() != mute {
        state.set_mute(mute);
    }
})
@global(asset_server: AssetServer)
<UiCheckBoxBundle UiCheckBoxEventDispatcher=(make_callback(this_entity,on_mute_event))
    @style="p-4 align-self:center" @id="mute_checkbox"
    UiCheckBoxState=(UiCheckBoxState::new(*state.mute()))
>
    <UiSvg @style="w-32 h-32" @id="mute_icon"
        UiSvg=(UiSvg::from( if *state.mute() {
            asset_server.load("embedded://dway_ui/icons/volume_off.svg")
        } else {
            asset_server.load("embedded://dway_ui/icons/volume_on.svg")
        } )) />
</UiCheckBoxBundle>
<UiSliderBundle @id="slider" @style="m-8 h-32 w-256 align-self:center"
    UiSliderEventDispatcher=(make_callback(this_entity,on_slider_event))
    UiSliderState=(UiSliderState{value: *state.volume(),..default()})/>
}

pub fn open_popup(event: UiEvent<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        let style = style!("absolute justify-items:center top-36 align-self:end p-8");
        commands
            .spawn((
                UiPopupBundle::default(),
                UiTranslationAnimationExt {
                    target_style: style.clone().into(),
                    ..Default::default()
                },
            ))
            .with_children(|c| {
                c.spawn(( VolumeControlBundle::default(), style!("h-auto w-auto")  ));
            })
            .set_parent(event.sender());
    }
}
