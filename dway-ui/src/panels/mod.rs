use crate::prelude::*;

#[derive(Bundle)]
pub struct PanelButtonBundle {
    pub button: UiButton,
    pub event_dispatch: UiButtonEventDispatcher,
    pub node: Node,
    pub material: MaterialNode<RoundedUiRectMaterial>,
}

impl PanelButtonBundle {
    pub fn new(theme: &Theme, rect_material_set: &mut Assets<RoundedUiRectMaterial>) -> Self {
        Self {
            node: Node {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            material: rect_material_set
                .add(rounded_rect(theme.color("panel"), 8.0))
                .into(),
            button: Default::default(),
            event_dispatch: Default::default(),
        }
    }

    pub fn with_callback(
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
        callbacks: &[(Entity, SystemId<UiEvent<UiButtonEvent>>)],
    ) -> Self {
        Self {
            node: Node {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            event_dispatch: EventDispatcher::default().with_systems(callbacks),
            material: rect_material_set
                .add(rounded_rect(theme.color("panel"), 8.0))
                .into(),
            button: Default::default(),
        }
    }
}
