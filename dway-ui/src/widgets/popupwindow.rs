use bevy::utils::HashSet;
use dway_client_core::navigation::windowstack::WindowStack;
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, toplevel::DWayToplevel, DWayWindow},
};

use crate::{prelude::*, util::irect_to_style};

use super::window::{WINDEOW_BASE_ZINDEX, WINDEOW_MAX_STEP};

#[derive(Component, Reflect, Debug)]
pub struct PopupUI {
    window_entity: Entity,
}
impl Default for PopupUI {
    fn default() -> Self {
        Self {
            window_entity: Entity::PLACEHOLDER,
        }
    }
}

dway_widget! {
PopupUI=>
@plugin{
    app.register_type::<PopupUI>();
    app.add_systems(Update, attach_popup);
}
@use_state(pub rect:IRect)
@use_state(pub bbox_rect:IRect)
@use_state(pub image:Handle<Image>)
@query(window_query:(rect,surface)<-Query<(Ref<GlobalGeometry>, Ref<WlSurface>), With<DWayWindow>>[prop.window_entity]->{
    let init = !widget.inited;
    if init || rect.is_changed(){ *state.rect_mut() = rect.geometry; }
    if init || rect.is_changed() || surface.is_changed() {
        *state.bbox_rect_mut() = surface.image_rect().offset(rect.pos());
    }
    if init || surface.is_changed(){ *state.image_mut() = surface.image.clone(); }
})
<ImageBundle UiImage=(UiImage::new(state.image().clone())) Style=(irect_to_style(*state.bbox_rect()))>
    <NodeBundle Style=(irect_to_style(*state.rect()))/>
</ImageBundle>
}

pub fn attach_popup(
    mut commands: Commands,
    mut ui_query: Query<(Entity, &mut PopupUI, &mut ZIndex)>,
    mut popup_query: Query<(Entity, &XdgPopup), (Added<DWayWindow>, Without<DWayToplevel>)>,
    mut destroy_window_events: RemovedComponents<DWayWindow>,
    window_stack: Res<WindowStack>,
) {
    let destroyed_windows: HashSet<_> = destroy_window_events.read().collect();
    ui_query.for_each_mut(|(entity, ui, ..)| {
        if destroyed_windows.contains(&ui.window_entity) {
            commands.entity(entity).despawn_recursive();
        }
    });
    popup_query.for_each(|(entity, popup)| {
        commands.spawn((
            Name::from("PopupUI"),
            PopupUIBundle {
                style: style!("absolute"),
                z_index: ZIndex::Global(
                    WINDEOW_BASE_ZINDEX
                        + WINDEOW_MAX_STEP
                            * (window_stack.list.len() as isize + popup.level) as i32,
                ),
                prop: PopupUI {
                    window_entity: entity,
                },
                ..default()
            },
        ));
    });
}
