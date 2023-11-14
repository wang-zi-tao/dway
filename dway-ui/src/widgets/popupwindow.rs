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

dway_widget! {
PopupUI(
    window_query: Query<(Ref<GlobalGeometry>, Ref<WlSurface>), With<DWayWindow>>,
)
#[derive(Reflect,Default)]{
    image: Handle<Image>,
    rect: IRect,
    bbox_rect: IRect,
} =>
{
    if let Ok((rect,surface)) = window_query.get(prop.window_entity){
        if rect.is_changed(){
            update_state!(rect = rect.geometry);
        }
        if rect.is_changed() || surface.is_changed() {
            update_state!(bbox_rect = surface.image_rect().offset(rect.pos()));
        }
        if surface.is_changed(){
            update_state!(image = surface.image.clone());
        }
    }
}
<ImageBundle UiImage=(UiImage::new(state.image.clone())) Style=(irect_to_style(state.bbox_rect))>
    <NodeBundle Style=(irect_to_style(state.rect))/>
</ImageBundle>
}

pub fn attach_popup(
    mut commands: Commands,
    mut ui_query: Query<(Entity, &mut PopupUI, &mut ZIndex)>,
    mut popup_query: Query<(Entity, &XdgPopup), (Added<DWayWindow>, Without<DWayToplevel>)>,
    mut destroy_window_events: RemovedComponents<DWayWindow>,
    window_stack: Res<WindowStack>,
) {
    let destroyed_windows: HashSet<_> = destroy_window_events.iter().collect();
    ui_query.for_each_mut(|(entity, ui, ..)| {
        if destroyed_windows.contains(&ui.window_entity) {
            commands.entity(entity).despawn_recursive();
        }
    });
    popup_query.for_each(|(entity, popup)| {
        commands.spawn(PopupUIBundle {
            node: NodeBundle {
                style: style!("absolute"),
                z_index: ZIndex::Global(
                    WINDEOW_BASE_ZINDEX
                        + WINDEOW_MAX_STEP
                            * (window_stack.list.len() as isize + popup.level) as i32,
                ),
                ..NodeBundle::default()
            },
            prop: PopupUI {
                window_entity: entity,
            },
            state: PopupUIState::default(),
            widget: PopupUIWidget::default(),
        });
    });
}

pub struct PopupWindowUIPlugin;
impl Plugin for PopupWindowUIPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PopupUI>();
        app.register_type::<PopupUIWidget>();
        app.register_type::<PopupUIState>();
        app.add_systems(Update, popupui_render.in_set(PopupUISystems::Render));
        app.add_systems(Update, attach_popup);
    }
}
