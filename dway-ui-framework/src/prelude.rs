pub(crate) use std::time::Duration;

pub(crate) use bevy::{ecs::system::SystemId, prelude::*, ui::FocusPolicy};
pub(crate) use bevy_trait_query::*;
pub use dway_ui_derive::*;
pub(crate) use smart_default::SmartDefault;

pub(crate) use crate as dway_ui_framework;
pub use crate::{
    animation::{Animation, AssetAnimationPlugin, AssetTweenExt, Interpolation, Tween},
    event::{CallbackRegisterAppExt, CallbackTypeRegister, EventDispatcher, UiEvent, make_callback},
    input::*,
    theme::Theme,
    widgets::{
        bundles::*,
        button::{UiButton, UiButtonBundle, UiButtonEvent, UiButtonEventKind, UiButtonExt},
        checkbox::{UiCheckBox, UiCheckBoxEvent, UiCheckBoxState},
        popup::*,
        scroll::UiScrollBundle,
        shader::*,
        slider::{UiSlider, UiSliderBundle, UiSliderEvent, UiSliderState},
        svg::{UiSvg},
        UiWidgetRoot,
    },
    UiFrameworkSystems,
};
