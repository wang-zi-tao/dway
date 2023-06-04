use crate::{prelude::*, xdg::XdgSurface};

#[derive(Component, Clone, Copy)]
pub struct WlGeometry {
    pub position: IVec2,
    pub size: IVec2,
    pub scale: f32,
}

#[derive(Default, Clone, Component)]
pub struct GlobalGeometry {
    pub position: IVec2,
}
#[derive(Default, Clone, Component)]
pub struct Geometry {
    pub position: IVec2,
}

fn do_update_node(
    mut dest: Mut<GlobalGeometry>,
    relative: IVec2,
    mut context_rect: IVec2,
    children: Option<&Children>,
    children_query: &Query<(&mut GlobalGeometry, &XdgSurface, Option<&Children>), With<Parent>>,
) {
    context_rect += relative;
    dest.position = context_rect;
    if let Some(c) = children {
        for child in c.iter() {
            if let Ok((global, relative, children)) =
                unsafe { children_query.get_unchecked(*child) }
            {
                do_update_node(
                    global,
                    relative.geometry.unwrap_or_default().pos(),
                    context_rect,
                    children,
                    children_query,
                );
            }
        }
    }
}

pub fn update_global_physical_rect(
    mut root_query: Query<(&mut GlobalGeometry, &XdgSurface, Option<&Children>), Without<Parent>>,
    children_query: Query<(&mut GlobalGeometry, &XdgSurface, Option<&Children>), With<Parent>>,
) {
    for (global, surface, children) in root_query.iter_mut() {
        do_update_node(
            global,
            surface.geometry.unwrap_or_default().pos(),
            Default::default(),
            children,
            &children_query,
        );
    }
}

pub struct GeometryPlugin;
impl Plugin for GeometryPlugin{
    fn build(&self, app: &mut App) {
        app.add_system(update_global_physical_rect);
    }
}
