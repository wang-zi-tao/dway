use crate::{panels::PanelButtonBundle, prelude::*, widgets::popup::UiPopupAddonBundle};

use self::sharder_ml::{fill::FillColor, shape::Circle, ShaderAsset, ShaderPlugin, ShapeRender};

use super::{
    button::UiButtonBundle,
    checkbox::{RoundedCheckBoxAddonBundle, UiCheckBox, UiCheckBoxState},
    slider::{UiSliderBundle, UiSliderState},
    svg::{UiSvg, UiSvgBundle},
    text::UiTextBundle,
};

#[derive(Component, Default)]
pub struct WidgetGallary;

type CircleButton = ShapeRender<Circle, (FillColor,)>;
fn circle_button_shader() -> CircleButton {
    ShapeRender::new(Circle::new(32.0), (FillColor::new(Color::BLUE),))
}

dway_widget! {
WidgetGallary=>
@plugin{
    app.add_plugins(ShaderPlugin::<CircleButton>::default());
}
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
    <(UiTextBundle::new("shadow material",24,&theme)) @style="w-256 h-24"/>
    <MiniNodeBundle @style="p-4 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("background1"), 16.0))
    >
        <MiniNodeBundle @style="p-4 full justify-content:space-evenly"
            @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(Color::WHITE, 16.0))
        >
            <MiniNodeBundle Style=(Style{
                ..style!("p-4 m-8 w-128 h-64 align-self:center")
            })
                @material(ShadowUiRectMaterial=>ShadowUiRectMaterial::new(
                        Color::WHITE,
                        8.0,
                        // theme.color("shadow"),
                        Color::BLACK.with_a(0.36),
                        Vec2::new(1.0,1.0),
                        Vec2::splat(3.0),
                        6.0))
            >
                <(UiTextBundle::new("text",24,&theme)) @style=""/>
            </MiniNodeBundle>
            <MiniNodeBundle Style=(Style{
                ..style!("p-4 m-8 w-64 h-64 align-self:center")
            }) @material(ShaderAsset<CircleButton>=>circle_button_shader().into()) >
                <(UiTextBundle::new("circle_button",12,&theme)) @style=""/>
            </MiniNodeBundle>
        </MiniNodeBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
