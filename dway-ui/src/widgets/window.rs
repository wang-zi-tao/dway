use bevy::{
    transform::commands,
    utils::{HashMap, HashSet},
};
use bevy_svg::prelude::{Origin, Svg, Svg2dBundle};
use bevy_vector_shapes::{prelude::ShapePainter, shapes::*};
use dway_client_core::navigation::windowstack::WindowStack;
use dway_server::{
    geometry::GlobalGeometry,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::DWayToplevel, DWayWindow, PopupList},
};
use dway_util::temporary::TemporaryEntity;

use crate::{
    default_system_font,
    framework::canvas::{UiCanvasBundle, UiCanvasSystems},
    prelude::*,
};
use crate::{
    framework::canvas::{UiCanvas, UiCanvasRenderCommand},
    util::irect_to_style,
};

pub const WINDEOW_MAX_INDEX: i32 = 0;
pub const WINDEOW_MAX_STEP: i32 = 64;
pub const DECORATION_HEIGHT: f32 = 24.0;

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
<NodeBundle @style="absolute">
    <ImageBundle @id="bbox" UiImage=(UiImage::new(state.image.clone())) Style=(irect_to_style(state.bbox_rect))>
        <NodeBundle @id="content" Style=(irect_to_style(state.rect))/>
    </ImageBundle>
    <UiCanvasBundle @id="canvas" Style=(Style{
        position_type: PositionType::Absolute,
        left:Val::Px(state.rect.x() as f32 ),
        top:Val::Px(state.rect.y() as f32 - DECORATION_HEIGHT),
        width: Val::Px(state.rect.width() as f32),
        height: Val::Px(state.rect.height() as f32 + DECORATION_HEIGHT),
        ..Style::default()
    })/>
    <NodeBundle @id="bar"
        Style=(Style{
            position_type: PositionType::Absolute,
            left:Val::Px(state.rect.x() as f32 ),
            top:Val::Px(state.rect.y() as f32 - DECORATION_HEIGHT),
            width: Val::Px(state.rect.width() as f32),
            height: Val::Px(DECORATION_HEIGHT),
            ..Style::default()
    })>
        <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="close" @style="m-4 w-16 h-16"/>
        <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="max" @style="m-4 w-16 h-16"/>
        <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="min" @style="m-4 w-16 h-16"/>
        <TextBundle @style="items-center justify-center m-auto"
            Text=(Text::from_section(
                &state.title,
                TextStyle {
                    font_size: DECORATION_HEIGHT - 2.0,
                    color: Color::WHITE,
                    font: asset_server.load("fonts/SmileySans-Oblique.ttf"),
                },
            ).with_alignment(TextAlignment::Center))
        />
    </NodeBundle>
</NodeBundle>
}

pub fn window_canvas_render(
    mut window_query: Query<
        (&WindowUIWidget, &WindowUIState),
        // Or<(Changed<WindowUI>, Changed<WindowUIState>)>,
    >,
    canvas_query: Query<(&UiCanvas, &GlobalTransform, &UiCanvasRenderCommand)>,
    mut widget_query: Query<(&GlobalTransform, &Node)>,
    mut painter: ShapePainter,
    resources: Res<WindowUiResources>,
    mut commands: Commands,
) {
    window_query.for_each_mut(|(widget, state)| {
        if let Ok((canvas, root_transform, render_command)) =
            canvas_query.get(widget.node_canvas_entity)
        {
            canvas.setup_painter(render_command, &mut painter);
            painter.transform.scale.y = -1.0;
            painter.color = Color::WHITE * 0.2;
            painter.corner_radii = Vec4::splat(16.0);
            painter.thickness = 2.0;
            painter.hollow = true;
            painter.rect(canvas.size());
            painter.hollow = false;

            let base_transform = painter.transform;

            if let Ok((transform, node)) = widget_query.get(widget.node_content_entity) {
                painter.transform = base_transform * transform.reparented_to(root_transform);
                painter.color = Color::RED.with_a(0.1);
                painter.corner_radii = Vec4::new(16.0, 16.0, 0.0, 0.0);
                // painter.rect(node.size());
            }
            if let Ok((transform, node)) = widget_query.get(widget.node_bar_entity) {
                painter.transform = base_transform * transform.reparented_to(root_transform);
                painter.color = Color::WHITE * 0.2;
                painter.corner_radii = Vec4::new(16.0, 16.0, 0.0, 0.0);
                painter.rect(node.size());
            }
            for (i, (transform, node)) in widget_query
                .get_many([
                    widget.node_close_entity,
                    widget.node_max_entity,
                    widget.node_min_entity,
                ])
                .iter()
                .flatten()
                .enumerate()
            {
                painter.transform = base_transform * transform.reparented_to(root_transform);
                painter.color = match i {
                    0 => Color::RED,
                    1 => Color::ORANGE,
                    2 => Color::GREEN,
                    _ => Color::GRAY,
                };
                painter.corner_radii = Vec4::splat(8.0);
                painter.rect(node.size());
                let svg = match i {
                    0 => resources.close_icon.clone(),
                    1 => resources.maximize_icon.clone(),
                    2 => resources.minimize_icon.clone(),
                    _ => continue,
                };
                commands.spawn((
                    Svg2dBundle {
                        svg,
                        origin: Origin::Center,
                        transform: painter.transform
                            * Transform::default()
                                .with_translation(Vec3::new(-1.0, 1.0, 1.0) * 8.0 * 0.8)
                            * Transform::default()
                                .with_scale(Vec3::new(1.0, -1.0, 1.0) * (2.0 * 8.0 * 0.8 / 960.0)),
                        ..default()
                    },
                    TemporaryEntity,
                ));
            }
        }
    });
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
                commands.entity(entity).despawn_recursive();
            }
        })
    }
}

#[derive(Resource)]
pub struct WindowUiResources {
    pub font: Handle<Font>,
    pub close_icon: Handle<Svg>,
    pub maximize_icon: Handle<Svg>,
    pub minimize_icon: Handle<Svg>,
}

pub struct WindowUIPlugin;
impl Plugin for WindowUIPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WindowUI>();
        app.register_type::<WindowUIWidget>();
        app.register_type::<WindowUIState>();
        app.add_systems(
            Update,
            (
                windowui_render,
                window_canvas_render.after(UiCanvasSystems::Prepare),
            )
                .in_set(WindowUISystems::Render),
        );
        app.add_systems(Update, attach_window);

        let asset_server = app.world.resource_mut::<AssetServer>();
        let resources = WindowUiResources {
            font: asset_server.load("fonts/SmileySans-Oblique.ttf"),
            close_icon: asset_server.load("icons/close.svg"),
            maximize_icon: asset_server.load("icons/maximize.svg"),
            minimize_icon: asset_server.load("icons/minimize.svg"),
        };
        app.insert_resource(resources);
    }
}
