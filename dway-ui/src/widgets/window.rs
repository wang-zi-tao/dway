use bevy::utils::{HashMap, HashSet};
use dway_client_core::navigation::windowstack::WindowStack;
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow, PopupList},
};

use crate::util::irect_to_style;
use crate::{default_system_font, prelude::*};

pub const WINDEOW_MAX_INDEX: i32 = 0;
pub const WINDEOW_MAX_STEP: i32 = 64;

#[derive(Component, Reflect, Debug)]
pub struct WindowUI {
    window_entity: Entity,
    app_entry: Entity,
}

dway_widget! {
WindowUI(
    window_query: Query<(Ref<GlobalGeometry>, Ref<WlSurface>, Ref<DWayToplevel>), With<DWayWindow>>,
    asset_server: Res<AssetServer>,
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
}
<NodeBundle @style="absolute">
    <ImageBundle UiImage=(UiImage::new(state.image.clone())) Style=(irect_to_style(state.bbox_rect))>
        <NodeBundle Style=(irect_to_style(state.rect))/>
    </ImageBundle>
    <NodeBundle Style=(Style{
            position_type: PositionType::Absolute,
            left:Val::Px( state.rect.x() as f32 ),
            top:Val::Px( ( state.rect.y() - 16 ) as f32 ),
            width: Val::Px(state.rect.width() as f32),
            height: Val::Px(16.0),
            ..Style::default()
        })
        BackgroundColor=(BackgroundColor::from(Color::WHITE))
    >
        <TextBundle @style="items-center justify-center m-auto"
            Text=(Text::from_section(
                &state.title,
                TextStyle {
                    font_size: 20.0,
                    color: Color::BLACK,
                    ..default()
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
                        style: styled!("absolute"),
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
                    *z_index =
                        ZIndex::Global(WINDEOW_MAX_INDEX - WINDEOW_MAX_STEP * (*index as i32));
                }
            }
            if destroyed_windows.contains(&ui.window_entity) {
                commands
                    .entity(entity)
                    .despawn_recursive_with_relationship();
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
