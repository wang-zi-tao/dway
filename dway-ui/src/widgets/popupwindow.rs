use dway_client_core::{input::SurfaceUiNode};
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, DWayWindow, PopupList},
};

use crate::{prelude::*, util::irect_to_style};


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
@use_state(pub grab:bool)
@use_state(pub rect:IRect)
@use_state(pub bbox_rect:IRect)
@use_state(pub image:Handle<Image>)
@use_state(pub popup_list:Vec<Entity>)
@query(window_query:(rect,surface, popup, popups)<-Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Ref<XdgPopup>, Option<Ref<PopupList>>), With<DWayWindow>>[prop.window_entity]->{
    let init = !widget.inited;
    if init || rect.is_changed(){ *state.rect_mut() = rect.geometry; }
    if init || rect.is_changed() || surface.is_changed() {
        *state.bbox_rect_mut() = surface.image_rect().offset(rect.pos());
    }
    if init || surface.is_changed(){ *state.image_mut() = surface.image.clone(); }
    if init || popup.is_changed() { *state.grab_mut() = popup.grab(); }
    if let Some(popups) = popups{
        if init || popups.is_changed() {
            state.set_popup_list(popups.iter().collect());
        }
    }
})
<Node @style="absolute full">
    <(ImageNode::new(state.image().clone())) @id="content"
        Node=(irect_to_style(*state.bbox_rect())) FocusPolicy=(FocusPolicy::Block) />
    <(irect_to_style(*state.rect())) >
        <Node @id="mouse_area"
            Node=({
                let distant = if *state.grab() { 16384.0 } else { 4.0 };
                style!("absolute left-{-distant} top-{-distant} right-{-distant} bottom-{-distant}")
            })
            SurfaceUiNode=(SurfaceUiNode::new(prop.window_entity,widget.node_content_entity).with_grab(*state.grab()))
            Interaction FocusPolicy=(FocusPolicy::Pass)
        />
    </>
    <Node @style="absolute full"
        @for_query(_ in Query<Ref<WlSurface>>::iter_many(state.popup_list().iter())=>[ ])>
        <(PopupUI{window_entity:widget.data_entity}) @style="full absolute"/>
    </Node>
</Node>
}
