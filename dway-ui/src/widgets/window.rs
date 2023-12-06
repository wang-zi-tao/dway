use bevy::utils::{HashMap, HashSet};
use dway_client_core::{navigation::windowstack::{WindowIndex, WindowStack}, input::SurfaceUiNode};
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow, PopupList},
};

use crate::{animation, framework::svg::UiSvgBundle, prelude::*};
use crate::{
    framework::{
        animation::despawn_animation,
        button::{UiButton, UiButtonAddonBundle, UiButtonBundle, UiButtonEvent, UiButtonEventKind},
    },
    util::irect_to_style,
};

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
) -> RoundedUiImageMaterial {
    let rect = geo.geometry;
    let bbox_rect = image_rect.offset(rect.pos());
    RoundedUiImageMaterial::new(
        rect.size().as_vec2(),
        16.0,
        (bbox_rect.min - rect.min).as_vec2(),
        bbox_rect.size().as_vec2(),
        image,
    )
}

pub fn create_window_material(surface: &WlSurface, geo: &GlobalGeometry) -> RoundedUiImageMaterial {
    let rect = geo.geometry;
    let bbox_rect = surface.image_rect().offset(rect.pos());
    RoundedUiImageMaterial::new(
        rect.size().as_vec2(),
        16.0,
        (bbox_rect.min - rect.min).as_vec2(),
        bbox_rect.size().as_vec2(),
        surface.image.clone(),
    )
}

pub fn window_mouse_event(
    ui_query: Query<(&Node, &GlobalGeometry, &Interaction, &SurfaceUiNode)>,
    window_query: Query<(&WlSurface, &GlobalGeometry)>,
) {
    ui_query.for_each(|(node, global, interaction, content)| {
        if *interaction != Interaction::Hovered {
            return;
        }
        let Ok((surface, global)) = window_query.get(content.surface_entity) else {
            return;
        };
    });
}

#[derive(Component, Reflect, Debug)]
pub struct WindowUI {
    pub window_entity: Entity,
    pub app_entry: Entity,
}
impl Default for WindowUI {
    fn default() -> Self {
        Self {
            window_entity: Entity::PLACEHOLDER,
            app_entry: Entity::PLACEHOLDER,
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
    ZIndex=(ZIndex::Global(*state.z_index()))
    SurfaceUiNode=(SurfaceUiNode::new(prop.window_entity,this_entity))
    Interaction=(default()) FocusPolicy=(FocusPolicy::Block)
    Animator<_>=(Animator::new(Tween::new(
        EaseFunction::BackOut,
        Duration::from_secs_f32(0.5),
        TransformScaleLens { start: Vec3::splat(0.8), end: Vec3::ONE, },
    ))) >
        <MaterialNodeBundle::<RoundedUiRectMaterial> @id="outter"
            ZIndex=(ZIndex::Local(0))
            Style=(Style{
                position_type: PositionType::Absolute,
                left:Val::Px(-DECORATION_MARGIN),
                right:Val::Px(-DECORATION_MARGIN),
                bottom:Val::Px(-DECORATION_MARGIN),
                top:Val::Px(-DECORATION_HEIGHT),
                ..Style::default() })
            @handle(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(Color::WHITE*0.2, 16.0)) />
        <MaterialNodeBundle::<RoundedUiImageMaterial> @id="surface" @style="absolute full"
        @handle(RoundedUiImageMaterial=>RoundedUiImageMaterial::new(
            state.rect().size().as_vec2(),
            14.0,
            ( state.bbox_rect().min-state.rect().min ).as_vec2(),
            state.bbox_rect().size().as_vec2(),
            state.image().clone())) />
        <NodeBundle @id="bar"
            ZIndex=(ZIndex::Local(2))
            Style=(Style{
                position_type: PositionType::Absolute,
                left:Val::Px(0.),
                right:Val::Px(0.),
                top:Val::Px(- DECORATION_HEIGHT),
                height: Val::Px(DECORATION_HEIGHT),
                ..Style::default() })
        >
            <UiButtonBundle @id="close" @style="m-2 w-20 h-20"
                UiButtonAddonBundle=(UiButton::new(this_entity, on_close_button_event).into())
                @handle(UiCircleMaterial=>UiCircleMaterial::new(Color::WHITE*0.3, 8.0)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/close.svg"))) />
            </UiButtonBundle>
            <UiButtonBundle @id="max" @style="m-2 w-20 h-20"
                UiButtonAddonBundle=(UiButton::new(this_entity, on_max_button_event).into())
                @handle(UiCircleMaterial=>UiCircleMaterial::new(Color::WHITE*0.3, 8.0)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/maximize.svg"))) />
            </UiButtonBundle>
            <UiButtonBundle @id="min" @style="m-2 w-20 h-20"
                UiButtonAddonBundle=(UiButton::new(this_entity, on_min_button_event).into())
                @handle(UiCircleMaterial=>UiCircleMaterial::new(Color::WHITE*0.3, 8.0)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/minimize.svg"))) />
            </UiButtonBundle>
            <TextBundle @style="items-center justify-center m-auto"
                Text=(Text::from_section(
                    state.title(),
                    TextStyle {
                        font_size: DECORATION_HEIGHT - 2.0,
                        color: Color::WHITE,
                        font: asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf"),
                    },
                ).with_alignment(TextAlignment::Center))
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
