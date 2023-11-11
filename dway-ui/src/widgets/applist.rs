use std::collections::BTreeMap;

use bevy_svg::prelude::Svg;
use dway_client_core::{desktop::FocusedWindow, navigation::windowstack::WindowStack};
use dway_server::apps::{
    icon::{Icon, IconLoader, IconResorce},
    DesktopEntry, WindowList,
};

use crate::{
    framework::icon::UiIcon,
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
    mut material_set: ResMut<Assets<RoundedUiRectMaterial>>,
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
<MaterialNodeBundle::<RoundedUiRectMaterial> @style="w-full h-full flex-col absolute"
        Handle<RoundedUiRectMaterial>=(material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.4), 10.0))) >
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
    mut material_set: ResMut<Assets<RoundedUiRectMaterial>>,
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
<MaterialNodeBundle::<RoundedUiRectMaterial> @id="list"
    Handle<RoundedUiRectMaterial>=(material_set.add(RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5), 16.0)))
    @for(entry_entity in state.list.values()) @key(*entry_entity:Entity) >
    <(AppEntryUIBundle::new(AppEntryUI(*entry_entity),default())) @style="w-48 h-48 m-4 flex-col"/>
</NodeBundle>
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
            ),
        );
    }
}
