use dway_client_core::{
    input::SurfaceUiNode,
    navigation::windowstack::{WindowIndex, WindowStack},
    UiAttachData,
};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow, PopupList},
};
use dway_ui_framework::widgets::{
    button::UiRawButtonExt,
    drag::{UiDrag, UiDragEvent, UiDragExt},
};

use super::popupwindow::{PopupUI, PopupUIBundle, PopupUISystems};
use crate::{prelude::*, util::irect_to_style};

pub const WINDEOW_BASE_ZINDEX: i32 = 128;
pub const WINDEOW_POPUP_BASE_ZINDEX: i32 = WINDEOW_BASE_ZINDEX + 256;
pub const WINDEOW_MAX_STEP: i32 = 16;
pub const DECORATION_HEIGHT: f32 = 24.0;
pub const DECORATION_MARGIN: f32 = 2.0;

pub fn create_raw_window_material(
    image_rect: IRect,
    image: Handle<Image>,
    geo: &GlobalGeometry,
    _size: Vec2,
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
#[require(GlobalZIndex)]
pub struct WindowUI {
    pub window_entity: Entity,
    pub screen_geomety: IRect,
}
impl Default for WindowUI {
    fn default() -> Self {
        Self {
            window_entity: Entity::PLACEHOLDER,
            screen_geomety: Default::default(),
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowUIFlush;

dway_widget! {
WindowUI=>
@plugin{
    app.register_type::<WindowUI>();
    app.register_type::<WindowUIState>();
    app.add_systems(Update, apply_deferred.after(WindowUISystems::Render).before(PopupUISystems::Render));
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_close_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Close(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_min_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Minimize(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_max_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: EventWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.send(WindowAction::Maximize(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiDragEvent>]
    fn on_title_bar_mouse_event(
        event: UiEvent<UiDragEvent>,
        this_query: Query<&WindowUI>,
        window_query: Query<&Geometry>,
        mut events: EventWriter<WindowAction>,
    ) {
        let Ok(prop) = this_query.get(event.receiver()) else {return};
        if let UiDragEvent::Move(alpha) = &*event{
            let Ok(geo) = window_query.get(prop.window_entity) else{ return};
            events.send(WindowAction::SetRect(prop.window_entity, IRect::from_pos_size(geo.pos() + ( *alpha*0.5 ).as_ivec2(), geo.size())));
        }
    }
}
@callback{ [UiEvent<UiDragEvent>]
    fn on_decorated_mouse_event(
        event: UiEvent<UiDragEvent>,
        this_query: Query<&WindowUI>,
        window_query: Query<&Geometry>,
        mut events: EventWriter<WindowAction>,
    ) {
        let Ok(prop) = this_query.get(event.receiver()) else {return};
        if let UiDragEvent::Move(pos) = &*event{
            let Ok(geo) = window_query.get(prop.window_entity) else{ return};
            // TODO
        }
    }
}
@arg(asset_server: Res<AssetServer>)
@arg(window_stack: Res<WindowStack>)
@state_component(#[derive(Reflect)])
@use_state(pub rect:IRect)
@use_state(pub bbox_rect:IRect)
@use_state(pub title:Option<String>)
@use_state(pub decorated:bool)
@use_state(pub image:Handle<Image>)
@use_state(pub popup_list:Vec<Entity>)
@global(theme: Theme)
@world_query(z_index: &mut GlobalZIndex)
@query(window_query:(rect,surface, toplevel, index, popups)<-Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Ref<DWayToplevel>, Ref<WindowIndex>, Option<Ref<PopupList>>), With<DWayWindow>>[prop.window_entity]->{
    let init = !widget.inited || prop.is_changed();
    if init {
        commands.queue(ConnectCommand::<UiAttachData>::new(this_entity, prop.window_entity));
    }
    if init || rect.is_changed(){
        *state.rect_mut() = rect.geometry.offset(- prop.screen_geomety.pos());
    }
    if init || rect.is_changed() || surface.is_changed() {
        *state.bbox_rect_mut() = surface.image_rect().offset(rect.pos() - prop.screen_geomety.pos());
    }
    if init || toplevel.is_changed(){
        if state.title() != &toplevel.title {
            *state.title_mut() = toplevel.title.clone();
        }
        if state.decorated() != &toplevel.decorated {
            *state.decorated_mut() = toplevel.decorated;
        }
    }
    if init || surface.is_changed(){ *state.image_mut() = surface.image.clone(); }
    if init || index.is_changed() {
        let z = WINDEOW_BASE_ZINDEX + WINDEOW_MAX_STEP * (window_stack.list.len() - index.global) as i32;
        *z_index = GlobalZIndex(z);
    }
    if let Some(popups) = popups{
        if init || popups.is_changed() {
            state.set_popup_list(popups.iter().collect());
        }
    }
})
<MiniNodeBundle @id="content"
    Node=(irect_to_style(*state.rect()))
    ZIndex=(ZIndex(4))
    FocusPolicy=(FocusPolicy::Block)
>
    <MiniNodeBundle @style="full absolute" @id="mouse_area"
        SurfaceUiNode=(SurfaceUiNode::new(prop.window_entity,widget.node_content_entity))
        Interaction=(default()) FocusPolicy=(FocusPolicy::Pass)
    />
</MiniNodeBundle>
<UiNodeBundle Node=(irect_to_style(*state.bbox_rect())) @if(!*state.decorated()) @id="without_decorated">
    <ImageBundle @id="image" @style="full" ImageNode=(state.image().clone().into()) />
</UiNodeBundle>
<NodeBundle Node=(irect_to_style(*state.rect())) @if(*state.decorated())
     @id="with_decorated">
    <MaterialNodeBundle::<RoundedUiRectMaterial> @id="decorated_box"
        ZIndex=(ZIndex(0))
        UiDragExt=(UiDragExt{
            event_dispatcher: make_callback(this_entity, on_decorated_mouse_event),
            ..default()
        })
        @style="absolute left-{-DECORATION_MARGIN} right-{-DECORATION_MARGIN} bottom-{-DECORATION_MARGIN} top-{-DECORATION_HEIGHT}"
        @handle(RoundedUiRectMaterial=>rounded_rect(color!("#333333"), 16.0)) />
    <MaterialNodeBundle::<RoundedUiImageMaterial> @id="surface" @style="absolute full"
    @handle(RoundedUiImageMaterial=>rounded_ui_image(
        14.0,
        ( state.bbox_rect().min-state.rect().min ).as_vec2() / state.rect().size().as_vec2(),
        state.bbox_rect().size().as_vec2() / state.rect().size().as_vec2(),
        state.image().clone())) />
    <NodeBundle @id="title_bar"
        UiDragExt=(UiDragExt{
            event_dispatcher: make_callback(this_entity, on_title_bar_mouse_event),
            ..default()
        })
        @style="absolute left-0 right-0 top-{-DECORATION_HEIGHT} height-{DECORATION_HEIGHT}" >
        <MiniNodeBundle @id="close" @style="m-2 w-20 h-20"
            UiRawButtonExt=(UiRawButtonExt::from_callback(prop.window_entity, on_close_button_event))
            @handle(UiCircleMaterial=>circle_material(color!("#505050"))) >
            <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/close.svg"))) @style="full" />
        </MiniNodeBundle>
        <MiniNodeBundle @id="max" @style="m-2 w-20 h-20"
            UiRawButtonExt=(UiRawButtonExt::from_callback(prop.window_entity, on_max_button_event))
            @handle(UiCircleMaterial=>circle_material(color!("#505050"))) >
            <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/maximize.svg"))) @style="full" />
        </MiniNodeBundle>
        <MiniNodeBundle @id="min" @style="m-2 w-20 h-20"
            UiRawButtonExt=(UiRawButtonExt::from_callback(prop.window_entity, on_min_button_event))
            @handle(UiCircleMaterial=>circle_material(color!("#505050"))) >
            <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/minimize.svg"))) @style="full" />
        </MiniNodeBundle>
        <Node @id="title" @style="items-center justify-center m-auto"
            Text=(Text::new(state.title().as_deref().unwrap_or_default()))
            TextFont=(theme.text_font(DECORATION_HEIGHT - 2.0))
            TextColor=(Color::WHITE.into())
            TextLayout=( TextLayout::new_with_justify(JustifyText::Left) )
        />
    </NodeBundle>
</NodeBundle>
<MiniNodeBundle @style="absolute full"
    @for_query(_ in Query<Ref<WlSurface>>::iter_many(state.popup_list().iter())=>[ ])>
    <PopupUIBundle GlobalZIndex=(GlobalZIndex(WINDEOW_POPUP_BASE_ZINDEX))
        PopupUI=(PopupUI{window_entity:widget.data_entity})/>
</MiniNodeBundle>
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
