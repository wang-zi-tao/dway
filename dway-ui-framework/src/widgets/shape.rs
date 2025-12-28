use bevy_prototype_lyon::entity::Shape;

use crate::{prelude::*, render::mesh::UiMesh};

#[derive(Clone, Copy, Component, Default)]
#[require(Node, Shape, UiMesh)]
pub struct UiShape;

pub type UiShapeMaterial = MeshMaterial2d<ColorMaterial>;

pub fn after_process_shape(mut query: Query<(&mut Mesh2d, &mut UiMesh), Changed<Mesh2d>>) {
    for (mut mesh2d, mut ui_mesh) in &mut query {
        ui_mesh.set_if_neq(UiMesh::from(mesh2d.0.clone()));
    }
}
