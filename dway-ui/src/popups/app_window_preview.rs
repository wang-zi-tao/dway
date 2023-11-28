use std::{collections::BTreeMap, time::Duration};

use bevy_tweening::{Animator, EaseFunction, lens::*, Tween};
use bitflags::bitflags;
use dway_client_core::desktop::FocusedWindow;
use dway_server::{
    apps::{icon::Icon, WindowList},
    geometry::GlobalGeometry,
    wl::surface::WlSurface,
    xdg::toplevel::DWayToplevel,
};

use crate::{
    framework::{
        button::{UiButton, UiButtonAddonBundle, UiButtonBundle, UiButtonEvent, UiButtonEventKind},
        svg::UiSvgBundle,
    },
    prelude::*,
    widgets::{
        popup::{PopupState, UiPopup, UiPopupAddonBundle, UiPopupBundle, PopupUiSystems},
        window::create_window_material,
    },
};

#[derive(Component, Reflect)]
pub struct AppWindowPreviewPopup {
    pub app: Entity,
}
impl Default for AppWindowPreviewPopup {
    fn default() -> Self {
        Self {
            app: Entity::PLACEHOLDER,
        }
    }
}

pub const PREVIEW_HIGHT: f32 = 128.0;

dway_widget! {
AppWindowPreviewPopup=>
@callback{ [UiButtonEvent]
fn close_window(
    In(event): In<UiButtonEvent>,
    prop_query: Query<&AppWindowPreviewPopupSubStateList>,
    mut events: EventWriter<WindowAction>,
){
    let Ok(state) = prop_query.get(event.receiver)else{return;};
    if event.kind == UiButtonEventKind::Released{
        events.send(WindowAction::Close(*state.window_entity()));
    }
}}
@callback{ [UiButtonEvent]
fn focus_window(
    In(event): In<UiButtonEvent>,
    prop_query: Query<&AppWindowPreviewPopupSubStateList>,
    mut focused: ResMut<FocusedWindow>,
){
    let Ok(state) = prop_query.get(event.receiver)else{return;};
    if event.kind == UiButtonEventKind::Released{
        focused.window_entity = Some(*state.window_entity());
    }
}}
@bundle{{pub popup: UiPopupAddonBundle}}
@plugin{
    app.register_type::<AppWindowPreviewPopup>();
    app.configure_sets(Update, AppWindowPreviewPopupSystems::Render.before(PopupUiSystems::Close));
}
@arg(asset_server: Res<AssetServer>)
<RounndedRectBundle @style="flex-row m-4" @id="List"
    Animator<_>=(Animator::new(Tween::new(
        EaseFunction::BackOut,
        Duration::from_secs_f32(0.5),
        TransformScaleLens { start: Vec3::splat(0.5), end: Vec3::ONE, },
    )))
    @handle(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(Color::WHITE*0.2, 16.0))
    @use_state(windows: Vec<Entity>)
    @component(window_list<-Query<Ref<WindowList>>[prop.app]->{ state.set_windows(window_list.iter().collect()); })
    @for_query((window_entity,surface,geo,toplevel) in Query<(Entity,&WlSurface,&GlobalGeometry,&DWayToplevel)>::iter_many(state.windows().iter().cloned())) >
        <RounndedRectBundle @style="flex-col m-4" @id="window_preview"
            @use_state(title:String<=toplevel.title.clone().unwrap_or_default())
            @use_state(window_entity:Entity=Entity::PLACEHOLDER @ window_entity) >
            <NodeBundle @style="flex-row">
                <UiButtonBundle @id="close" @style="m-2 w-20 h-20"
                UiButtonAddonBundle=(UiButton::new(node!(window_preview), close_window).into())>
                    <(UiSvgBundle::new(asset_server.load("embedded://dway_ui/icons/close.svg").into())) />
                </UiButtonBundle>
                <TextBundle @style="items-center justify-center m-auto"
                    Text=(Text::from_section(
                        state.title(),
                        TextStyle {
                            font_size: 16.0,
                            color: Color::WHITE,
                            font: asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf"),
                        },
                    ).with_alignment(TextAlignment::Center))
                />
            </NodeBundle>
            <UiButtonBundle
            UiButtonAddonBundle=(UiButton::new(node!(window_preview), focus_window).into())>
                <MaterialNodeBundle::<RoundedUiImageMaterial>
                @handle(RoundedUiImageMaterial=>create_window_material(surface, geo))
                Style=({ let size = geo.size().as_vec2() * PREVIEW_HIGHT / geo.height() as f32;
                        Style{ width:Val::Px(size.x), height:Val::Px(size.y), ..default() } }) />
            </UiButtonBundle>
        </RounndedRectBundle>
</RounndedRectBundle>
}
