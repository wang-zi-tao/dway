use std::collections::HashSet;

use bevy::{
    color::palettes::{css::*, tailwind::CYAN_50},
    ecs::system::SystemIdMarker,
    prelude::*,
    ui::{RelativeCursorPosition, UiStack},
};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector::{
        ui_for_entities_filtered, ui_for_entity_with_children, ui_for_world,
        ui_for_world_entities_filtered, Filter,
    },
    egui,
};
use egui_dock::{egui::Color32, DockArea, DockState};

use crate::{
    prelude::*,
    shader::{effect::Border, shape::RoundedRect},
};

#[derive(Debug, Clone, Default)]
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin::default());
        }

        app.add_plugins(bevy_inspector_egui::DefaultInspectorConfigPlugin)
            .add_plugins(UiPickingPlugin)
            .add_plugins(PickerMaterial::plugin())
            .init_resource::<InspectorSetting>()
            .init_resource::<InspectorUiState>()
            .add_systems(
                EguiPrimaryContextPass,
                inspector_ui.run_if(|ui_state: Res<InspectorUiState>| ui_state.enable),
            )
            .add_systems(Update, open_close_inspector);
    }
}

#[derive(Resource, SmartDefault, Clone)]
pub struct InspectorSetting {
    #[default(Some(KeyCode::F12))]
    pub key: Option<KeyCode>,
    #[default(4096)]
    pub zindex: i32,
}

structstruck::strike! {
    #[derive(Resource)]
    pub struct InspectorUiState {
        pub enable: bool,
        dock_state: DockState<UiTab>,
        data: struct {
            entity_bookmarks: Vec<Entity>,
            picking: struct PickingData {
                picking: bool,
                ui: Vec<Entity>,
                ui_inited:bool,
                node_stack: Vec<Entity>,
            },
            search: struct SearchData {
                text: String,
            }
        },
    }
}

impl Default for InspectorUiState {
    fn default() -> Self {
        Self {
            enable: true,
            dock_state: DockState::new(vec![
                UiTab::Pick,
                UiTab::Search,
                UiTab::World,
                UiTab::BookMarks,
            ]),
            data: Data {
                entity_bookmarks: Vec::new(),
                picking: PickingData {
                    picking: false,
                    node_stack: Vec::new(),
                    ui: Vec::new(),
                    ui_inited: false,
                },
                search: SearchData {
                    text: String::new(),
                },
            },
        }
    }
}

enum UiTab {
    World,
    BookMarks,
    Pick,
    Search,
}

struct InspectorTabViewer<'l> {
    world: &'l mut World,
    data: &'l mut Data,
}

impl<'l> egui_dock::TabViewer for InspectorTabViewer<'l> {
    type Tab = UiTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            UiTab::World => "World".into(),
            UiTab::BookMarks => "Bookmarks".into(),
            UiTab::Pick => "Pick".into(),
            UiTab::Search => "Search".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let settings = self.world.resource::<InspectorSetting>().clone();

        match tab {
            UiTab::World => {
                ui_for_entities_filtered(
                    self.world,
                    ui,
                    true,
                    &Filter::<(Without<ChildOf>, Without<SystemIdMarker>, Without<Observer>)>::all(
                    ),
                );
            }
            UiTab::BookMarks => {
                for entity in self.data.entity_bookmarks.clone() {
                    ui_for_entity(self.world, entity, ui, self.data, true);
                }
            }
            UiTab::Pick => {
                let data = &mut self.data.picking;
                let width = ui.available_width();
                if ui
                    .add_sized((width, 32.0), egui::Button::new("Pick Node"))
                    .clicked()
                {
                    data.picking = true;

                    if !data.ui_inited {
                        let mut query = self.world.query_filtered::<Entity, With<Camera2d>>();
                        let cameras = query.iter(self.world).collect::<Vec<Entity>>();
                        for camera in cameras {
                            let entity = self
                                .world
                                .spawn((
                                    style!("absolute full"),
                                    UiPicking {},
                                    UiTargetCamera(camera),
                                    Name::new("Inspector Picker"),
                                    GlobalZIndex(settings.zindex),
                                ))
                                .id();
                            data.ui.push(entity);
                        }
                        data.ui_inited = true;
                    }
                }

                for (index, entity) in self.data.picking.node_stack.iter().enumerate().rev() {
                    let color = picker_colors(index).to_u8_array();
                    let name = entity_name(self.world, *entity);

                    egui::Frame::default()
                        .stroke(egui::Stroke::new(
                            1.0,
                            Color32::from_rgb(color[0], color[1], color[2]),
                        ))
                        .corner_radius(ui.visuals().widgets.noninteractive.corner_radius)
                        .outer_margin(1.0)
                        .inner_margin(1.0)
                        .show(ui, |ui| {
                            if ui.button("+").clicked()
                                && !self.data.entity_bookmarks.contains(entity)
                            {
                                self.data.entity_bookmarks.push(*entity);
                            }

                            ui.collapsing(&name, |ui| {
                                ui_for_entity_with_children(self.world, *entity, ui);
                            });
                        });
                }
            }
            UiTab::Search => {
                let data = &mut self.data.search;
                let size = (ui.available_width(), 24.0);
                ui.add_sized(size, egui::TextEdit::singleline(&mut data.text));

                let components = self
                    .world
                    .components_registrator()
                    .iter_registered()
                    .filter(|component| component.name().contains(&data.text))
                    .map(|c| c.id())
                    .collect::<HashSet<_>>();

                let archetypes = self
                    .world
                    .archetypes()
                    .iter()
                    .filter(|archetype| {
                        archetype
                            .components()
                            .iter()
                            .any(|comp| components.contains(&comp))
                    })
                    .map(|a| a.id())
                    .collect::<HashSet<_>>();

                let entitys = self
                    .world
                    .iter_entities()
                    .filter(|entity| archetypes.contains(&entity.archetype().id()))
                    .map(|e| e.id())
                    .collect::<Vec<_>>();

                for entity in entitys {
                    ui_for_entity(self.world, entity, ui, self.data, false);
                }
            }
        }
    }
}

fn ui_for_entity(
    world: &mut World,
    entity: Entity,
    ui: &mut egui::Ui,
    data: &mut Data,
    in_bookmark: bool,
) {
    egui::Frame::default()
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(ui.visuals().widgets.noninteractive.corner_radius)
        .outer_margin(1.0)
        .inner_margin(1.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if in_bookmark {
                    if ui.button("remove bookmark").clicked() {
                        data.entity_bookmarks.retain(|e| *e != entity);
                    }
                } else if ui.button("add bookmark").clicked()
                    && !data.entity_bookmarks.contains(&entity)
                {
                    data.entity_bookmarks.push(entity);
                }

                if ui.button("despawn").clicked() {
                    world.entity_mut(entity).despawn();
                    data.entity_bookmarks.retain(|e| *e != entity);
                }
            });

            let name = entity_name(world, entity);
            ui.collapsing(name, |ui| {
                ui_for_entity_with_children(world, entity, ui);
            });
        });
}

fn entity_name(world: &World, entity: Entity) -> String {
    if let Ok(entity_ref) = world.get_entity(entity) {
        if let Some(name) = entity_ref.get::<Name>() {
            name.as_str().to_owned()
        } else if entity_ref.get::<Node>().is_some() {
            format!("Node({entity})")
        } else if entity_ref.get::<Camera>().is_some() {
            format!("Camera({entity})")
        } else {
            format!("Entity({entity})")
        }
    } else {
        format!("Entity Not Found({entity})")
    }
}

pub fn inspector_ui(world: &mut World) -> Result {
    world.resource_scope::<InspectorUiState, Result>(|world, mut ui_state| {
        let InspectorUiState {
            enable,
            dock_state,
            data,
        } = &mut *ui_state;

        if !*enable {
            return Ok(());
        }

        let ctx = world
            .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
            .single(world)?;

        let mut egui_context = ctx.clone();

        egui::Window::new("Inspector")
            .resizable(true)
            .collapsible(true)
            .default_width(300.0)
            .default_height(800.0)
            .show(egui_context.get_mut(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut tabs = InspectorTabViewer { world, data };

                    DockArea::new(dock_state).show_inside(ui, &mut tabs);
                });
            });

        Ok(())
    })
}

pub fn open_close_inspector(
    setting: Res<InspectorSetting>,
    mut ui_state: ResMut<InspectorUiState>,
    keys: ResMut<ButtonInput<KeyCode>>,
) {
    if let Some(key) = setting.key {
        if keys.just_pressed(key) {
            ui_state.enable = !ui_state.enable;
        }
    }
}

#[derive(Component)]
#[require(PickerUiNode)]
struct UiPicking {}

#[derive(Component, Default)]
struct PickerUiNode;

fn on_mouse_event(
    event: UiEvent<UiInputEvent>,
    mut inspector_ui_state: ResMut<InspectorUiState>,
    node_query: Query<
        (Entity, &ComputedNode, &GlobalTransform),
        (With<Node>, Without<PickerUiNode>),
    >,
    mut state: Query<(
        &mut UiPickingState,
        &GlobalTransform,
        &ComputedNode,
        &ComputedUiTargetCamera,
    )>,
    node_stack: Res<UiStack>,
) {
    let Ok((mut state, widget_transform, widget_node, widget_camera)) =
        state.get_mut(event.receiver())
    else {
        return;
    };

    if !inspector_ui_state.data.picking.picking {
        return;
    }

    match event.event() {
        UiInputEvent::MousePress(MouseButton::Left) => {
            if state.enable {
                inspector_ui_state.data.picking.picking = false;
            }
        }
        UiInputEvent::MouseMove(normaled) => {
            let rect =
                Rect::from_center_size(widget_transform.translation().xy(), widget_node.size());
            let mouse_position = rect.min + rect.size() * normaled;

            let mut rects = Vec::new();
            let mut entitys = Vec::new();

            for (entity, node, transform) in node_query.iter_many(&node_stack.uinodes) {
                let rect = Rect::from_center_size(transform.translation().xy(), node.size());
                if rect.contains(mouse_position) {
                    rects.push(rect);
                    entitys.push(entity);
                }
            }

            state.set_areas(rects);
            inspector_ui_state.data.picking.node_stack = entitys;
        }
        _ => {}
    }
}

type PickerMaterial = ShaderAsset<ShapeRender<RoundedRect, Border>>;

fn picker_colors(index: usize) -> Srgba {
    let colors = [RED, GREEN, BLUE, YELLOW, ORANGE, PINK, PURPLE, CYAN_50];
    colors[index % colors.len()]
}

fn picker_material(index: usize) -> PickerMaterial {
    ShapeRender::new(
        RoundedRect::new(4.0),
        Border::new(Color::Srgba(picker_colors(index)), 2.0),
    )
    .into()
}

dway_widget! {
UiPicking=>
@use_state(enable: bool)
@use_state(picking: bool)
@use_state(areas: Vec<Rect>)
@add_callback([UiEvent<UiInputEvent>]on_mouse_event)
@state_reflect()
@global(setting: InspectorSetting)
@global(mut ui_state: InspectorUiState -> {
    if ui_state.enable != *state.enable(){
        state.set_enable(ui_state.enable);
    }
    if ui_state.data.picking.picking != *state.picking(){
        state.set_picking(ui_state.data.picking.picking);
    }
})
<Node PickerUiNode=(default())
    FocusPolicy=(if *state.picking() {FocusPolicy::Block} else {FocusPolicy::Pass})
    @style="absolute top-0 left-0 w-full h-full"
    @if(state.enable)>
    <UiInput PickerUiNode=(default()) RelativeCursorPosition=(default())
        @style="absolute top-0 left-0 w-full h-full"
        @on_event(on_mouse_event)
        @for((index,area): (usize, &Rect) in state.areas().iter().enumerate() => {
            state.set_index(index);
            state.set_area(*area);
        })>
        <(Node{
            left: Val::Px(state.area().min.x),
            top: Val::Px(state.area().min.y),
            width: Val::Px(state.area().width()),
            height: Val::Px(state.area().height()),
            position_type: PositionType::Absolute,
            ..default()
        }) PickerUiNode=(default())
        @material(PickerMaterial=>picker_material(*state.index()))
        @use_state(index: usize)
        @use_state(area: Rect)
        >
        </Node>
    </UiInput>
</Node>
}
