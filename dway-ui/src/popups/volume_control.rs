use dway_client_core::controller::volume::VolumeController;
use dway_ui_derive::dway_widget;
use dway_ui_framework::{
    animation::{
        interpolation::EaseFunction,
        translation::UiTranslationAnimationExt,
        ui::{popup_open_close_up, popup_open_drop_down},
    },
    widgets::checkbox::UiCheckBoxBundle,
};

use crate::prelude::*;

#[derive(Component, Default)]
pub struct VolumeControl;

dway_widget! {
VolumeControl=>
@state_reflect()
@callback{[UiSliderEvent]
    fn on_slider_event(
        In(event): In<UiSliderEvent>,
        mut volume_control: NonSendMut<VolumeController>,
    ) {
        if let Err(e) = volume_control.set_volume(event.value) {
            error!("failed to set volume: {e}");
        }
    }
}
@callback{[UiCheckBoxEvent]
    fn on_mute_event(
        In(event): In<UiCheckBoxEvent>,
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
<UiCheckBoxBundle UiCheckBox=(UiCheckBox::new(vec![(this_entity,on_mute_event)]))
    @style="p-4 align-self:center" @id="mute_checkbox"
    UiCheckBoxState=(UiCheckBoxState::new(*state.mute()))
>
    <UiSvgBundle @style="w-32 h-32" @id="mute_icon"
        UiSvg=(UiSvg::from( if *state.mute() {
            asset_server.load("embedded://dway_ui/icons/volume_off.svg")
        } else {
            asset_server.load("embedded://dway_ui/icons/volume_on.svg")
        } )) />
</UiCheckBoxBundle>
<UiSliderBundle @id="slider" @style="m-8 h-32 w-256 align-self:center"
    UiSlider=(UiSlider{ callback:Some((this_entity,on_slider_event)), ..default() })
    UiSliderState=(UiSliderState{value: *state.volume(),..default()})/>
}

pub fn open_popup(In(event): In<UiButtonEvent>, mut commands: Commands) {
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
                c.spawn(VolumeControlBundle {
                    style: style!("h-auto w-auto"),
                    ..Default::default()
                });
            })
            .set_parent(event.button);
    }
}
