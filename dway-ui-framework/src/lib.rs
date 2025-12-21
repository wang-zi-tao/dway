#![feature(round_char_boundary)]
#![feature(btree_cursors)]

pub mod animation;
pub mod assets;
pub mod command;
pub mod diagnostics;
pub mod event;
pub mod future;
pub mod input;
pub mod mvvm;
pub mod prelude;
pub mod render;
pub mod shader;
pub mod theme;
pub mod util;
pub mod widget;
pub mod widgets;
pub mod text;

#[cfg(feature = "develop")]
pub mod develop;

pub mod reexport {
    #[cfg(feature = "hot_reload")]
    pub use bevy_dexterous_developer;
    #[cfg(feature = "hot_reload")]
    pub use dexterous_developer;
    pub use smart_default::SmartDefault;
    pub mod shape {
        pub use bevy_prototype_lyon::prelude::*;
    }
}

use animation::AnimationEvent;
use bevy::{render::view::VisibilitySystems, ui::UiSystem};
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_svg::{prelude::Svg, SvgPlugin};
pub use dway_ui_derive::*;
use dway_util::asset_cache::AssetCachePlugin;
use egui_dock::egui::PopupAnchor;
use event::EventReceiver;
use widgets::drag::UiDrag;

use crate::{
    prelude::*,
    render::mesh::{UiMesh, UiMeshMaterialPlugin, UiMeshTransform},
    widgets::svg::{SvgLayout},
};

pub struct UiFrameworkPlugin;
impl Plugin for UiFrameworkPlugin {
    fn build(&self, app: &mut App) {
        use UiFrameworkSystems::*;
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        if !app.is_plugin_added::<ShapePlugin>() {
            app.add_plugins(ShapePlugin);
        }
        app.insert_resource(CallbackTypeRegister::default());
        app.add_plugins((
            assets::UiAssetsPlugin,
            theme::ThemePlugin,
            theme::flat::FlatThemePlugin::default(),
            render::mesh::UiMeshPlugin,
            render::mesh::UiMeshMaterialPlugin::<ColorMaterial>::default(),
            render::blur::BlurRenderPlugin,
            render::layer_manager::LayerManagerPlugin,
            render::ui_nodes::UiNodeRenderPlugin,
            shader::ShaderFrameworkPlugin,
            mvvm::MvvmPlugin,
            animation::AnimationPlugin,
            AssetCachePlugin::<Svg>::default(),
        ))
        .add_plugins((
            text::UiTextPlugin::default(),
            widgets::slider::UiSliderPlugin,
            widgets::scroll::UiScrollPlugin,
            widgets::combobox::UiComboBoxPlugin,
            widgets::inputbox::UiInputBoxPlugin,
            UiMeshMaterialPlugin::<Svg>::default(),
        ))
        .add_event::<event::DespawnLaterEvent>()
        .add_systems(
            Last,
            event::on_despawn_later_event.run_if(on_event::<event::DespawnLaterEvent>),
        )
        .register_type::<UiCheckBox>()
        .register_type::<UiCheckBoxState>()
        .register_type::<UiSlider>()
        .register_type::<UiButton>()
        .register_type::<UiSvg>()
        .register_type::<UiPopup>()
        .register_type::<AnchorPolicy>()
        .register_type::<AttachToAnchor>()
        .register_type::<Anchor>()
        .register_type::<SvgLayout>()
        .register_type::<input::UiInput>()
        .register_type::<animation::Animation>()
        .register_type::<animation::ui::AnimationTargetNodeState>()
        .register_type::<input::MousePosition>()
        .init_resource::<input::MousePosition>()
        .register_type::<input::UiFocusState>()
        .init_resource::<input::UiFocusState>()
        .add_event::<input::UiFocusEvent>()
        .register_type::<input::UiFocusEvent>()
        .register_type::<UiDrag>()
        .register_callback(delay_destroy)
        .register_component_as::<dyn EventReceiver<AnimationEvent>, UiPopup>()
        .add_systems(
            PreUpdate,
            (
                input::update_mouse_position
                    .run_if(on_event::<CursorMoved>)
                    .in_set(InputSystems),
                update_ui_input.in_set(InputSystems).after(UiSystem::Focus),
                widgets::button::update_ui_button.in_set(WidgetInputSystems),
                widgets::checkbox::update_ui_checkbox.in_set(WidgetInputSystems),
                widgets::drag::update_ui_drag.in_set(WidgetInputSystems),
            ),
        )
        .add_systems(
            PostUpdate,
            (
                widgets::svg::update_uisvg.in_set(UpdateWidgets),
                widgets::shape::after_process_shape
                    .in_set(ProcessMesh)
                    .before(VisibilitySystems::CheckVisibility)
                    .before(VisibilitySystems::CalculateBounds)
                    .after(bevy_prototype_lyon::plugin::BuildShapes),
                (widgets::popup::anchor_update_system, widgets::popup::update_popup ).in_set(UpdatePopup),
            ),
        )
        .configure_sets(
            PreUpdate,
            (
                InputSystems.after(bevy::ui::UiSystem::Focus),
                WidgetInputSystems,
            )
                .chain(),
        )
        .configure_sets(
            PostUpdate,
            (
                UpdateViewLayout,
                UpdateMVVM,
                UpdateWidgets,
                (UpdatePopup, UpdateTheme, ApplyAnimation),
            )
                .before(UiSystem::Layout),
        )
        .configure_sets(Last, UpdateLayersMaterial.after(UpdateLayers))
        .add_plugins((
            RoundedUiRectMaterial::plugin(),
            UiCircleMaterial::plugin(),
            RoundedUiImageMaterial::plugin(),
            RoundedBlockMaterial::plugin(),
            RoundedBorderBlockMaterial::plugin(),
            HollowBlockMaterial::plugin(),
            ButtonMaterial::plugin(),
            UiImageMaterial::plugin(),
            RoundedRainbowBlockMaterial::plugin(),
            Fake3dButton::plugin(),
            CheckboxMaterial::plugin(),
            RoundedInnerShadowBlockMaterial::plugin(),
            ArcMaterial::plugin(),
            AssetAnimationPlugin::<CheckboxMaterial>::default(),
        ));

        app.add_plugins(bevy_image_export::ImageExportPlugin::default());

        #[cfg(feature = "develop")]
        {
            app.add_plugins(develop::inspector::InspectorPlugin);
        }
    }
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum UiFrameworkSystems {
    InputSystems,
    WidgetInputSystems,
    UpdateViewLayout,
    UpdateMVVM,
    UpdateWidgets,
    UpdatePopup,
    UpdateTheme,
    UpdateLayers,
    UpdateLayersMaterial,
    ApplyAnimation,
    ProcessMesh,
}
