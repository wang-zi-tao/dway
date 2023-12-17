use crate::{panels::PanelButtonBundle, prelude::*, widgets::popup::UiPopupAddonBundle};

use super::{
    button::UiButtonBundle,
    slider::{UiSliderBundle, UiSliderState},
    svg::{UiSvgBundle, UiSvg},
    text::UiTextBundle, checkbox::{RoundedCheckBoxAddonBundle, UiCheckBox, UiCheckBoxState},
};

#[derive(Component, Default)]
pub struct WidgetGallary;

dway_widget! {
WidgetGallary=>
@bundle{{pub popup: UiPopupAddonBundle}}
@global(theme: Theme)
@global(asset_server: AssetServer)
<MiniNodeBundle @style="full flex-col min-w-512 min-h-512 p-8 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background"), 16.0))
    >
    <(UiTextBundle::new("text",24,&theme)) @style="w-256 h-24"/>
    <MiniNodeBundle @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background1"), 16.0))
    >
        <(UiTextBundle::new("text",24,&theme))/>
        <(UiTextBundle::new("text",24,&theme)) @style="p-4"
            @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background2"), 16.0)) />
        <(UiTextBundle::new("text",24,&theme)) @style="p-4"
            BackgroundColor=(theme.color("background2").into()) />
    </MiniNodeBundle>
    <(UiTextBundle::new("buttons",24,&theme)) @style="w-256 h-24"/>
    <MiniNodeBundle @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background1"), 16.0))
    >
        <UiButtonBundle>
            <(UiTextBundle::new("button",24,&theme))/>
        </UiButtonBundle>
        <UiButtonBundle @style="m-4"
            @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background2"), 16.0)) >
            <(UiTextBundle::new("button",24,&theme))/>
        </UiButtonBundle>
        <(PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material)) @style="m-4">
            <(UiSvgBundle::new(theme.icon("settings"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <(PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material)) @style="m-4">
            <(UiTextBundle::new("button",24,&theme))/>
        </PanelButtonBundle>
    </MiniNodeBundle>
    <(UiTextBundle::new("slider",24,&theme)) @style="w-256 h-24"/>
    <MiniNodeBundle @style="p-4 justify-content:space-evenly flex-col"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background1"), 16.0))
    >
        <UiSliderBundle @style="h-16 m-16"/>
        <UiSliderBundle @style="h-16 m-16"
            UiSliderState=({let mut s=UiSliderState::default();s.set_value(0.5); s}) />
    </MiniNodeBundle>
    <(UiTextBundle::new("checkbox",24,&theme)) @style="w-256 h-24"/>
    <MiniNodeBundle @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background1"), 16.0))
    >
        <MiniNodeBundle @style="p-4 align-self:center"
            RoundedCheckBoxAddonBundle=(RoundedCheckBoxAddonBundle::new(UiCheckBox::default(),&mut assets_rounded_ui_rect_material,&theme,"panel",this_entity)) >
            <UiSvgBundle @style="w-32 h-32" UiSvg=(asset_server.load("embedded://dway_ui/icons/volume_off.svg").into()) />
        </MiniNodeBundle>
        <MiniNodeBundle @style="p-4 align-self:center"
            RoundedCheckBoxAddonBundle=(RoundedCheckBoxAddonBundle::new(UiCheckBox::default(),&mut assets_rounded_ui_rect_material,&theme,"panel",this_entity)) >
            <(UiTextBundle::new("text",24,&theme))/>
        </MiniNodeBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
