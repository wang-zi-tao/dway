use dway_client_core::desktop::{CursorOnOutput, CursorOnWindow};
use dway_server::{
    geometry::GlobalGeometry,
    input::{pointer::WlPointer, seat::SeatHasPointer},
    util::rect::IRect,
    wl::surface::{ClientHasSurface, WlSurface},
};

use crate::prelude::*;

#[derive(Component, Clone, Debug, SmartDefault)]
pub struct Cursor {
    default_cursor: Handle<Image>,
    default_size: Vec2,
}

impl Cursor {
    pub fn new(default_cursor: Handle<Image>, default_size: Vec2) -> Self {
        Self { default_cursor, default_size }
    }
}

graph_query! { CursorQuery=>[
    surface=<Entity,With<WlSurface>>,
    client=Entity,
    pointer=<(&'static WlSurface, &'static GlobalGeometry),With<WlPointer>>,
]=>{
    pointer=surface<-[ClientHasSurface]-client-[SeatHasPointer]->pointer
}}

pub fn update_cursor_state(
    graph: CursorQuery,
    cursor_on_window: Res<CursorOnWindow>,
    mut widget_query: Query<(Ref<Cursor>, &mut CursorState)>,
    focus_screen: Res<CursorOnOutput>,
) {
    let Some((_output, pos)) = &focus_screen.0 else {
        return;
    };
    for (prop, mut state) in &mut widget_query {
        let init = state.is_added();
        let mut surface_changed = false;
        if init || cursor_on_window.is_changed() {
            let surface = cursor_on_window.0.map(|(s, _)| s);
            if init || surface != *state.surface_entity() {
                state.set_surface_entity(surface);
                surface_changed = true;
            }
        }
        if prop.is_changed() || surface_changed || cursor_on_window.is_changed() {
            let cursor_data = state.surface_entity().and_then(|surface| {
                graph.for_each_pointer_from(surface, |_, _, &(surface, global_geometry)| {
                    ControlFlow::Return((
                        surface.image.clone(),
                        surface.image_rect(),
                        global_geometry.geometry,
                    ))
                })
            });
            if let Some((image, image_rect, geo)) = cursor_data {
                state.set_cursor_image(image);
                state.set_cursor_geo(IRect::from_pos_size(
                    geo.pos() + image_rect.pos(),
                    image_rect.size(),
                ));
            } else {
                state.set_cursor_image(prop.default_cursor.clone());
                state.set_cursor_geo(IRect::from_pos_size(*pos, prop.default_size.as_ivec2()));
            }
        }
    }
}

dway_widget! {
Cursor=>
@plugin{app.add_systems(Update, update_cursor_state.in_set(CursorSystems::Render).before(cursor_render));}
@use_state(pub surface_entity: Option<Entity>)
@use_state(pub cursor_geo: IRect)
@use_state(pub cursor_image: Handle<Image>)
@state_component(#[derive(Debug)])
<ImageBundle UiImage=(state.cursor_image().clone().into()) ZIndex=(ZIndex::Global(4096))
    Style=({
        let b = state.cursor_geo();
        Style{
            top: Val::Px(b.y() as f32),
            left: Val::Px(b.x() as f32),
            width: Val::Px(b.width() as f32),
            height: Val::Px(b.height() as f32),
            position_type: PositionType::Absolute,
            ..Default::default()
        }
    })
/>
}
