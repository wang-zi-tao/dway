use crate::prelude::*;
use dway_client_core::{
    input::SurfaceUiNode,
    navigation::windowstack::{WindowIndex, WindowStack},
    UiAttachData,
};
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow, PopupList},
};

use crate::util::irect_to_style;

use super::popupwindow::{PopupUI, PopupUIBundle, PopupUISystems};

pub const WINDEOW_BASE_ZINDEX: i32 = 128;
pub const WINDEOW_POPUP_BASE_ZINDEX: i32 = WINDEOW_BASE_ZINDEX + 256;
pub const WINDEOW_MAX_STEP: i32 = 16;
pub const DECORATION_HEIGHT: f32 = 24.0;
pub const DECORATION_MARGIN: f32 = 2.0;

pub fn create_raw_window_material(
    image_rect: IRect,
    image: Handle<Image>,
    geo: &GlobalGeometry,
    size: Vec2,
) -> RoundedUiImageMaterial {
    let rect = geo.geometry;
    rounded_ui_image(
        16.0,
        image_rect.pos().as_vec2() / rect.size().as_vec2(),
        image_rect.size().as_vec2() / rect.size().as_vec2(),
        image,
    )
}

#[derive(Component, Reflect, Debug)]
pub struct WindowUI {
    pub window_entity: Entity,
    pub workspace_entity: Entity,
    pub screen_entity: Entity,
    pub workspace_rect: IRect,
}
impl Default for WindowUI {
    fn default() -> Self {
        Self {
            window_entity: Entity::PLACEHOLDER,
            workspace_entity: Entity::PLACEHOLDER,
            screen_entity: Entity::PLACEHOLDER,
            workspace_rect: default(),
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowUIFlush;

dway_widget! {
WindowUI=>
@plugin{
    app.register_type::<WindowUI>();
    app.add_systems(Update, apply_deferred.after(WindowUISystems::Render).before(PopupUISystems::Render));
}
@callback{ [UiButtonEvent]
    fn on_close_button_event(
        In(event): In<UiButtonEvent>,
        prop_query: Query<&WindowUI>,
        mut events: EventWriter<WindowAction>,
    ) {
        let Ok(prop) = prop_query.get(event.receiver)else{return;};
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Close(prop.window_entity));
        }
    }
}
@callback{ [UiButtonEvent]
    fn on_min_button_event(
        In(event): In<UiButtonEvent>,
        prop_query: Query<&WindowUI>,
        mut events: EventWriter<WindowAction>,
    ) {
        let Ok(prop) = prop_query.get(event.receiver)else{return;};
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Minimize(prop.window_entity));
        }
    }
}
@callback{ [UiButtonEvent]
    fn on_max_button_event(
        In(event): In<UiButtonEvent>,
        prop_query: Query<&WindowUI>,
        mut events: EventWriter<WindowAction>,
    ) {
        let Ok(prop) = prop_query.get(event.receiver)else{return;};
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Maximize(prop.window_entity));
        }
    }
}
@arg(asset_server: Res<AssetServer>)
@arg(window_stack: Res<WindowStack>)
@state_component(#[derive(Debug)])
@use_state(pub rect:IRect)
@use_state(pub bbox_rect:IRect)
@use_state(pub title:String)
@use_state(pub image:Handle<Image>)
@use_state(pub z_index:i32)
@use_state(pub popup_list:Vec<Entity>)
@global(theme: Theme)
@query(window_query:(rect,surface, toplevel, index, popups)<-Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Ref<DWayToplevel>, Ref<WindowIndex>, Option<Ref<PopupList>>), With<DWayWindow>>[prop.window_entity]->{
    let init = !widget.inited;
    if init || rect.is_changed(){
        *state.rect_mut() = rect.geometry;
    }
    if init || rect.is_changed() || surface.is_changed() {
        *state.bbox_rect_mut() = surface.image_rect().offset(rect.pos());
    }
    if init || toplevel.is_changed(){ *state.title_mut() = toplevel.title.clone().unwrap_or_default(); }
    if init || surface.is_changed(){ *state.image_mut() = surface.image.clone(); }
    if init || index.is_changed() {
        state.set_z_index(WINDEOW_BASE_ZINDEX + WINDEOW_MAX_STEP * (window_stack.list.len() - index.global) as i32);
    }
    if let Some(popups) = popups{
        if init || popups.is_changed() {
            state.set_popup_list(popups.iter().collect());
        }
    }
})
<NodeBundle @style="absolute full" >
    <MiniNodeBundle @id="content" Style=(irect_to_style(*state.rect()))
    ZIndex=(ZIndex::Global(*state.z_index())) FocusPolicy=(FocusPolicy::Block)
    // Animator<_>=(Animator::new(Tween::new(
    //     EaseFunction::BackOut,
    //     Duration::from_secs_f32(0.5),
    //     TransformScaleLens { start: Vec3::splat(0.8), end: Vec3::ONE, },
    // )))
    >
        <MiniNodeBundle @style="full absolute" @id="mouse_area"
            SurfaceUiNode=(SurfaceUiNode::new(prop.window_entity,widget.node_content_entity))
            @connect(-[UiAttachData]->(prop.window_entity))
            Interaction=(default()) FocusPolicy=(FocusPolicy::Pass)
        />
        <MaterialNodeBundle::<RoundedUiRectMaterial> @id="outter"
            ZIndex=(ZIndex::Local(0))
            @style="absolute left-{-DECORATION_MARGIN} right-{-DECORATION_MARGIN} bottom-{-DECORATION_MARGIN} top-{-DECORATION_HEIGHT}"
            @handle(RoundedUiRectMaterial=>rounded_rect(Color::WHITE*0.2, 16.0)) />
        <MaterialNodeBundle::<RoundedUiImageMaterial> @id="surface" @style="absolute full"
        @handle(RoundedUiImageMaterial=>rounded_ui_image(
            14.0,
            ( state.bbox_rect().min-state.rect().min ).as_vec2() / state.rect().size().as_vec2(),
            state.bbox_rect().size().as_vec2() / state.rect().size().as_vec2(),
            state.image().clone())) />
        <NodeBundle @id="bar" ZIndex=(ZIndex::Local(2))
            @style="absolute left-0 right-0 top-{-DECORATION_HEIGHT} height-{DECORATION_HEIGHT}" >
            <UiButtonBundle @id="close" @style="m-2 w-20 h-20"
                UiButtonExt=(UiButton::new(this_entity, on_close_button_event).into())
                @handle(UiCircleMaterial=>circle_material(Color::WHITE*0.3)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/close.svg"))) />
            </UiButtonBundle>
            <UiButtonBundle @id="max" @style="m-2 w-20 h-20"
                UiButtonExt=(UiButton::new(this_entity, on_max_button_event).into())
                @handle(UiCircleMaterial=>circle_material(Color::WHITE*0.3)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/maximize.svg"))) />
            </UiButtonBundle>
            <UiButtonBundle @id="min" @style="m-2 w-20 h-20"
                UiButtonExt=(UiButton::new(this_entity, on_min_button_event).into())
                @handle(UiCircleMaterial=>circle_material(Color::WHITE*0.3)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/minimize.svg"))) />
            </UiButtonBundle>
            <TextBundle @style="items-center justify-center m-auto"
                Text=(Text::from_section(
                    state.title(),
                    TextStyle {
                        font_size: DECORATION_HEIGHT - 2.0,
                        color: Color::WHITE,
                        font: theme.default_font(),
                    },
                ).with_justify(JustifyText::Center))
            />
        </NodeBundle>
    </>
    <MiniNodeBundle @style="absolute full"
        @for_query(_ in Query<Ref<WlSurface>>::iter_many(state.popup_list().iter())=>[ ])>
        <PopupUIBundle ZIndex=(ZIndex::Global(WINDEOW_POPUP_BASE_ZINDEX))
            PopupUI=(PopupUI{window_entity:widget.data_entity})/>
    </MiniNodeBundle>
</NodeBundle>
}

#[derive(Component)]
pub struct ScreenWindows {
    pub screen: Entity,
}
impl Default for ScreenWindows {
    fn default() -> Self {
        Self {
            screen: Entity::PLACEHOLDER,
        }
    }
}
