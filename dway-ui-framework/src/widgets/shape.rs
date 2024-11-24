use bevy::sprite::Mesh2dHandle;
pub use bevy_prototype_lyon::{
    draw::*,
    entity::{Path, ShapeBundle},
    geometry::GeometryBuilder,
    path::PathBuilder,
};

use crate::{
    make_bundle,
    prelude::*,
    render::mesh::{UiMeshHandle, UiMeshTransform},
};

pub fn after_process_shape(
    mut query: Query<(&mut Mesh2dHandle, &mut UiMeshHandle), Changed<Mesh2dHandle>>,
) {
    for (mut mesh2d, mut ui_mesh) in &mut query {
        ui_mesh.set_if_neq(UiMeshHandle::from(mesh2d.0.clone()));
        mesh2d.0 = Default::default();
    }
}

make_bundle! {
    UiShapeBundle{
        pub path: Path,
        pub mesh2d: Mesh2dHandle,
        pub mesh: UiMeshHandle,
        pub mesh_transform: UiMeshTransform,
        pub focus_policy: FocusPolicy,
        #[default(ShapeBundle::default().material)]
        pub material: Handle<ColorMaterial>,
    }
}

impl From<Path> for UiShapeBundle {
    fn from(value: Path) -> Self {
        Self {
            path: value,
            ..Default::default()
        }
    }
}
