use bevy::utils::HashSet;
use bevy_relationship::{ConnectCommand, Connectable, DisconnectCommand};
use wayland_server::protocol::wl_output::Mode;

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    schedule::DWayServerSet,
    state::create_global_system_config,
    util::rect::IRect,
    xdg::XdgSurface,
};
use std::sync::Arc;

use super::surface::{ContainsSurface, InOutput, InOutputRelationship, WlSurface};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlOutput {
    #[reflect(ignore)]
    pub raw: wl_output::WlOutput,
    pub rect: IRect,
}

impl WlOutput {
    pub fn new(raw: wl_output::WlOutput) -> Self {
        Self {
            raw,
            rect: IRect::new(0, 0, 1920, 1080),
        }
    }
}

#[derive(Bundle)]
pub struct WlOutputBundle {
    resource: WlOutput,
    surfaces: ContainsSurface,
}

impl WlOutputBundle {
    pub fn new(resource: WlOutput) -> Self {
        Self {
            resource,
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
        state.despawn_object(*data, resource);
    }
}
impl GlobalDispatch<wl_output::WlOutput, Entity> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_output::WlOutput>,
        global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind_spawn(client, resource, data_init, |output| {
            output.geometry(
                0,
                0,
                1920,
                1080,
                wl_output::Subpixel::VerticalRgb,
                "dway".to_string(),
                "dway".to_string(),
                wl_output::Transform::Normal,
            );
            output.mode(Mode::Current, 1920, 1080, 60000);
            if output.version() >= 4 {
                output.name("WL-1".to_string());
                output.description("dway output".to_string())
            }

            if output.version() >= 2 {
                output.scale(1);
                output.done();
            }
            WlOutput::new(output)
        });
    }
}

pub struct WlOutputPlugin;
impl Plugin for WlOutputPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(create_global_system_config::<wl_output::WlOutput, 4>());
        app.add_system(surface_enter_output.in_set(DWayServerSet::UpdateJoin));
        app.register_type::<HashSet<Entity>>();
        app.register_type::<WlOutput>();
    }
}

pub fn surface_enter_output(
    mut surfaces_query: Query<(Entity, &WlSurface, &Geometry, &GlobalGeometry, &InOutput)>,
    mut output_query: Query<(Entity, &WlOutput, &ContainsSurface)>,
    mut commands: Commands,
) {
    for (output_entity, output, contains_surfaces) in output_query.iter_mut() {
        let old_surface_set: HashSet<Entity> = contains_surfaces.iter().collect();
        let mut new_surface_set = HashSet::new();
        let output_rect = output.rect;
        for (entity, surface, geometry, global_geometry, in_output) in surfaces_query.iter() {
            let Some( size )=surface.size else{
                continue
            };
            let rect = geometry.geometry;
            if output_rect.intersection(rect).area() > 0 {
                new_surface_set.insert(entity);
            }
        }
        for entity in &new_surface_set {
            if !old_surface_set.contains(&entity) {
                if let Ok((entity, mut surface, xdg_surface, global_geometry, in_output)) =
                    surfaces_query.get_mut(*entity)
                {
                    surface.raw.leave(&output.raw);
                    commands.add(DisconnectCommand::<InOutputRelationship>::new(
                        output_entity,
                        entity,
                    ));
                }
            }
        }
        for entity in contains_surfaces.iter() {
            if !new_surface_set.contains(&entity) {
                if let Ok((entity, mut surface, xdg_surface, global_geometry, in_output)) =
                    surfaces_query.get_mut(entity)
                {
                    surface.raw.enter(&output.raw);
                    commands.add(ConnectCommand::<InOutputRelationship>::new(
                        output_entity,
                        entity,
                    ));
                }
            }
        }
    }
}
