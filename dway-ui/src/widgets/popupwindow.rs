use bevy::utils::HashSet;
use dway_client_core::{input::SurfaceUiNode, navigation::windowstack::WindowStack};
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, toplevel::DWayToplevel, DWayWindow, PopupList},
};

use crate::{prelude::*, util::irect_to_style};

use super::window::{WINDEOW_BASE_ZINDEX, WINDEOW_MAX_STEP};

#[derive(Component, Reflect, Debug)]
pub struct PopupUI {
    pub window_entity: Entity,
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
}
@use_state(pub rect:IRect)
@use_state(pub bbox_rect:IRect)
@use_state(pub image:Handle<Image>)
@use_state(pub popup_list:Vec<Entity>)
@query(window_query:(rect,surface, popups)<-Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Option<Ref<PopupList>>), With<DWayWindow>>[prop.window_entity]->{
    let init = !widget.inited;
    if init || rect.is_changed(){ *state.rect_mut() = rect.geometry; }
    if init || rect.is_changed() || surface.is_changed() {
        *state.bbox_rect_mut() = surface.image_rect().offset(rect.pos());
    }
    if init || surface.is_changed(){ *state.image_mut() = surface.image.clone(); }
    if let Some(popups) = popups{
        if init || popups.is_changed() {
            state.set_popup_list(popups.iter().collect());
        }
    }
})
<MiniNodeBundle @style="absolute full">
    <ImageBundle UiImage=(UiImage::new(state.image().clone())) @id="content"
        Style=(irect_to_style(*state.bbox_rect())) FocusPolicy=(FocusPolicy::Block) >
        <NodeBundle Style=(irect_to_style(*state.rect()))/>
        <MiniNodeBundle @style="full absolute" @id="mouse_area"
            SurfaceUiNode=(SurfaceUiNode::new(prop.window_entity,widget.node_content_entity))
            Interaction=(default()) FocusPolicy=(FocusPolicy::Pass)
        />
    </ImageBundle>
    <MiniNodeBundle @style="absolute full"
        @for_query(_ in Query<Ref<WlSurface>>::iter_many(state.popup_list().iter())=>[ ])>
        <PopupUIBundle PopupUI=(PopupUI{window_entity:widget.data_entity}) @style="full absolute"/>
    </MiniNodeBundle>
</MiniNodeBundle>
}
