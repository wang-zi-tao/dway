use std::collections::HashSet;

use bevy::{
    camera::RenderTarget,
    color::palettes::{css::*, tailwind::CYAN_50},
    ecs::{archetype::Archetype, system::SystemIdMarker},
    prelude::*,
    ui::{RelativeCursorPosition, UiStack, UiSystems},
    window::PrimaryWindow,
};
use bevy_inspector_egui::{
    bevy_egui::{
        input::WindowToEguiContextMap, EguiContext, EguiInputSet, EguiPlugin, EguiPostUpdateSet,
        EguiPreUpdateSet, EguiPrimaryContextPass, PrimaryEguiContext,
    },
    bevy_inspector::{
        ui_for_entities_filtered, ui_for_entity_with_children, ui_for_world,
        ui_for_world_entities_filtered, Filter,
    },
    egui,
};
use egui_dock::{egui::Color32, DockArea, DockState};

use crate::{
    prelude::*,
    render::layer_manager::{after_ui_focus_system, before_ui_focus_system},
    shader::{effect::Border, shape::RoundedRect},
    UiFrameworkSystems,
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
            .add_systems(Update, open_close_inspector)
            // --- ordering against the dway UI framework / LayerManager ----------------
            //
            // LayerManager keeps every managed camera pointed at its layer image and
            // only swaps the real window target in for the duration of
            // UiSystems::Focus (before_ui_focus_system .. after_ui_focus_system, see
            // render/layer_manager.rs). bevy_egui reads that target, so its
            // context/input systems have to be pinned to the same window.
            .add_systems(
                PreUpdate,
                bind_egui_context_to_window
                    .run_if(|ui_state: Res<InspectorUiState>| ui_state.enable)
                    .in_set(UiSystems::Focus)
                    .after(before_ui_focus_system)
                    .before(after_ui_focus_system),
            )
            .configure_sets(
                PreUpdate,
                // Creates the primary context, binds it to a window by sampling the
                // camera's RenderTarget (WindowToEguiContextMap) and derives
                // screen_rect/pixels_per_point from the camera target. Run it while
                // LayerManager exposes the window target, otherwise the context is born
                // as a *non-window* context: no window input, and egui is painted into
                // whichever layer image the camera happened to point at.
                EguiPreUpdateSet::InitContexts
                    .after(before_ui_focus_system)
                    .before(after_ui_focus_system),
            )
            .configure_sets(
                PreUpdate,
                (
                    EguiInputSet::InitReading,
                    EguiInputSet::FocusContext,
                    EguiInputSet::ReadBevyMessages,
                    EguiInputSet::WriteEguiEvents,
                )
                    .chain()
                    // Read egui's input in the same world state as the framework's own
                    // input systems: after UiSystems::Focus (LayerManager has restored
                    // the layer targets and the binding installed by
                    // bind_egui_context_to_window is visible) and after
                    // UiFrameworkSystems::InputSystems, so that
                    // absorb_bevy_input_system cannot drain Bevy's input before dway's
                    // widgets and the compositor cursor have seen it.
                    .after(UiSystems::Focus)
                    .after(UiFrameworkSystems::InputSystems),
            )
            .configure_sets(
                PostUpdate,
                (
                    EguiPostUpdateSet::EndPass,
                    EguiPostUpdateSet::ProcessOutput,
                    EguiPostUpdateSet::PostProcessOutput,
                )
                    .chain()
                    // EguiPrimaryContextPass can spawn/despawn bevy_ui nodes (the
                    // inspector picker overlay), so it belongs into the same
                    // "before layout" bucket as the framework's widget sets, and it must
                    // not mutate the UI tree while update_layers snapshots it.
                    .before(UiSystems::Layout)
                    .before(UiFrameworkSystems::UpdateLayers),
            );
    }
}

/// Keeps the egui context -> window binding in sync with the LayerManager.
///
/// bevy_egui samples the camera's RenderTarget exactly once, when the context is
/// created (WindowToEguiContextMap). LayerManager exposes the real window target
/// only while UiSystems::Focus runs, and only from the first update_layers pass
/// onwards, so a context created for a camera that already renders into a layer (the
/// TTY/DRM output is an image target) would stay unbound forever: egui then receives
/// no window input and EguiInput::focused stays false. Re-assert the binding while
/// the window target is exposed.
fn bind_egui_context_to_window(
    mut map: ResMut<WindowToEguiContextMap>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    contexts: Query<(Entity, &RenderTarget), With<EguiContext>>,
) {
    let primary_window = primary_window.iter().next();
    for (context, target) in &contexts {
        let RenderTarget::Window(window_ref) = target else {
            continue;
        };
        let Some(window) = window_ref.normalize(primary_window).map(|w| w.entity()) else {
            continue;
        };
        if map.context_to_window.get(&context) == Some(&window) {
            continue;
        }
        map.context_to_window.insert(context, window);
        map.window_to_contexts
            .entry(window)
            .or_default()
            .insert(context);
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

                let mut query = self.world.query::<(Entity, &Archetype)>();
                let entitys = query
                    .iter(self.world)
                    .filter(|(_, archetype)| archetypes.contains(&archetype.id()))
                    .map(|(e, _)| e)
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
        (Entity, &ComputedNode, &UiGlobalTransform),
        (With<Node>, Without<PickerUiNode>),
    >,
    mut state: Query<(
        &mut UiPickingState,
        &UiGlobalTransform,
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
        UiInputEvent::MouseMove(delta) => {
            let inspector_rect = get_node_rect(widget_transform, widget_node);
            let mouse_position = inspector_rect.min + delta;

            let mut rects = Vec::new();
            let mut entitys = Vec::new();

            for (entity, node, transform) in node_query.iter_many(&node_stack.uinodes) {
                let node_rect = get_node_rect(transform, node);
                if node_rect.contains(mouse_position) {
                    rects.push(node_rect);
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
