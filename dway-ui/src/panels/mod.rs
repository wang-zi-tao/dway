use dway_client_core::navigation::windowstack::{WindowStack};
use dway_server::xdg::toplevel::DWayToplevel;
use dway_ui_framework::{make_bundle, theme::{ThemeComponent, WidgetKind}};

use crate::prelude::*;

make_bundle!{
    PanelButtonBundle {
        pub button: UiButtonExt,
        pub material: Handle<RoundedUiRectMaterial>,
    }
}

impl PanelButtonBundle {
    pub fn new(
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
    ) -> Self {
        Self {
            style: Style {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            button: UiButtonExt {
                button: UiButton::default(),
                theme: ThemeComponent::widget(WidgetKind::None),
                ..Default::default()
            },
            material: rect_material_set.add(rounded_rect(theme.color("panel"), 8.0)),
            ..Default::default()
        }
    }
    pub fn with_callback(
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
        callback: &[(Entity, SystemId<UiButtonEvent>)],
    ) -> Self {
        Self {
            style: Style {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            button: UiButtonExt {
                button: UiButton::with_callbacks(callback),
                theme: ThemeComponent::widget(WidgetKind::None),
                ..Default::default()
            },
            material: rect_material_set.add(rounded_rect(theme.color("panel"), 8.0)),
            ..Default::default()
        }
    }
}
