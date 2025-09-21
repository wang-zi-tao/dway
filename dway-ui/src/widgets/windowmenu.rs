use widgets::{button::UiButtonEventDispatcher, text::UiTextBundle};

pub use crate::prelude::*;

#[derive(Component, Reflect)]
#[require(UiPopup)]
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
@plugin{
    app.register_callback(open_popup); 
    app.register_type::<WindowMenu>();
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_close_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Close(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_maximize_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Maximize(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_minimize_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Minimize(event.receiver()));
        }
    }
}
@global(theme: Theme)
<Node @style="flex-col">
    <UiButton NoTheme @on_event(on_close_button_event->prop.window_entity) >
        <(UiTextBundle::new("close", 32, &theme))/>
    </UiButton>
    <UiButton NoTheme @on_event(on_maximize_button_event->prop.window_entity) >
        <(UiTextBundle::new("maximize", 32, &theme))/>
    </UiButton>
    <UiButton NoTheme @on_event(on_minimize_button_event->prop.window_entity) >
        <(UiTextBundle::new("minimize", 32, &theme))/>
    </UiButton>
</Node>
}

pub fn open_popup(event: UiEvent<UiButtonEvent>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Released {
        let style = style!("absolute justify-items:center top-36 align-self:end p-8");
        commands
            .spawn((
                UiPopup::default(),
                UiTranslationAnimation::default(),
                AnimationTargetNodeState(style.clone()),
            ))
            .with_children(|c| {
                c.spawn((WindowMenu::default(), style!("h-auto w-auto")));
            })
            .set_parent(event.sender());
    }
}
