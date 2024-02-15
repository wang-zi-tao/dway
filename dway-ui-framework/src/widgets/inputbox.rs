use std::ops::Range;

use bevy::text::BreakLineOn;

use crate::prelude::*;

use super::text::UiTextBundle;

structstruck::strike!{
    pub struct UiInputEvent{
        pub receiver: Entity,
        pub widget: Entity,
        pub kind: 
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum UiInputEventKind {
            Enter,
            Changed,
        }
    }
}

pub struct UiInputState {
    pub data: String,
    pub selected: Option<Range<usize>>,
}

#[derive(Debug)]
pub enum UiInputCommand {
    Insert(usize, String),
    Delete(usize, String),
    Replace(usize, String, String),
}

structstruck::strike!{
    #[derive(Debug, Component, SmartDefault)]
    pub struct UiInputBox{
        pub placeholder: String,
        pub callback: Option<(Entity, SystemId<UiInputEvent>)>,
        pub kind:
            #[derive(Debug, Clone, Default)]
            enum UiInputBoxKind{
                #[default]
                Normal,
                Password,
                Path,
            },
        pub readonly: bool,
        pub multi_line: bool,
        pub storage_key: Option<String>,
    }
}

dway_widget!{
UiInputBox=>
@global(theme: Theme)
@use_state(pub data: String)
@use_state(pub cursor_position: Option<(usize,Vec2)>)
@use_state(pub undo: undo::history::History<UiInputCommand>)
@before{{
    if !widget.inited && !prop.readonly{
        state.set_cursor_position(Some((0,Vec2::ZERO)));
    }
}}
<UiTextBundle @id="text" Text=(Text{ 
    sections: {
        if state.data().is_empty() {
            vec![TextSection{
                value: prop.placeholder.clone(),
                style: TextStyle {
                    font: theme.default_font(),
                    font_size: 24.0,
                    color: theme.color("inputbox:placeholder"),
                },
            }]
        } else {
            vec![ TextSection{
                value: state.data().clone(),
                style: TextStyle {
                    font: theme.default_font(),
                    font_size: 24.0,
                    color: theme.color("inputbox:text"),
                },
            } ]
        }
    },
    justify: JustifyText::Left,
    linebreak_behavior: BreakLineOn::AnyCharacter,
}) />
<MiniNodeBundle @style="absolute full" @if(state.cursor_position().is_some())>
    <MiniNodeBundle @id="cursor" Style=(Style{
        left: Val::Px(state.cursor_position().unwrap().1.x),
        top: Val::Px(state.cursor_position().unwrap().1.y),
        ..style!("w-2 h-24")
    })
    @material(RoundedUiRectMaterial=>rounded_rect(theme.color("inputbox:cursor"), 4.0)) />
</MiniNodeBundle>
}
