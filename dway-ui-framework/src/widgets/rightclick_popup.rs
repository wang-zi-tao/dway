
use crate::{event::UiEvent, prelude::*};

pub trait RgithClickPopupConfig {
    fn on_open(node: Entity, commands: &mut ChildBuilder) {
    }
}

pub fn open_right_click_popup<C: RgithClickPopupConfig>(
    event: UiEvent<UiInputEvent>,
    mut commands: Commands,
) {
    if let UiInputEvent::MouseRelease(MouseButton::Right) = &*event {
        commands.entity(event.sender()).with_children(|c| {
            c.spawn(UiPopupBundle::from(UiPopup::default().with_auto_destroy()))
                .with_children(|c| {
                    C::on_open(event.sender(), c);
                });
        });
    }
}
