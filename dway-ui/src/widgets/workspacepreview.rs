use dway_client_core::{desktop::FocusedWindow, workspace::WindowList};
use dway_server::{
    geometry::GlobalGeometry, util::rect::IRect, wl::surface::WlSurface,
    xdg::toplevel::DWayToplevel,
};
use dway_ui_framework::widgets::button::UiRawButtonBundle;
use widgets::button::UiButtonEventDispatcher;

use crate::{prelude::*, widgets::window::create_raw_window_material};

#[derive(Component, Reflect)]
pub struct WorkspacePreview {
    pub workspace: Entity,
    pub scale: f32,
}
impl Default for WorkspacePreview {
    fn default() -> Self {
        Self {
            workspace: Entity::PLACEHOLDER,
            scale: 1.0 / 16.0,
        }
    }
}

dway_widget! {
WorkspacePreview=>
@callback{ [UiEvent<UiButtonEvent>]
fn focus_window(
    event: UiEvent<UiButtonEvent>,
    prop_query: Query<&WorkspacePreviewSubWidgetList>,
    mut focused: ResMut<FocusedWindow>,
){
    let Ok(widget) = prop_query.get(event.receiver())else{return;};
    if event.kind == UiButtonEventKind::Released{
        focused.window_entity = Some(widget.data_entity);
    }
}}
@global(theme: Theme)
@arg(asset_server: Res<AssetServer>)
@use_state(windows: Vec<Entity>)
@query(workspace_query: (window_list)<-Query<Ref<WindowList>>[prop.workspace]->{
    if !widget.inited || window_list.is_changed() {
        state.set_windows(window_list.iter().collect());
    }
})
<MiniNodeBundle @style="flex-row m-4" @id="List"
    @for_query((surface,geo,toplevel) in Query<(Ref<WlSurface>,Ref<GlobalGeometry>,Ref<DWayToplevel>)>::iter_many(state.windows().iter().cloned()) =>[
        toplevel=>{state.set_title(toplevel.title.clone().unwrap_or_default());},
        geo=>{state.set_geo(geo.clone());},
        surface=>{
            state.set_image(surface.image.clone());
            state.set_image_rect(surface.image_rect());
        }
    ]) >
        <MiniNodeBundle @style="absolute m-4" @id="window_preview"
            @use_state(title:String) @use_state(geo:GlobalGeometry) @use_state(image:Handle<Image>) @use_state(image_rect:IRect)
            @use_state(preview_rect:Rect <= *state.image_rect() * prop.scale)
        >
            <UiRawButtonBundle UiButtonEventDispatcher=(make_callback(node!(window_preview), focus_window))
                @style="absolute left-{state.preview_rect().min.x} top-{state.preview_rect().min.y} w-{state.preview_rect().width()} h-{state.preview_rect().height()}"
                @handle(RoundedUiRectMaterial=>rounded_rect(theme.color("border"), 16.0))
            >
                <MaterialNodeBundle::<RoundedUiImageMaterial>
                @handle(RoundedUiImageMaterial=>create_raw_window_material(*state.image_rect(),state.image().clone(),&state.geo, state.preview_rect().size()))
                @style="m-2 full" />
            </UiRawButtonBundle>
        </MiniNodeBundle>
</MiniNodeBundle>
}
