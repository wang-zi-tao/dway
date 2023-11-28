use bevy::utils::{HashMap, HashSet};
use dway_client_core::navigation::windowstack::WindowStack;
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow},
};

use crate::{
    framework::button::{UiButton, UiButtonAddonBundle, UiButtonBundle, UiButtonEvent, UiButtonEventKind},
    util::irect_to_style,
};
use crate::{framework::svg::UiSvgBundle, prelude::*};

pub const WINDEOW_BASE_ZINDEX: i32 = 128;
pub const WINDEOW_MAX_STEP: i32 = 16;
pub const DECORATION_HEIGHT: f32 = 24.0;
pub const DECORATION_MARGIN: f32 = 2.0;

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

#[derive(Component, Reflect, Debug)]
pub struct WindowUI {
    window_entity: Entity,
    app_entry: Entity,
}
impl Default for WindowUI {
    fn default() -> Self {
        Self {
            window_entity: Entity::PLACEHOLDER,
            app_entry: Entity::PLACEHOLDER,
        }
    }
}
dway_widget! {
WindowUI=>
@plugin{
    app.register_type::<WindowUI>();
    app.add_systems(Update, attach_window);
}
@arg(asset_server: Res<AssetServer>)
@state_component(#[derive(Debug)])
@use_state(pub rect:IRect)
@use_state(pub bbox_rect:IRect)
@use_state(pub title:String)
@use_state(pub image:Handle<Image>)
@query(window_query:(rect,surface, toplevel)<-Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Ref<DWayToplevel>), With<DWayWindow>>[prop.window_entity]->{
    let init = !widget.inited;
    if init || rect.is_changed(){
        *state.rect_mut() = rect.geometry;
    }
    if init || rect.is_changed() || surface.is_changed() {
        *state.bbox_rect_mut() = surface.image_rect().offset(rect.pos());
    }
    if init || toplevel.is_changed(){ *state.title_mut() = toplevel.title.clone().unwrap_or_default(); }
    if init || surface.is_changed(){ *state.image_mut() = surface.image.clone(); }
})
@callback{ [UiButtonEvent]
    fn on_close_button_event(
        In(event): In<UiButtonEvent>,
        prop_query: Query<&WindowUI>,
        mut events: EventWriter<WindowAction>,
    ) {
        let Ok(prop) = prop_query.get(event.receiver)else{return;};
        match &event.kind{
            UiButtonEventKind::Released=>{
                events.send(WindowAction::Close(prop.window_entity));
            }
            _=>{}
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
        match &event.kind{
            UiButtonEventKind::Released=>{
                events.send(WindowAction::Minimize(prop.window_entity));
            }
            _=>{}
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
        match &event.kind{
            UiButtonEventKind::Released=>{
                events.send(WindowAction::Maximize(prop.window_entity));
            }
            _=>{}
        }
    }
}
<NodeBundle @style="absolute" >
    <MiniNodeBundle @id="content" Style=(irect_to_style(*state.rect()))
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
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/close.svg").into())) />
            </UiButtonBundle>
            <UiButtonBundle @id="max" @style="m-2 w-20 h-20"
                UiButtonAddonBundle=(UiButton::new(this_entity, on_max_button_event).into())
                @handle(UiCircleMaterial=>UiCircleMaterial::new(Color::WHITE*0.3, 8.0)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/maximize.svg").into())) />
            </UiButtonBundle>
            <UiButtonBundle @id="min" @style="m-2 w-20 h-20"
                UiButtonAddonBundle=(UiButton::new(this_entity, on_min_button_event).into())
                @handle(UiCircleMaterial=>UiCircleMaterial::new(Color::WHITE*0.3, 8.0)) >
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/minimize.svg").into())) />
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
</NodeBundle>
}

pub fn attach_window(
    mut commands: Commands,
    mut create_window_events: EventReader<Insert<DWayWindow>>,
    mut destroy_window_events: RemovedComponents<DWayWindow>,
    window_stack: Res<WindowStack>,
    mut ui_query: Query<(Entity, &mut WindowUI, &mut ZIndex)>,
) {
    if window_stack.is_changed()
        || !create_window_events.is_empty()
        || !destroy_window_events.is_empty()
    {
        let destroyed_windows: HashSet<_> = destroy_window_events.read().collect();
        let window_index_map: HashMap<_, _> = if window_stack.is_changed() {
            window_stack
                .list
                .iter()
                .enumerate()
                .map(|(i, e)| (*e, i))
                .collect()
        } else {
            HashMap::new()
        };
        create_window_events
            .read()
            .for_each(|Insert { entity, .. }| {
                commands.spawn((
                    Name::from("WindowUI"),
                    WindowUIBundle {
                        style: style!("absolute"),
                        prop: WindowUI {
                            window_entity: *entity,
                            app_entry: Entity::PLACEHOLDER,
                        },
                        ..Default::default()
                    },
                ));
            });
        ui_query.for_each_mut(|(entity, ui, mut z_index)| {
            if window_stack.is_changed() {
                if let Some(index) = window_index_map.get(&ui.window_entity) {
                    *z_index = ZIndex::Global(
                        WINDEOW_BASE_ZINDEX
                            + WINDEOW_MAX_STEP * (window_stack.list.len() - *index) as i32,
                    );
                }
            }
            if destroyed_windows.contains(&ui.window_entity) {
                commands.entity(entity).despawn_recursive();
            }
        })
    }
}
