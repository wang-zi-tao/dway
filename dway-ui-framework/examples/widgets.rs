use std::sync::Arc;

use bevy::{prelude::*, ui::RelativeCursorPosition};
use dway_ui_derive::{dway_widget, spawn, style};
use dway_ui_framework::{
    animation::ui::UiAnimationDropdownConfig,
    event::{make_callback, UiEvent},
    input::{UiInputEvent, UiInputExt},
    prelude::*,
    theme::Theme,
    widgets::{
        bundles::{
            MiniNodeBundle, UiBlockBundle, UiHighlightBlockBundle, UiHollowBlockBundle,
            UiSunkenBlockBundle,
        },
        button::{
            UiButtonBundle, UiButtonEvent, UiButtonEventDispatcher, UiButtonEventKind,
            UiHightlightButtonBundle,
        },
        checkbox::UiCheckBoxBundle,
        combobox::{StringItem, UiComboBox, UiComboBoxBundle},
        inputbox::UiInputBoxBundle,
        popup::{popup_animation_system, UiPopup, UiPopupExt},
        slider::UiSliderBundle, text::UiTextBundle,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(dway_ui_framework::UiFrameworkPlugin)
        .add_systems(Startup, setup)
        .add_plugins(CounterPlugin)
        .insert_resource(ClearColor(Color::rgb(0.8, 0.8, 0.8)))
        .register_callback(button_open_poppup)
        .register_callback(open_menu)
        .register_callback(popup_animation_system::<UiAnimationDropdownConfig>)
        .run();
}

fn setup(mut commands: Commands, theme: Res<Theme>, callbacks: Res<CallbackTypeRegister>) {
    // Camera so we can see UI
    commands.spawn((Camera2dBundle::default(), Msaa::Sample4));

    spawn! {&mut commands=>
        <UiBlockBundle Name=(Name::new("widgets"))
            Node=(Node {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..style!("w-256 h-512 p-8 m-8 flex-col")
            })>
            <UiButtonBundle @style="flex-col w-64 h-32 m-8 align-items:center justify-content:center">
                <( UiTextBundle::new("button", 24, &theme) )/>
            </UiButtonBundle>
            <UiCheckBoxBundle @style="w-64 h-32 p-4"/>
            <UiSliderBundle @style="w-128 h-32 p-4"/>
            <UiInputBoxBundle @style="w-128 h-32 p-4"/>
            <CounterBundle/>
            <UiButtonBundle  @style="flex-col p-4 m-4 justify-content:center"
                UiButtonEventDispatcher=( make_callback( Entity::PLACEHOLDER, callbacks.system(button_open_poppup),) )>
                <(UiTextBundle::new("open popup", 32, &theme))/>
            </UiButtonBundle>
            <UiHollowBlockBundle  @style="flex-col p-4 m-4 justify-content:center"
                UiInputExt=(UiInputExt{
                    event_dispatcher: make_callback(Entity::PLACEHOLDER, callbacks.system(open_menu)),
                    ..default()
                })
            >
                <(UiTextBundle::new("open menu", 32, &theme))/>
            </UiHollowBlockBundle>
            <UiComboBoxBundle Name=(Name::new("combobox")) @style="w-128 h-32" UiComboBox=(UiComboBox {
                default_index: None,
                items: vec![
                    Arc::new(StringItem::new("item1".to_string())),
                    Arc::new(StringItem::new("item22".to_string())),
                    Arc::new(StringItem::new("item333".to_string())),
                ],
            })/>
            <(UiTextBundle::new("text", 32, &theme))/>
        </UiBlockBundle>
    }
    spawn! {&mut commands=>
        <MiniNodeBundle>
            <UiHollowBlockBundle @style="w-256 h-256 p-8 m-8">
                <UiInputBoxBundle @style="full m-8" />
            </UiHollowBlockBundle>
            <UiSunkenBlockBundle @style="w-256 h-256 p-8 m-8"/>
            <UiHighlightBlockBundle @style="w-256 h-256 p-8 m-8"/>
        </MiniNodeBundle>
    }
}

pub fn button_open_poppup(
    event: UiEvent<UiButtonEvent>,
    mut commands: Commands,
    theme: Res<Theme>,
    callbacks: Res<CallbackTypeRegister>,
) {
    if event.kind == UiButtonEventKind::Released {
        commands.entity(event.sender()).with_children(|c| {
            spawn! {c=>
                <UiBlockBundle GlobalZIndex=(GlobalZIndex(1024)) @style="w-200 h-200 top-120% absolute align-self:center"
                    UiPopupExt=(UiPopupExt{
                        event_dispatcher: make_callback(event.receiver(),
                            callbacks.system(popup_animation_system::<UiAnimationDropdownConfig>)),
                        ..default()
                    })
                >
                    <(UiTextBundle::new("popup inner", 32, &theme))/>
                </UiBlockBundle>
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
                <UiBlockBundle @style="absolute flex-col p-8 left-{delta.x} top-{delta.y}"
                    UiPopupExt=(UiPopup::default().with_auto_destroy().into())>
                    <UiButtonBundle @style="m-4 p-4">
                        <(UiTextBundle::new("item 1", 24, &theme))/>
                    </UiButtonBundle>
                    <UiButtonBundle @style="m-4 p-4">
                        <(UiTextBundle::new("item 2", 24, &theme))/>
                    </UiButtonBundle>
                </UiBlockBundle>
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
<UiHollowBlockBundle @style="p-8">
    <Node @style="w-64" Text=(Text::new(state.count().to_string())) TextFont=(theme.text_font(32.0))/>
    <UiHightlightButtonBundle @style="p-4 w-32 h-32 align-items:center justify-content:center"  @id="button"
        UiWidgetRoot=(this_entity.into())
        UiButtonEventDispatcher=(make_callback(this_entity, inc))
    >
        <Node Text=(Text::new("+")) TextFont=(theme.text_font(32.0)) TextColor=(TextColor::WHITE)/>
    </UiHightlightButtonBundle>
</UiHollowBlockBundle>
}
