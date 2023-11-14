use bevy::utils::{HashMap, HashSet};
use dway_client_core::navigation::windowstack::WindowStack;
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow},
};

use crate::util::irect_to_style;
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
dway_widget! {
WindowUI(
    window_query: Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Ref<DWayToplevel>), With<DWayWindow>>,
    button_query: Query<&Interaction>,
    asset_server: Res<AssetServer>,
    mut window_action: EventWriter<WindowAction>,
    mut rect_materials: ResMut<Assets<RoundedUiRectMaterial>>,
    mut image_materials: ResMut<Assets<RoundedUiImageMaterial>>,
    mut circle_materials: ResMut<Assets<UiCircleMaterial>>,
)
#[derive(Reflect,Default)]{
    image: Handle<Image>,
    rect: IRect,
    bbox_rect: IRect,
    title: String,
}=>
{
    if let Ok((rect,surface, toplevel)) = window_query.get(prop.window_entity){
        if rect.is_changed(){
            update_state!(rect = rect.geometry);
        }
        if rect.is_changed() || surface.is_changed() {
            update_state!(bbox_rect = surface.image_rect().offset(rect.pos()));
        }
        if toplevel.is_changed(){
            update_state!(title = toplevel.title.clone().unwrap_or_default());
        }
        if surface.is_changed(){
            update_state!(image = surface.image.clone());
        }
    }
    if button_query.get(node!(close)).map(|e|*e==Interaction::Pressed).unwrap_or_default() {
        window_action.send(WindowAction::Close(prop.window_entity));
    }
    if button_query.get(node!(min)).map(|e|*e==Interaction::Pressed).unwrap_or_default() {
        window_action.send(WindowAction::Minimize(prop.window_entity));
    }
    if button_query.get(node!(max)).map(|e|*e==Interaction::Pressed).unwrap_or_default() {
        window_action.send(WindowAction::Maximize(prop.window_entity));
    }
}
<NodeBundle @style="absolute" >
    <MaterialNodeBundle::<RoundedUiRectMaterial> @id="outter"
        ZIndex=(ZIndex::Local(0))
        Style=(Style{
            position_type: PositionType::Absolute,
            left:Val::Px(state.rect.x() as f32 - DECORATION_MARGIN ),
            top:Val::Px(state.rect.y() as f32 - DECORATION_HEIGHT),
            width: Val::Px(state.rect.width() as f32 + 2.0*DECORATION_MARGIN),
            height: Val::Px(state.rect.height() as f32 + DECORATION_HEIGHT + DECORATION_MARGIN),
            ..Style::default() })
        Handle<RoundedUiRectMaterial>=(rect_materials.add(RoundedUiRectMaterial::new(Color::WHITE*0.2, 16.0)))
    />
    <MaterialNodeBundle::<RoundedUiImageMaterial> @id="content"
        ZIndex=(ZIndex::Local(1))
        Style=(irect_to_style(state.rect))
        Handle<RoundedUiImageMaterial>=(image_materials.add(RoundedUiImageMaterial::new(
            state.rect.size().as_vec2(),
            14.0,
            ( state.bbox_rect.min-state.rect.min ).as_vec2(),
            state.bbox_rect.size().as_vec2(),
            state.image.clone())))
    />
    <NodeBundle @id="bar"
        ZIndex=(ZIndex::Local(2))
        Style=(Style{
            position_type: PositionType::Absolute,
            left:Val::Px(state.rect.x() as f32),
            top:Val::Px(state.rect.y() as f32 - DECORATION_HEIGHT),
            width: Val::Px(state.rect.width() as f32),
            height: Val::Px(DECORATION_HEIGHT),
            ..Style::default() })
    >
        <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="close" @style="m-2 w-20 h-20" >
            <MaterialNodeBundle::<UiCircleMaterial>
                @style="m-2 w-16 h-16"
                Handle<UiCircleMaterial>=(circle_materials.add(UiCircleMaterial::new(Color::WHITE*0.3, 8.0)))>
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/close.svg").into())) />
            </UiMaterialBundle>
        </ButtonBundle>
        <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="max" @style="m-2 w-20 h-20" >
            <MaterialNodeBundle::<UiCircleMaterial>
                @style="m-2 w-16 h-16"
                Handle<UiCircleMaterial>=(circle_materials.add(UiCircleMaterial::new(Color::WHITE*0.3, 8.0)))>
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/maximize.svg").into())) />
            </UiMaterialBundle>
        </ButtonBundle>
        <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="min" @style="m-2 w-20 h-20" >
            <MaterialNodeBundle::<UiCircleMaterial>
                @style="m-2 w-16 h-16"
                Handle<UiCircleMaterial>=(circle_materials.add(UiCircleMaterial::new(Color::WHITE*0.3, 8.0)))>
                <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/minimize.svg").into())) />
            </MaterialNodeBundle>
        </ButtonBundle>
        <TextBundle @style="items-center justify-center m-auto"
            Text=(Text::from_section(
                &state.title,
                TextStyle {
                    font_size: DECORATION_HEIGHT - 2.0,
                    color: Color::WHITE,
                    font: asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf"),
                },
            ).with_alignment(TextAlignment::Center))
        />
    </NodeBundle>
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
        let destroyed_windows: HashSet<_> = destroy_window_events.iter().collect();
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
            .iter()
            .for_each(|Insert { entity, .. }| {
                commands.spawn(WindowUIBundle {
                    node: NodeBundle {
                        style: style!("absolute"),
                        ..NodeBundle::default()
                    },
                    prop: WindowUI {
                        window_entity: *entity,
                        app_entry: Entity::PLACEHOLDER,
                    },
                    state: WindowUIState::default(),
                    widget: WindowUIWidget::default(),
                });
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

pub struct WindowUIPlugin;
impl Plugin for WindowUIPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WindowUI>();
        app.register_type::<WindowUIWidget>();
        app.register_type::<WindowUIState>();
        app.add_systems(Update, windowui_render.in_set(WindowUISystems::Render));
        app.add_systems(Update, attach_window);
    }
}
