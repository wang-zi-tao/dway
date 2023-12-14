use dway_ui_derive::dway_widget;
use crate::prelude::*;
use crate::prelude::MiniNodeBundle;
use crate::widgets::popup::UiPopupAddonBundle;


#[derive(Component, Default)]
pub struct VolumeControl;

dway_widget!{
VolumeControl=>
// @bundle{{pub popup: UiPopupAddonBundle}}
<MiniNodeBundle>
</MiniNodeBundle>
}
