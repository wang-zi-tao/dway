use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        renderer::{RenderAdapter, RenderDevice},
        texture::GpuImage,
        Extract,
    },
    sprite::SpriteAssetEvents,
    ui::UiImageBindGroups,
};
use smithay::{delegate_layer_shell, wayland::shell::wlr_layer::WlrLayerShellHandler};

use crate::DWay;

impl WlrLayerShellHandler for DWay {
    fn shell_state(&mut self) -> &mut smithay::wayland::shell::wlr_layer::WlrLayerShellState {
        todo!()
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
        layer: smithay::wayland::shell::wlr_layer::Layer,
        namespace: String,
    ) {
        todo!()
    }

    fn new_popup(
        &mut self,
        parent: smithay::wayland::shell::wlr_layer::LayerSurface,
        popup: smithay::wayland::shell::xdg::PopupSurface,
    ) {
    }

    fn ack_configure(
        &mut self,
        surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        configure: smithay::wayland::shell::wlr_layer::LayerSurfaceConfigure,
    ) {
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {}
}

delegate_layer_shell!(DWay);
