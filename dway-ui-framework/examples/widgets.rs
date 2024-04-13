use std::thread::spawn;

use bevy::{prelude::*, ui::RelativeCursorPosition};
use dway_ui_derive::{dway_widget, spawn, style};
use dway_ui_framework::{
    animation::ui::UiAnimationDropdownConfig,
    input::{UiInput, UiInputEvent, UiInputEventKind, UiInputExt},
    theme::{self, Theme, ThemeAppExt},
    widgets::{
        bundles::{
            MiniNodeBundle, UiBlockBundle, UiHighlightBlockBundle, UiHollowBlockBundle,
            UiNodeBundle, UiSunkenBlockBundle,
        },
        button::{
            UiButton, UiButtonBundle, UiButtonEvent, UiButtonEventKind, UiHightlightButtonBundle,
        },
        checkbox::UiCheckBoxBundle,
        inputbox::UiInputBoxBundle,
        popup::{popup_animation_system, UiPopup, UiPopupExt},
        rightclick_popup::RgithClickPopupConfig,
        slider::UiSliderBundle,
        text::{UiTextBundle, UiTextExt},
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((dway_ui_framework::UiFrameworkPlugin,))
        .add_systems(Startup, setup)
        .add_plugins(CounterPlugin)
        .insert_resource(ClearColor(Color::WHITE * 0.8))
        .insert_resource(Msaa::Sample4)
        .register_system(button_open_poppup)
        .register_system(open_menu)
        .register_system(popup_animation_system::<UiAnimationDropdownConfig>)
        .run();
}

fn setup(mut commands: Commands, theme: Res<Theme>) {
    // Camera so we can see UI
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(UiNodeBundle {
            style: Style {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..style!("p-8")
            },
            ..default()
        })
        .with_children(|c| {
            c.spawn(( UiBlockBundle {
                style: Style {
                    align_self: AlignSelf::Center,
                    justify_self: JustifySelf::Center,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..style!("w-256 h-512 p-8 m-8")
                },
                ..default()
            }, ))
            .with_children(|c| {
                c.spawn(UiButtonBundle {
                    style: style!("w-64 h-32 m-8 align-items:center justify-content:center"),
                    ..Default::default()
                })
                .with_children(|c| {
                    c.spawn(UiTextBundle::new("button", 24, &theme));
                });
                c.spawn(UiCheckBoxBundle {
                    style: style!("w-64 h-32 m-8"),
                    ..Default::default()
                });
                c.spawn(UiSliderBundle {
                    style: style!("w-128 h-32 m-8"),
                    ..Default::default()
                });
                c.spawn((UiInputBoxBundle {
                    style: style!("w-128 h-32 p-4"),
                    ..Default::default()
                },));
                c.spawn(CounterBundle::default());
                spawn!{c=>
                    <MiniNodeBundle @style="flex-col" >
                        <UiButtonBundle  @style="flex-col p-4 m-4 justify-content:center"
                            UiButton=( UiButton::with_callback( Entity::PLACEHOLDER, theme.system(button_open_poppup),) )>
                            <(UiTextBundle::new("open popup", 32, &theme))/>
                        </UiButtonBundle>
                        <UiHollowBlockBundle  @style="flex-col p-4 m-4 justify-content:center"
                            UiInputExt=( UiInput::default().with_callback( Entity::PLACEHOLDER, theme.system(open_menu),).into() )>
                            <(UiTextBundle::new("open menu", 32, &theme))/>
                        </UiHollowBlockBundle>
                    </MiniNodeBundle>
                };
            });
            c.spawn(UiHollowBlockBundle {
                style: style!("w-256 h-256 p-8 m-8"),
                ..Default::default()
            })
            .with_children(|c| {
                c.spawn(UiInputBoxBundle {
                    style: style!("full m-8"),
                    ..Default::default()
                });
            });
            c.spawn(UiSunkenBlockBundle {
                style: style!("w-256 h-256 p-8 m-8"),
                ..Default::default()
            });
            c.spawn(UiHighlightBlockBundle {
                style: style!("w-256 h-256 p-8 m-8"),
                ..Default::default()
            });
        });
}

pub fn button_open_poppup(In(event): In<UiButtonEvent>, mut commands: Commands, theme: Res<Theme>) {
    if event.kind == UiButtonEventKind::Released {
        commands.entity(event.button).with_children(|c| {
            spawn! {c=>
                <UiBlockBundle ZIndex=(ZIndex::Global(1024)) @style="w-200 h-200 top-120% absolute align-self:center"
                    UiPopupExt=( UiPopupExt::from(UiPopup::new(Some(
                        theme.system(popup_animation_system::<UiAnimationDropdownConfig>),
                    ))) )>
                    <(UiTextBundle::new("popup inner", 32, &theme))/>
                </UiBlockBundle>
            }
        });
    }
}

pub fn open_menu(
    In(event): In<UiInputEvent>,
    theme: Res<Theme>,
    mut commands: Commands,
    node_query:Query<( &RelativeCursorPosition,&Node )>
) {
    match event.kind {
        UiInputEventKind::MouseRelease(MouseButton::Left) => {
            let Ok(( relative_pos,node )) = node_query.get(event.node) else {return};
            let Some(normalized) = relative_pos.normalized else {return};
            let delta = normalized * node.size();
            commands.entity(event.node).with_children(|c|{
                spawn! {c=>
                    <UiBlockBundle @style="absolute flex-col p-8 left-{delta.x} top-{delta.y}"
                        UiPopupExt=(UiPopup::new_auto_destroy(None).into())>
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
        _ => {}
    }
}

#[derive(Component, Default)]
pub struct Counter;
dway_widget! {
Counter=>
@global(theme: Theme)

@callback{[UiButtonEvent]
    fn inc( In(event): In<UiButtonEvent>, mut query: Query<&mut CounterState>) {
        let Ok(mut state) = query.get_mut(event.receiver) else {return};
        if event.kind == UiButtonEventKind::Released{
            *state.count_mut() += 1;
        }
    }
}

@use_state(count: usize)
<UiHollowBlockBundle @style="p-8">
    <UiTextBundle @style="w-64"
        Text=(Text::from_section(state.count().to_string(), TextStyle{ font_size: 32.0, ..theme.default_text_style() }))/>
    <UiHightlightButtonBundle @style="p-4 w-32 h-32 align-items:center justify-content:center" UiButton=(UiButton::new(this_entity, inc)) >
        <UiTextBundle Text=(Text::from_section("+", TextStyle{ font_size: 32.0, color: Color::WHITE, font:theme.default_font() }))/>
    </>
</>
}
