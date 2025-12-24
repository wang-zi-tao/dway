use bevy::ecs::{lifecycle::HookContext, world::DeferredWorld};
use derive_builder::Builder;
use dway_ui_framework::render::layer_manager::{LayerKind, LayerRenderArea, RenderToLayer};

use crate::prelude::*;

pub mod dock;
pub mod top_panel;

#[derive(Bundle)]
pub struct PanelButtonBundle {
    pub button: UiButton,
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
        }
    }
}

#[derive(Component, Default)]
#[component(on_insert=on_insert_panel_popup)]
pub struct PanelPopup;

pub fn on_insert_panel_popup(mut world: DeferredWorld, context: HookContext) {
    let Some(ahchor) = world.get::<AttachToAnchor>(context.entity).map(|c| c.0) else {
        warn!("PanelPopup missing AttachToAnchor component");
        return;
    };

    let Some(camera_entity) = world
        .get::<ComputedUiTargetCamera>(ahchor)
        .and_then(|c| c.get())
    else {
        warn!("anchor is not a ui node or has no camera");
        return;
    };

    let mut commands = world.commands();
    commands.entity(context.entity).insert((
        RenderToLayer::new(camera_entity, LayerKind::Blur),
        LayerRenderArea,
    ));
}

#[derive(Bundle, Builder)]
#[builder(pattern = "owned")]
pub struct PanelPopupBundle {
    pub prop: PanelPopup,
    pub ui_popup: UiPopup,
    pub translation: UiTranslationAnimation,
    pub animation_target_state: AnimationTargetNodeState,
    pub anchor: AttachToAnchor,
    pub anchor_policy: AnchorPolicy,
}

impl PanelPopupBundle {
    pub fn new(anchor: Entity, style: Node) -> Self {
        Self {
            prop: PanelPopup,
            ui_popup: UiPopup::default(),
            translation: Default::default(),
            animation_target_state: AnimationTargetNodeState(style),
            anchor: AttachToAnchor(anchor),
            anchor_policy: AnchorPolicy {
                vertical_align: PopupAnlign::None,
                horizontal_align: PopupAnlign::None,
            },
        }
    }
}
