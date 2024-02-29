use crate::prelude::MiniNodeBundle;
use crate::widgets::popup::{delay_destroy, UiPopup, UiPopupAddonBundle};
use crate::{prelude::*};
use dway_client_core::controller::volume::VolumeController;
use dway_ui_derive::dway_widget;
use dway_ui_framework::widgets::checkbox::UiCheckboxBundle;

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
@plugin{ app.register_system(open_popup); }
@use_state(volume:f32)
@use_state(mute:bool)
@global(theme: Theme)
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
@global(mut rect_material_set: Assets<RoundedUiRectMaterial>)
<UiCheckboxBundle UiCheckBox=(UiCheckBox::new(vec![(this_entity,on_mute_event)]))
    @style="p-4 align-self:center" @id="mute_checkbox"
    UiCheckBoxState=(UiCheckBoxState::new(*state.mute()))
>
    <UiSvgBundle @style="w-32 h-32" @id="mute_icon"
        UiSvg=(UiSvg::from( if *state.mute() {
            asset_server.load("embedded://dway_ui/icons/volume_off.svg")
        } else {
            asset_server.load("embedded://dway_ui/icons/volume_on.svg")
        } )) />
</UiCheckboxBundle>
<UiSliderBundle @id="slider" @style="m-8 h-32 w-256 align-self:center"
    UiSlider=(UiSlider{ callback:Some((this_entity,on_slider_event)), ..default() })
    UiSliderState=(UiSliderState{value: *state.volume(),..default()})/>
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
                // animation!(0.5 secs:BackOut->TransformScaleLens(Vec3::splat(0.5)=>Vec3::ONE)),
                rect_material_set.add(rounded_rect(theme.color("panel-popup"), 16.0)),
                VolumeControlBundle {
                    style: style!("absolute top-120% align-self:end p-8"),
                    ..default()
                },
            ))
            .insert(UiPopupAddonBundle::from(UiPopup::new(Some(
                theme.system(delay_destroy),
            ))))
            .set_parent(event.button);
    }
}
