use dway_client_core::{desktop::FocusedWindow, workspace::WindowList};
use dway_server::{
    geometry::GlobalGeometry, util::rect::IRect, wl::surface::WlSurface,
    xdg::toplevel::DWayToplevel,
};
use event::make_callback;
use widgets::button::UiButtonEventDispatcher;

use crate::{prelude::*, widgets::window::create_raw_window_material};

pub const PREVIEW_HIGHT: f32 = 128.0;

#[derive(Component, Reflect)]
pub struct WorkspaceWindowPreviewPopup {
    pub workspace: Entity,
    pub height: f32,
}
impl Default for WorkspaceWindowPreviewPopup {
    fn default() -> Self {
        Self {
            workspace: Entity::PLACEHOLDER,
            height: 250.0,
        }
    }
}

fn close_window(
    event: UiEvent<UiButtonEvent>,
    prop_query: Query<&WorkspaceWindowPreviewPopupSubWidgetList>,
    mut events: EventWriter<WindowAction>,
){
    let Ok(widget) = prop_query.get(event.receiver())else{return;};
    if event.kind == UiButtonEventKind::Released{
        events.send(WindowAction::Close(widget.data_entity));
    }
}

fn focus_window(
    event: UiEvent<UiButtonEvent>,
    prop_query: Query<&WorkspaceWindowPreviewPopupSubWidgetList>,
    mut focused: ResMut<FocusedWindow>,
){
    let Ok(widget) = prop_query.get(event.receiver())else{return;};
    if event.kind == UiButtonEventKind::Released{
        focused.window_entity = Some(widget.data_entity);
    }
}

dway_widget! {
WorkspaceWindowPreviewPopup=>
@plugin{
    app.register_type::<WorkspaceWindowPreviewPopup>();
}
@state_reflect()
@global(theme: Theme)
@arg(asset_server: Res<AssetServer>)
@use_state(windows: Vec<Entity>)
@component(window_list<-Query<Ref<WindowList>>[prop.workspace]->{ state.set_windows(window_list.iter().collect()); })
@add_callback([UiEvent<UiButtonEvent>] close_window)
@add_callback([UiEvent<UiButtonEvent>] focus_window)
<Node @style="flex-row m-4" @id="List"
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
                <UiButton NoTheme @id="close" @style="m-2 w-20 h-20" @on_event(close_window) >
                    <(UiSvg::new(asset_server.load("embedded://dway_ui/icons/close.svg")))  @style="full"/>
                </UiButton>
                <Node @style="items-center justify-center m-auto"
                    Text=(Text::new(state.title()))
                    TextFont=(theme.text_font(16.0))
                    TextColor=(theme.default_text_color.into())
                    TextLayout=( TextLayout::new_with_justify(JustifyText::Left) )
                />
            </Node>
            <UiButton NoTheme @on_event(focus_window)>
                <MaterialNode::<RoundedUiImageMaterial>
                @handle(RoundedUiImageMaterial=>create_raw_window_material(*state.image_rect(),state.image().clone(),&state.geo, *state.image_size()))
                @style="w-{state.image_size().x} h-{state.image_size().y}" />
            </UiButton>
        </Node>
</Node>
}
