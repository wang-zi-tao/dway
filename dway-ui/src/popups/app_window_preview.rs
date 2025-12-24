use dway_client_core::desktop::FocusedWindow;
use dway_server::{
    apps::WindowList, geometry::GlobalGeometry, util::rect::IRect, wl::surface::WlSurface,
    xdg::toplevel::DWayToplevel,
};
use dway_ui_framework::theme::NoTheme;

use crate::{prelude::*, widgets::window::create_raw_window_material};

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
@callback{ [UiEvent<UiButtonEvent>]
fn close_window(
    event: UiEvent<UiButtonEvent>,
    prop_query: Query<&AppWindowPreviewPopupSubWidgetList>,
    mut events: MessageWriter<WindowAction>,
){
    let Ok(widget) = prop_query.get(event.receiver())else{return;};
    if event.kind == UiButtonEventKind::Released{
        events.write(WindowAction::Close(widget.data_entity));
    }
}}
@callback{ [UiEvent<UiButtonEvent>]
fn focus_window(
    event: UiEvent<UiButtonEvent>,
    prop_query: Query<&AppWindowPreviewPopupSubWidgetList>,
    mut focused: ResMut<FocusedWindow>,
){
    let Ok(widget) = prop_query.get(event.receiver())else{return;};
    if event.kind == UiButtonEventKind::Released{
        focused.window_entity = Some(widget.data_entity);
    }
}}
@plugin{
    app.register_type::<AppWindowPreviewPopup>();
    app.configure_sets(Update, AppWindowPreviewPopupSystems::Render.before(UiFrameworkSystems::UpdatePopup));
}
@global(theme: Theme)
@arg(asset_server: Res<AssetServer>)
@use_state(windows: Vec<Entity>)
@component(window_list<-Query<Ref<WindowList>>[prop.app]->{ state.set_windows(window_list.iter().collect()); })
<Node @style="flex-row m-4" @id="List"
    // Animator<_>=(Animator::new(Tween::new(
    //     EaseFunction::BackOut,
    //     Duration::from_secs_f32(0.5),
    //     TransformScaleLens { start: Vec3::splat(0.5), end: Vec3::ONE, },
    // )))
    @for_query((surface,geo,toplevel) in Query<(Ref<WlSurface>,Ref<GlobalGeometry>,Ref<DWayToplevel>)>::iter_many(state.windows().iter().cloned()) =>[
        toplevel=>{state.set_title(toplevel.title.clone().unwrap_or_default());},
        geo=>{state.set_geo(geo.clone());},
        surface=>{
            state.set_image(surface.image.clone());
            state.set_image_rect(surface.image_rect());
        }
    ]) >
        <Node @style="flex-col m-4" @id="window_preview"
            @use_state(title:String) @use_state(geo:GlobalGeometry) @use_state(image:Handle<Image>) @use_state(image_rect:IRect)
            @use_state(image_size:Vec2 <= state.geo().size().as_vec2() * PREVIEW_HIGHT / state.geo().height() as f32)
        >
            <Node @style="flex-row">
                <UiButton @id="close" @style="m-2 w-20 h-20" NoTheme @on_event(close_window) >
                    <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/close.svg")))  @style="full"/>
                </UiButton>
                <Node @style="items-center justify-center m-auto"
                    Text=(Text::new(state.title()))
                    TextFont=(theme.text_font(16.0))
                    TextColor=(theme.default_text_color.into())
                    TextLayout=( TextLayout::new_with_justify(Justify::Left) )
                />
            </Node>
            <UiButton NoTheme @on_event(focus_window) >
                <MaterialNode::<RoundedUiImageMaterial>
                @handle(RoundedUiImageMaterial=>create_raw_window_material(*state.image_rect(),state.image().clone(),&state.geo, *state.image_size()))
                @style="w-{state.image_size().x} h-{state.image_size().y}" />
            </UiButton>
        </Node>
</Node>
}
