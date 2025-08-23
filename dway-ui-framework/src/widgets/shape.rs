use bevy_prototype_lyon::entity::Shape;
pub use bevy_prototype_lyon::{draw::*, entity::ShapeBundle};

use crate::{prelude::*, render::mesh::UiMeshHandle};

#[derive(Clone, Copy, Component, Default)]
#[require(Node, Shape, UiMeshHandle)]
#[require(MeshMaterial2d<ColorMaterial>=ShapeBundle::default().material)]
pub struct UiShape;

pub fn after_process_shape(mut query: Query<(&mut Mesh2d, &mut UiMeshHandle), Changed<Mesh2d>>) {
    for (mut mesh2d, mut ui_mesh) in &mut query {
        ui_mesh.set_if_neq(UiMeshHandle::from(mesh2d.0.clone()));
        mesh2d.0 = Default::default();
    }
}
