use bevy::utils::HashSet;
use bevy_relationship::{
    graph_query, relationship, AppExt, ConnectCommand, Connectable, DisconnectCommand,
};
use wayland_server::protocol::wl_output::Mode;

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    state::add_global_dispatch,
    util::rect::IRect,
};

use super::surface::{ClientHasSurface, WlSurface};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlOutput {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_output::WlOutput,
}

impl WlOutput {
    pub fn new(raw: wl_output::WlOutput) -> Self {
        Self { raw }
    }
}

#[derive(Bundle)]
pub struct WlOutputBundle {
    resource: WlOutput,
    surfaces: SurfaceList,
    pub geo: Geometry,
    pub global: GlobalGeometry,
}

impl WlOutputBundle {
    pub fn new(resource: WlOutput) -> Self {
        Self {
            resource,
            surfaces: Default::default(),
            geo: Geometry::new(IRect::new(0, 0, 1920, 1080)),
            global: Default::default(),
        }
    }
}

relationship!(ClientHasOutput=>EnteredOutputList-<ClientRef);
relationship!(SurfaceInOutput => OutputList>-<SurfaceList);

#[derive(Resource)]
pub struct OutputDelegate(pub GlobalId);
delegate_dispatch!(DWay: [wl_output::WlOutput: Entity] => OutputDelegate);
impl wayland_server::Dispatch<wl_output::WlOutput, bevy::prelude::Entity, DWay> for OutputDelegate {
    fn request(
        _state: &mut DWay,
        _client: &wayland_server::Client,
        _resource: &wl_output::WlOutput,
        request: <wl_output::WlOutput as wayland_server::Resource>::Request,
        _data: &bevy::prelude::Entity,
        _dhandle: &wayland_server::DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_output::Request::Release => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_output::WlOutput,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
impl GlobalDispatch<wl_output::WlOutput, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_output::WlOutput>,
        _global_data: &Entity,
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
            WlOutputBundle::new(WlOutput::new(output))
        });
    }
}

pub struct WlOutputPlugin;
impl Plugin for WlOutputPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<wl_output::WlOutput, 4>(app);
        app.register_type::<HashSet<Entity>>();
        app.register_type::<WlOutput>();
        app.register_relation::<ClientHasOutput>();
        app.register_relation::<SurfaceInOutput>();
    }
}

graph_query!(SurfaceToOutput=>[
    surface=(Entity, &'static WlSurface, &'static GlobalGeometry, Option<&'static EnteredOutputList>),
    client=Entity,
    output=(Entity, &'static WlOutput, &'static GlobalGeometry),
]=>{
    new_connection=surface<-[ClientHasSurface]-client-[ClientHasOutput]->output,
    old_connection=surface-[SurfaceInOutput]->output,
});

pub fn surface_enter_output(graph: SurfaceToOutput, mut commands: Commands) {
    graph.for_each_new_connection::<()>(
        |(surface_entity, surface, surface_rect, output_list),
         _,
         (output_entity, output, output_rect)| {
            if output_rect.intersection(surface_rect.geometry).area() > 0
                && !output_list
                    .map(|o| o.contains(*output_entity))
                    .unwrap_or_default()
            {
                surface.raw.enter(&output.raw);
                commands.add(ConnectCommand::<SurfaceInOutput>::new(
                    *surface_entity,
                    *output_entity,
                ));
            }
            ControlFlow::Continue
        },
    );
    graph.for_each_old_connection::<()>(
        |(surface_entity, surface, surface_rect, output_list),
         (output_entity, output, output_rect)| {
            if output_rect.intersection(surface_rect.geometry).area() <= 0
                && !output_list
                    .map(|o| o.contains(*output_entity))
                    .unwrap_or_default()
            {
                surface.raw.leave(&output.raw);
                commands.add(DisconnectCommand::<SurfaceInOutput>::new(
                    *surface_entity,
                    *output_entity,
                ));
            }
            ControlFlow::Continue
        },
    );
}
