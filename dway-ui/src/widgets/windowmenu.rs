use dway_ui_framework::{
    animation::translation::UiTranslationAnimationExt, widgets::button::UiRawButtonBundle,
};

pub use crate::prelude::*;

#[derive(Component)]
pub struct WindowMenu {
    pub window_entity: Entity,
}

impl Default for WindowMenu {
    fn default() -> Self {
        Self {
            window_entity: Entity::PLACEHOLDER,
        }
    }
}

dway_widget! {
WindowMenu=>
@bundle{{ popup: UiPopupBundle }}
@plugin{ app.register_system(open_popup); }
@callback{ [UiButtonEvent]
    fn on_close_button_event(
        In(event): In<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Close(event.receiver));
        }
    }
}
@callback{ [UiButtonEvent]
    fn on_maximize_button_event(
        In(event): In<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Maximize(event.receiver));
        }
    }
}
@callback{ [UiButtonEvent]
    fn on_minimize_button_event(
        In(event): In<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Minimize(event.receiver));
        }
    }
}
@global(theme: Theme)
<MiniNodeBundle @style="flex-col">
    <UiRawButtonBundle UiButton=(UiButton::new(prop.window_entity, on_close_button_event))>
        <(UiTextBundle::new("close", 32, &theme))/>
    </UiRawButtonBundle>
    <UiRawButtonBundle UiButton=(UiButton::new(prop.window_entity, on_maximize_button_event))>
        <(UiTextBundle::new("maximize", 32, &theme))/>
    </UiRawButtonBundle>
    <UiRawButtonBundle UiButton=(UiButton::new(prop.window_entity, on_minimize_button_event))>
        <(UiTextBundle::new("minimize", 32, &theme))/>
    </UiRawButtonBundle>
</MiniNodeBundle>
}

pub fn open_popup(In(event): In<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        let style = style!("absolute justify-items:center top-36 align-self:end p-8");
        commands
            .spawn((
                UiPopupBundle::default(),
                UiTranslationAnimationExt {
                    target_style: style.clone().into(),
                    ..Default::default()
                },
            ))
            .with_children(|c| {
                c.spawn(WindowMenuBundle {
                    style: style!("h-auto w-auto"),
                    ..Default::default()
                });
            })
            .set_parent(event.button);
    }
}
