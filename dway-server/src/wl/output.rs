use bevy::utils::HashSet;

use crate::{
    geometry::GlobalGeometry, prelude::*, schedule::DWayServerSet, util::rect::IRect,
    xdg::XdgSurface,
};
use std::sync::Arc;

use super::surface::WlSurface;

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlOutput {
    #[reflect(ignore)]
    pub raw: wl_output::WlOutput,
    pub rect: IRect,
    pub surfaces: HashSet<Entity>,
}

impl WlOutput {
    pub fn new(raw: wl_output::WlOutput) -> Self {
        Self {
            raw,
            rect: IRect::new(0, 0, 1920, 1080),
            surfaces: Default::default(),
        }
    }
}

#[derive(Resource)]
pub struct OutputDelegate(pub GlobalId);
delegate_dispatch!(DWay: [wl_output::WlOutput: Entity] => OutputDelegate);
impl wayland_server::Dispatch<wl_output::WlOutput, bevy::prelude::Entity, DWay> for OutputDelegate {
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wl_output::WlOutput,
        request: <wl_output::WlOutput as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &wayland_server::DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_output::Request::Release => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data,resource);
    }
}
impl GlobalDispatch<wl_output::WlOutput, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_output::WlOutput>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        trace!("bind output");
        state.init_object(resource, data_init, WlOutput::new);
    }
}

pub struct WlOutputPlugin(pub Arc<DisplayHandle>);
impl Plugin for WlOutputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(OutputDelegate(
            self.0.create_global::<DWay, wl_output::WlOutput, ()>(3, ()),
        ));
        app.add_system(surface_enter_output.in_set(DWayServerSet::UpdateJoin));
        app.register_type::<HashSet<Entity>>();
        app.register_type::<WlOutput>();
    }
}

pub fn surface_enter_output(
    mut surfaces_query: Query<(Entity, &mut WlSurface, &XdgSurface, &GlobalGeometry)>,
    mut output_query: Query<(&mut WlOutput)>,
) {
    for mut output in output_query.iter_mut() {
        let mut new_surface_set = HashSet::new();
        let output_rect = output.rect;
        for (entity, surface, xdg_surface, global_geometry) in surfaces_query.iter() {
            let Some( size )=surface.size else{
                continue
            };
            let rect = xdg_surface.geometry.unwrap_or_default();
            if output_rect.intersection(rect).area() > 0 {
                new_surface_set.insert(entity);
            }
        }
        let mut changed = false;
        for entity in &new_surface_set {
            if !output.surfaces.contains(&entity) {
                if let Ok((entity, mut surface, xdg_surface, global_geometry)) =
                    surfaces_query.get_mut(*entity)
                {
                    surface.raw.leave(&output.raw);
                    surface.outputs.remove(&entity);
                    changed = true;
                }
            }
        }
        for entity in &output.surfaces {
            if !new_surface_set.contains(&entity) {
                if let Ok((entity, mut surface, xdg_surface, global_geometry)) =
                    surfaces_query.get_mut(*entity)
                {
                    surface.raw.enter(&output.raw);
                    surface.outputs.insert(entity);
                    changed = true;
                }
            }
        }
        if changed {
            output.surfaces = new_surface_set;
        }
    }
}
