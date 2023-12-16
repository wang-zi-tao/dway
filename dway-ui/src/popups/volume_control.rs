use crate::framework::button::{ButtonColor, UiButtonBundle};
use crate::framework::slider::{UiSlider, UiSliderBundle, UiSliderEvent, UiSliderState};
use crate::prelude::MiniNodeBundle;
use crate::prelude::*;
use crate::widgets::popup::{UiPopup, UiPopupAddonBundle};
use dway_client_core::controller::volume::VolumeController;
use dway_ui_derive::dway_widget;

#[derive(Component, Default)]
pub struct VolumeControl;

dway_widget! {
VolumeControl=>
// @bundle{{pub popup: UiPopupAddonBundle = UiPopup::new_auto_destroy(None).into()}}
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
@use_state(volume:f32)
@global(theme: Theme)
@arg(volume_control: NonSend<VolumeController> => {
    let volume = volume_control.volume();
    if *state.volume() != volume {
        dbg!(volume);
        state.set_volume(volume);
    }
})
<MiniNodeBundle @style="h-32 w-256">
    <UiButtonBundle ButtonColor=( ButtonColor::from_theme(&theme, "panel") )/>
    <UiSliderBundle @id="slider" 
        UiSlider=(UiSlider{ callback:Some((this_entity,on_slider_event)), ..default() })
        UiSliderState=(UiSliderState{value: *state.volume(),..default()})/>
</MiniNodeBundle>
}
