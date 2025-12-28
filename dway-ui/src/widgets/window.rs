use bevy::ui::RelativeCursorPosition;
use dway_client_core::{
    input::{GrabRequestKind, SurfaceInputEvent, SurfaceUiNode},
    navigation::windowstack::{WindowIndex, WindowStack},
    DWayClientPlugin, UiAttachData,
};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow, PopupList},
};
use dway_ui_framework::widgets::drag::{UiDrag, UiDragEvent, UiDragEventDispatcher};

use super::popupwindow::{PopupUI, PopupUISystems};
use crate::{prelude::*, util::irect_to_style};

pub const WINDEOW_BASE_ZINDEX: i32 = 128;
pub const WINDEOW_POPUP_BASE_ZINDEX: i32 = WINDEOW_BASE_ZINDEX + 256;
pub const WINDEOW_MAX_STEP: i32 = 16;
pub const DECORATION_HEIGHT: f32 = 24.0;
pub const DECORATION_MARGIN: f32 = 2.0;

pub fn ui_input_event_to_surface_input_event(
    surface_entity: Entity,
    computed_node: &ComputedNode,
    relative_cursor_position: &RelativeCursorPosition,
    global_transform: &UiGlobalTransform,
    event: &UiInputEvent,
    window_geometry: Geometry,
) -> Option<SurfaceInputEvent> {
    let mouse_position =
        relative_cursor_position.normalized.unwrap_or_default() * computed_node.size();

    let surface_rect = Rect::from_center_size(global_transform.translation, computed_node.size());

    let surface_input_event_kind = match &*event {
        UiInputEvent::MouseEnter => Some(GrabRequestKind::Enter()),
        UiInputEvent::MouseLeave => Some(GrabRequestKind::Leave()),
        UiInputEvent::MousePress(_) => None,
        UiInputEvent::MouseRelease(_) => None,
        UiInputEvent::KeyboardEnter => None,
        UiInputEvent::KeyboardLeave => None,
        UiInputEvent::MouseMove(vec2) => Some(GrabRequestKind::Move(vec2 * surface_rect.size())),
        UiInputEvent::KeyboardInput(keyboard_input) => {
            Some(GrabRequestKind::KeyboardInput(keyboard_input.clone()))
        }
        UiInputEvent::Wheel(mouse_wheel) => Some(GrabRequestKind::Asix(mouse_wheel.clone())),
        UiInputEvent::RawMouseButton(mouse_button_input) => {
            Some(GrabRequestKind::Button(mouse_button_input.clone()))
        }
    };
    surface_input_event_kind.map(|kind| SurfaceInputEvent {
        surface_entity: Some(surface_entity),
        mouse_position,
        surface_rect,
        kind,
        window_geometry,
    })
}

pub fn on_window_ui_input(
    event: UiEvent<UiInputEvent>,
    query: Query<(&WindowUI, &WindowUIState, &WindowUIWidget)>,
    contents_query: Query<(&ComputedNode, &RelativeCursorPosition, &UiGlobalTransform)>,
    mut surface_input_events: MessageWriter<SurfaceInputEvent>,
) {
    let Ok((prop, state, widget)) = query.get(event.receiver()) else {
        return;
    };
    let Ok((computed_node, relative_cursor_position, global_transform)) =
        contents_query.get(widget.node_content_entity)
    else {
        return;
    };

    if let Some(surface_input_event) = ui_input_event_to_surface_input_event(
        prop.window_entity,
        computed_node,
        relative_cursor_position,
        global_transform,
        &event,
        Geometry::new(*state.rect()),
    ) {
        surface_input_events.write(surface_input_event);
    }
}

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
    app.configure_sets(PreUpdate, DWayClientSystem::Input.after(UiFrameworkSystems::InputSystems));
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_close_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: MessageWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.write(WindowAction::Close(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_min_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: MessageWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.write(WindowAction::Minimize(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn on_max_button_event(
        event: UiEvent<UiButtonEvent>,
        mut events: MessageWriter<WindowAction>,
    ) {
        if event.kind == UiButtonEventKind::Released{
            events.write(WindowAction::Maximize(event.receiver()));
        }
    }
}
@callback{ [UiEvent<UiDragEvent>]
    fn on_title_bar_mouse_event(
        event: UiEvent<UiDragEvent>,
        this_query: Query<&WindowUI>,
        window_query: Query<&Geometry>,
        mut events: MessageWriter<WindowAction>,
    ) {
        let Ok(prop) = this_query.get(event.receiver()) else {return};
        if let UiDragEvent::Move{ delta: delta, .. } = &*event{
            let Ok(geo) = window_query.get(prop.window_entity) else{ return};
            events.write(WindowAction::SetRect(prop.window_entity, IRect::from_pos_size(geo.pos() + delta.as_ivec2(), geo.size())));
        }
    }
}
@callback{ [UiEvent<UiDragEvent>]
    fn on_decorated_mouse_event(
        event: UiEvent<UiDragEvent>,
        this_query: Query<&WindowUI>,
        window_query: Query<&Geometry>,
        events: MessageWriter<WindowAction>,
    ) {
        let Ok(prop) = this_query.get(event.receiver()) else {return};
        if let UiDragEvent::Move{ delta: delta, .. } = &*event{
            let Ok(geo) = window_query.get(prop.window_entity) else{ return};
            // TODO
        }
    }
}
@add_callback{ [UiEvent<UiInputEvent>] on_window_ui_input}
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
<UiInput @id="content"
    Node=(irect_to_style(*state.rect()))
    ZIndex=(ZIndex(4))
    RelativeCursorPosition
    FocusPolicy=(FocusPolicy::Block)
    @on_event(on_window_ui_input)
/>
<(irect_to_style(*state.bbox_rect())) @if(!*state.decorated()) @id="without_decorated">
    <(ImageNode::from(state.image().clone())) @id="image" @style="full" />
</Node>
<(irect_to_style(*state.rect())) @if(*state.decorated())
     @id="with_decorated">
    <MaterialNode::<RoundedUiRectMaterial> @id="decorated_box"
        ZIndex=(ZIndex(0))
        UiDrag @on_event(on_decorated_mouse_event)
        @style="absolute left-{-DECORATION_MARGIN} right-{-DECORATION_MARGIN} bottom-{-DECORATION_MARGIN} top-{-DECORATION_HEIGHT}"
        @handle(RoundedUiRectMaterial=>rounded_rect(color!("#333333"), 16.0)) />
    <MaterialNode::<RoundedUiImageMaterial> @id="surface" @style="absolute full"
    @handle(RoundedUiImageMaterial=>rounded_ui_image(
        14.0,
        ( state.bbox_rect().min-state.rect().min ).as_vec2() / state.rect().size().as_vec2(),
        state.bbox_rect().size().as_vec2() / state.rect().size().as_vec2(),
        state.image().clone())) />
    <Node @id="title_bar"
        UiDrag=(UiDrag{ auto_move: false,..Default::default() }) @on_event(on_title_bar_mouse_event)
        @style="absolute left-0 right-0 top-{-DECORATION_HEIGHT} height-{DECORATION_HEIGHT}" >
        <Node @id="close" @style="m-2 w-20 h-20"
            UiButton NoTheme @on_event(on_close_button_event->prop.window_entity)
            @handle(UiCircleMaterial=>circle_material(color!("#505050"))) >
            <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/close.svg"))) @style="full" />
        </Node>
        <Node @id="max" @style="m-2 w-20 h-20"
            UiButton NoTheme @on_event(on_max_button_event->prop.window_entity)
            @handle(UiCircleMaterial=>circle_material(color!("#505050"))) >
            <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/maximize.svg"))) @style="full" />
        </Node>
        <Node @id="min" @style="m-2 w-20 h-20"
            UiButton NoTheme @on_event(on_min_button_event->prop.window_entity)
            @handle(UiCircleMaterial=>circle_material(color!("#505050"))) >
            <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/minimize.svg"))) @style="full" />
        </Node>
        <Node @id="title" @style="items-center justify-center m-auto"
            Text=(Text::new(state.title().as_deref().unwrap_or_default()))
            TextFont=(theme.text_font(DECORATION_HEIGHT - 2.0))
            TextColor=(Color::WHITE.into())
            TextLayout=( TextLayout::new_with_justify(Justify::Left) )
        />
    </Node>
</Node>
<Node @style="absolute full"
    @for_query(_ in Query<Ref<WlSurface>>::iter_many(state.popup_list().iter())=>[ ])>
    <(PopupUI{window_entity:widget.data_entity}) GlobalZIndex=(GlobalZIndex(WINDEOW_POPUP_BASE_ZINDEX)) />
</Node>
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
