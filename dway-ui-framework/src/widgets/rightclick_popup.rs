use crate::prelude::*;
use bevy::ecs::system::{Command, EntityCommands};

pub trait RgithClickPopupConfig {
    fn on_open(node: Entity, mut commands: &mut ChildBuilder) {}
}

pub fn open_right_click_popup<C: RgithClickPopupConfig>(
    In(event): In<UiInputEvent>,
    mut commands: Commands,
) {
    match event.event {
        UiInputEventKind::MouseRelease(MouseButton::Right) => {
            commands.entity(event.node).with_children(|c| {
                c.spawn(UiPopupBundle::from(UiPopup::new_auto_destroy(None))).with_children(|mut c|{
                C::on_open(event.node, &mut c);
                });
            });
        }
        _ => {}
    }
}
