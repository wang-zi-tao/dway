pub(crate) use std::time::Duration;

pub(crate) use bevy::{ecs::system::SystemId, prelude::*, ui::FocusPolicy};
pub(crate) use bevy_trait_query::*;
pub use dway_ui_derive::*;
pub(crate) use smart_default::SmartDefault;

pub(crate) use crate as dway_ui_framework;
pub use crate::{
    animation::{
        translation::UiTranslationAnimation, ui::AnimationTargetNodeState, Animation,
        AssetAnimationPlugin, AssetTweenExt, Interpolation, Tween,
    },
    event::{
        make_callback, CallbackRegisterAppExt, CallbackTypeRegister, EventDispatcher, UiEvent,
    },
    input::*,
    mvvm::{
        container::{ItemCell, ItemCellPlugin},
        list::{ListViewLayout, ListViewModelPlugin},
        view::{list::ListViewBundle, TextViewFactory},
        viewmodel::{SimpleItemViewModel, SimpleListViewModel, ViewModelPlugin},
    },
    render::mesh::UiMeshHandle,
    shader::{ShaderAsset, ShaderPlugin, ShapeRender, Transformed},
    text::{
        cursor::UiTextCursor, editor::UiTextEditor, selection::UiTextSelection,
        textarea::UiTextArea,
    },
    theme::{BlockStyle, NoTheme, Theme, ThemeHightlight},
    util::DwayUiDirection,
    widgets::{
        button::{UiButton, UiButtonEvent, UiButtonEventDispatcher, UiButtonEventKind},
        checkbox::{UiCheckBox, UiCheckBoxEvent, UiCheckBoxState},
        popup::*,
        shape::UiShape,
        shader::*,
        slider::{UiSlider, UiSliderEvent, UiSliderState},
        svg::UiSvg,
        UiWidgetRoot,
    },
};
