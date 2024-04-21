use crate::{prelude::*, util::rect::IRect};

#[derive(Component, Clone, Copy, Reflect)]
pub struct WlGeometry {
    pub position: IVec2,
    pub size: IVec2,
    pub scale: f32,
}

#[derive(Default, Debug, Clone, Component, Reflect)]
pub struct GlobalGeometry {
    pub geometry: IRect,
}

impl GlobalGeometry {
    pub fn new(geometry: IRect) -> Self {
        Self { geometry }
    }

    pub fn add(&self, geometry: &Geometry) -> Self {
        Self {
            geometry: IRect::from_pos_size(self.pos() + geometry.pos(), geometry.size()),
        }
    }
}

impl std::ops::DerefMut for GlobalGeometry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.geometry
    }
}

impl lazy_static::__Deref for GlobalGeometry {
    type Target = IRect;

    fn deref(&self) -> &Self::Target {
        &self.geometry
    }
}
#[derive(Default, Debug, Clone, Component, Reflect)]
pub struct Geometry {
    pub geometry: IRect,
}

impl Geometry {
    pub fn new(geometry: IRect) -> Self {
        Self { geometry }
    }
}

impl std::ops::DerefMut for Geometry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.geometry
    }
}

impl lazy_static::__Deref for Geometry {
    type Target = IRect;

    fn deref(&self) -> &Self::Target {
        &self.geometry
    }
}

fn do_update_node(
    dest: Option<Mut<GlobalGeometry>>,
    relative: IRect,
    mut context_rect: IVec2,
    children: Option<&Children>,
    children_query: &Query<
        (
            Option<&mut GlobalGeometry>,
            Option<&Geometry>,
            Option<&Children>,
        ),
        With<Parent>,
    >,
) {
    context_rect += relative.pos();
    if let Some(mut dest) = dest {
        let rect = IRect::from_pos_size(context_rect, relative.size());
        if dest.geometry != rect {
            dest.geometry = rect;
        }
    }
    if let Some(c) = children {
        for &child in c.iter() {
            if let Ok((global, child_relative, children)) =
                unsafe { children_query.get_unchecked(child) }
            {
                do_update_node(
                    global,
                    child_relative.map(|r| r.geometry).unwrap_or_else(|| {
                        IRect::from_pos_size(Default::default(), relative.size())
                    }),
                    context_rect,
                    children,
                    children_query,
                );
            }
        }
    }
}

pub fn update_global_physical_rect(
    mut root_query: Query<
        (
            Option<&mut GlobalGeometry>,
            Option<&Geometry>,
            Option<&Children>,
        ),
        Without<Parent>,
    >,
    children_query: Query<
        (
            Option<&mut GlobalGeometry>,
            Option<&Geometry>,
            Option<&Children>,
        ),
        With<Parent>,
    >,
) {
    for (global, geometry, children) in root_query.iter_mut() {
        do_update_node(
            global,
            geometry.map(|r| r.geometry).unwrap_or_else(|| {
                IRect::from_pos_size(Default::default(), IVec2::new(i32::MAX, i32::MAX))
            }),
            Default::default(),
            children,
            &children_query,
        );
    }
}

pub struct GeometryPlugin;
impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            update_global_physical_rect.after(DWayServerSet::UpdateGeometry),
        );
        app.register_type::<Geometry>();
        app.register_type::<GlobalGeometry>();
        app.register_type::<WlGeometry>();
    }
}

pub fn set_geometry(geo: &mut Geometry, global_geo: &mut GlobalGeometry, rect: IRect){
    let global_pos = global_geo.pos();
    global_geo.set_pos(global_pos + rect.pos() - geo.pos());
    global_geo.set_size(rect.size());
    geo.geometry = rect;
}
