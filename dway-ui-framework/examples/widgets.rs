use std::sync::Arc;

use bevy::{prelude::*, ui::RelativeCursorPosition};
use dway_ui_derive::{dway_widget, spawn, style};
use dway_ui_framework::{
    animation::ui::UiAnimationDropdownConfig,
    event::UiEvent,
    input::UiInputEvent,
    prelude::*,
    theme::Theme,
    widgets::{
        button::{UiButtonEvent, UiButtonEventKind},
        combobox::{StringItem, UiComboBox},
        inputbox::UiInputBox,
        popup::{popup_animation_system, UiPopup},
        text::UiTextBundle,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(dway_ui_framework::UiFrameworkPlugin)
        .add_systems(Startup, setup)
        .add_plugins(CounterPlugin)
        .insert_resource(ClearColor(Color::srgb(0.8, 0.8, 0.8)))
        .register_callback(button_open_poppup)
        .register_callback(open_menu)
        .register_callback(popup_animation_system::<UiAnimationDropdownConfig>)
        .run();
}

fn setup(mut commands: Commands, theme: Res<Theme>, callbacks: Res<CallbackTypeRegister>) {
    // Camera so we can see UI
    commands.spawn((Camera2d::default(), Msaa::Sample4));
    let button_open_poppup = callbacks.system(button_open_poppup);
    let open_menu = callbacks.system(open_menu);

    spawn! {&mut commands=>
        <(Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..style!("w-256 h-512 p-8 m-8 flex-col")
            }) BlockStyle=(BlockStyle::Normal) Name=(Name::new("widgets")) >
            <UiButton @style="flex-col w-64 h-32 m-8 align-items:center justify-content:center">
                <( UiTextBundle::new("button", 24, &theme) )/>
            </UiButton>
            <UiCheckBox @style="w-64 h-32 p-4"/>
            <UiSlider @style="w-128 h-32 p-4"/>
            <UiInputBox @style="w-128 h-32 p-4"/>
            <Counter/>
            <UiButton  @style="flex-col p-4 m-4 justify-content:center" @on_event(button_open_poppup->self) >
                <(UiTextBundle::new("open popup", 32, &theme))/>
            </UiButton>
            <Node BlockStyle=(BlockStyle::Hollow)  @style="flex-col p-4 m-4 justify-content:center"
                UiInput @on_event(open_menu->self) >
                <(UiTextBundle::new("open menu", 32, &theme))/>
            </Node>
            <(UiComboBox {
                default_index: None,
                items: vec![
                    Arc::new(StringItem::new("item1".to_string())),
                    Arc::new(StringItem::new("item22".to_string())),
                    Arc::new(StringItem::new("item333".to_string())),
                ],
            }) Name=(Name::new("combobox")) @style="w-128 h-32" />
            <(UiTextBundle::new("text", 32, &theme))/>
        </Node>
    }
    spawn! {&mut commands=>
        <Node>
            <Node BlockStyle=(BlockStyle::Hollow) @style="w-256 h-256 p-8 m-8">
                <UiInputBox @style="full m-8" />
            </Node>
            <Node BlockStyle=(BlockStyle::Sunken) @style="w-256 h-256 p-8 m-8"/>
            <Node @style="w-256 h-256 p-8 m-8"/>
        </Node>
    }
}

pub fn button_open_poppup(
    event: UiEvent<UiButtonEvent>,
    mut commands: Commands,
    theme: Res<Theme>,
    callbacks: Res<CallbackTypeRegister>,
) {
    if event.kind == UiButtonEventKind::Released {
        let callback = callbacks.system(popup_animation_system::<UiAnimationDropdownConfig>);
        commands.entity(event.sender()).with_children(|c| {
            spawn! {c=>
                <Node BlockStyle=(BlockStyle::Normal) GlobalZIndex=(GlobalZIndex(1024)) @style="w-200 h-200 top-120% absolute align-self:center"
                    UiPopup @on_event(callback->event.receiver())
                >
                    <Node BlockStyle=(BlockStyle::Hollow) @style="full">
                        <(UiTextBundle::new("popup inner", 32, &theme))/>
                    </Node>
                </Node>
            }
        });
    }
}

pub fn open_menu(
    event: UiEvent<UiInputEvent>,
    theme: Res<Theme>,
    mut commands: Commands,
    node_query: Query<(&RelativeCursorPosition, &ComputedNode)>,
) {
    if let UiInputEvent::MouseRelease(MouseButton::Left) = &*event {
        let Ok((relative_pos, computed_node)) = node_query.get(event.sender()) else {
            return;
        };
        let Some(normalized) = relative_pos.normalized else {
            return;
        };
        let delta = normalized * computed_node.size();
        commands.entity(event.sender()).with_children(|c| {
            spawn! {c=>
                <Node BlockStyle=(BlockStyle::Normal) @style="absolute flex-col p-8 left-{delta.x} top-{delta.y}"
                    UiPopup=(UiPopup::default().with_auto_destroy())
                >
                    <UiButton @style="m-4 p-4">
                        <(UiTextBundle::new("item 1", 24, &theme))/>
                    </UiButton>
                    <UiButton @style="m-4 p-4">
                        <(UiTextBundle::new("item 2", 24, &theme))/>
                    </UiButton>
                </Node>
            };
        });
    }
}

#[derive(Component, Default)]
pub struct Counter;
dway_widget! {
Counter=>
@global(theme: Theme)

@callback{[UiEvent<UiButtonEvent>]
    fn inc( event: UiEvent<UiButtonEvent>, mut query: Query<&mut CounterState>) {
        let Ok(mut state) = query.get_mut(event.receiver()) else {return};
        if event.kind == UiButtonEventKind::Released{
            *state.count_mut() += 1;
        }
    }
}

@use_state(count: usize)
<Node BlockStyle=(BlockStyle::Hollow) @style="p-8">
    <Node @style="w-64"
        Text=(Text::new(state.count().to_string()))
        TextFont=(theme.text_font(32.0))
        TextColor=(TextColor::BLACK)
    />
    <UiButton ThemeHightlight @style="p-4 w-32 h-32 align-items:center justify-content:center"  @id="button"
        UiWidgetRoot=(this_entity.into()) @on_event(inc)
    >
        <Node Text=(Text::new("+")) TextFont=(theme.text_font(32.0)) TextColor=(TextColor::WHITE)/>
    </UiButton>
</Node>
}
