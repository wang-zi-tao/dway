use bevy::ecs::system::EntityCommands;

use crate::{event::UiEvent, prelude::*};

pub trait RgithClickPopupConfig {
    fn on_open(node: Entity, mut commands: &mut ChildBuilder) {
    }
}

pub fn open_right_click_popup<C: RgithClickPopupConfig>(
    In(event): In<UiEvent<UiInputEvent>>,
    mut commands: Commands,
) {
    match &*event {
        UiInputEvent::MouseRelease(MouseButton::Right) => {
            commands.entity(event.sender()).with_children(|c| {
                c.spawn(UiPopupBundle::from(UiPopup::default().with_auto_destroy()))
                    .with_children(|mut c| {
                        C::on_open(event.sender(), &mut c);
                    });
            });
        }
        _ => {}
    }
}
