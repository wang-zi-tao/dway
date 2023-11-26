use std::collections::BTreeMap;

use bevy_svg::prelude::Svg;
use dway_client_core::{desktop::FocusedWindow, navigation::windowstack::WindowStack};
use dway_server::apps::{
    icon::{Icon, IconLoader, IconResorce},
    DesktopEntry, WindowList,
};

use crate::{
    framework::icon::UiIcon,
    // popups::app_window_preview::{AppWindowPreviewPopupSystems, OpenAppWindowPreviewPopup},
    prelude::*,
};

use super::popup::UiPopup;

#[derive(Component, Reflect)]
pub struct AppEntryUI(pub Entity);

#[derive(Component, Reflect, Default)]
pub struct AppListUI {}

dway_widget! {
AppListUI=>
@bundle{{pub node:MiniNodeBundle}}
@arg(mut icon_loader: ResMut<IconLoader>)
@arg(mut svg_assets: ResMut<Assets<Svg>>)
@arg(mut mesh_assets: ResMut<Assets<Mesh>>)
@arg(mut icon_loader: ResMut<IconLoader>)
@arg(mut assets_server: ResMut<AssetServer>)
<MaterialNodeBundle::<RoundedUiRectMaterial> @id="list"
    @handle(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(Color::WHITE.with_a(0.5), 16.0))
    @for_query(mut(entity,desktop,window_list,mut icon) in Query<(Entity,&DesktopEntry,&WindowList,Option<&mut Icon>)>::iter_mut()) >
    <NodeBundle @if(window_list.len()>0) @id="app_root" >
        <RounndedRectBundle @style="w-48 h-48 m-4 flex-col" @id="app_rect"
            @use_state(pub count:usize <= window_list.len())
            @use_state(pub icon:IconResorce <= icon.as_mut().and_then(|icon| icon_loader.load(icon, 48, &mut assets_server, &mut svg_assets, &mut mesh_assets) ).unwrap_or_default())
            @use_state(pub is_focused:bool=false)
            @arg(focused_window: ResMut<FocusedWindow> => {
                state.set_is_focused(focused_window.app_entity == Some(entity));
            })
            @handle(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(Color::WHITE.with_a(0.4), 10.0)) >
            <ButtonBundle BackgroundColor=(Color::NONE.into()) @id="button" @style="absolute full flex-col" >
                <ImageBundle @style="w-full h-full" UiIcon=(state.icon().clone().into()) @id="app_icon" />
                <NodeBundle @id="focus_mark" Style=(Style{
                        width:Val::Percent(((*state.count() as f32)/4.0).min(1.0)*80.0),
                    ..style!("absolute bottom-0 h-2 align-center")})
                    BackgroundColor=((if *state.is_focused() {Color::BLUE} else {Color::WHITE} ).into())
                />
            </ButtonBundle>
        </>
    </NodeBundle>
</>
}
