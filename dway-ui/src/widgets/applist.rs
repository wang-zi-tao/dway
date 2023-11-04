use std::{collections::BTreeMap, ops::Mul};

use bevy_svg::prelude::Svg;
use bevy_vector_shapes::{
    painter::CanvasBundle,
    prelude::*,
    shapes::{DiscPainter, RectPainter},
};
use dway_client_core::{desktop::FocusedWindow, navigation::windowstack::WindowStack};
use dway_server::apps::{
    icon::{Icon, IconLoader, IconResorce},
    AppEntryRoot, AppRef, DesktopEntry, ToplevelConnectAppEntry, WindowList,
};

use crate::{
    framework::{
        canvas::{UiCanvas, UiCanvasBundle, UiCanvasRenderCommand, UiCanvasSystems},
        icon::{uiicon_render, UiIcon},
        svg::{UiSvg, UiSvgBundle},
    },
    prelude::*,
};

#[derive(Component, Reflect)]
pub struct AppEntryUI(pub Entity);
dway_widget! {
AppEntryUI(
    mut app_query: Query<(Ref<WindowList>,Option<&mut Icon>)>,
    mut assets_server: ResMut<AssetServer>,
    mut icon_loader: ResMut<IconLoader>,
    mut svg_assets: ResMut<Assets<Svg>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
)#[derive(Default,Reflect)]{
    pub count: usize,
    pub icon: IconResorce,
    pub is_focused: bool,
}=>
{
    if let Ok(( window_list,icon )) = app_query.get_mut(prop.0){
        if let Some(mut icon) = icon{
            let icon = icon_loader.load(&mut icon, 48, &mut assets_server, &mut svg_assets, &mut mesh_assets).unwrap_or_default();
            update_state!(icon = icon);
        }
        update_state!(count = window_list.len());
    }
}
(focused_window:Res<FocusedWindow>){
    update_state!(is_focused = focused_window.app_entity == Some(prop.0));
}
<NodeBundle @style="w-full h-full flex-col absolute" >
    <ImageBundle @style="w-full h-full" UiIcon=(state.icon.clone().into())/>
    <NodeBundle Style=(Style{
        bottom:Val::Px(0.0),
        width:Val::Percent(((state.count as f32)/4.0).min(1.0)*80.0),
        height:Val::Px(2.0),
        position_type:PositionType::Absolute,
        align_self:AlignSelf::Center,
        ..default()})
        BackgroundColor=((if state.is_focused {Color::BLUE} else {Color::WHITE} ).into())
    />
</NodeBundle>
}

#[derive(Component, Reflect, Default)]
pub struct AppListUI {}

dway_widget! {
AppListUI(
    mut app_query: Query<(Entity,Ref<DesktopEntry>,Ref<WindowList>)>,
    window_index: Res<WindowStack>,
)#[derive(Default)]{
    pub list: BTreeMap<String,Entity>,
}=>
{
    if window_index.is_changed() {
        state_mut!(list).clear();
        app_query.for_each_mut(|(entry_entity,entry,window_list)|{
            if window_list.len()>0{
                state_mut!(list).insert(entry.appid.clone(),entry_entity);
            }
        });
    }
}
<UiCanvasBundle @id="list" @for(entry_entity in state.list.values()) @key(*entry_entity:Entity) >
    <(AppEntryUIBundle::new(AppEntryUI(*entry_entity),default())) @style="w-48 h-48 m-4 flex-col"/>
</UiCanvasBundle>
}

pub fn applist_canvas_render(
    canvas_query: Query<
        (
            &AppListUIWidget,
            &UiCanvas,
            &GlobalTransform,
            &UiCanvasRenderCommand,
        ),
        With<AppListUI>,
    >,
    mut painter: ShapePainter,
    mut sub_query: Query<(&GlobalTransform, &Node)>,
) {
    canvas_query.for_each(|(widgets, canvas, root_transform, render_command)| {
        canvas.setup_painter(render_command, &mut painter);
        let size = canvas.size();
        painter.color = Color::WHITE.with_a(0.5);
        painter.corner_radii = Vec4::splat(10.0);
        painter.rect(size);

        for (app_entity, element_entity) in widgets.node_list_entity_map.iter() {
            if let Ok((transform, node)) = sub_query.get(*element_entity) {
                painter.transform =
                    render_command.transform() * transform.reparented_to(root_transform);
                painter.color = Color::BLACK.with_a(0.4);
                painter.rect(node.size());
            }
        }
    });
}

#[derive(Bundle, Default)]
pub struct AppListPanelBundle {
    pub node: ImageBundle,
    pub canvas: UiCanvas,
    pub prop: AppListUI,
    pub state: AppListUIState,
    pub widget: AppListUIWidget,
}

pub struct AppListUIPlugin;
impl Plugin for AppListUIPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AppListUI>();
        app.register_type::<AppListUIWidget>();
        app.register_type::<AppEntryUI>();
        app.register_type::<AppEntryUIWidget>();
        app.register_type::<AppEntryUIState>();
        app.add_systems(
            Update,
            (
                applistui_render.in_set(AppListUISystems::Render),
                appentryui_render.in_set(AppEntryUISystems::Render),
                applist_canvas_render.after(UiCanvasSystems::Prepare),
            ),
        );
    }
}
