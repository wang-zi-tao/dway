use bevy_svg::prelude::Svg;
use dway_client_core::controller::notify::{NotifyAction, NotifyData, NotifyDataBuilder, NotifyHistory, NotifyRequest};
use dway_ui_framework::widgets::{button::UiRawButtonBundle, util::visibility};

use crate::prelude::*;

#[derive(Component, Default)]
pub struct NotifyButton;

dway_widget! {
NotifyButton=>
@callback{[UiButtonEvent]
    fn open_notify_list(
        In(event): In<UiButtonEvent>,
        mut query: Query<&mut NotifyButtonState>,
        mut notify_sender: EventWriter<NotifyRequest>
    ) {
        let Ok(mut state) = query.get_mut(event.receiver) else {return};
        if event.kind == UiButtonEventKind::Released{
        }
    }
}
@use_state(notify_count: usize)
@global(theme:Theme)
@global(asset_server: AssetServer)
<UiRawButtonBundle UiButton=(UiButton::new(this_entity, open_notify_list))
    @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 8.0))
>
    <(UiSvgBundle::new(theme.icon("notifications", &asset_server))) @style="w-24 h-24" @id="icon"/>
</UiRawButtonBundle>
}

#[derive(Component, Default)]
pub struct NotifyView {
    pub notify: NotifyHistory,
    pub with_close_button: bool,
}

dway_widget!{
NotifyView=>
@global(theme: Theme)
@global(asset_server: AssetServer)
@use_state(image: Handle<Image>)
@use_state(svg: Handle<Svg>)
<MiniNodeBundle @style="flex-col">
    <MiniNodeBundle>
        <(UiSvgBundle::new(theme.icon("close", &asset_server))) @style="w-24 h-24" />
        <( UiTextBundle::new(&prop.notify.data.summary, 24, &theme) )/>
        <( UiTextBundle::new(&prop.notify.data.app_name, 24, &theme) ) @style="right-0"/>
    </MiniNodeBundle>
    <MiniNodeBundle @style="right-0" >
        <( UiTextBundle::new(&prop.notify.data.body, 24, &theme) )/>
        <UiSvgBundle UiSvg=(state.svg().clone().into())
            Visibility=(visibility(state.svg()!=&Handle::default())) />
        <UiImageBundle UiImage=(state.image().clone().into())
            Visibility=(visibility(state.image()!=&Handle::default()))/>
    </MiniNodeBundle>
    <MiniNodeBundle @id="actions" 
        Visibility=(visibility(prop.notify.data.actions.is_empty()))
    @for(action: NotifyAction in prop.notify.data.actions.iter().cloned() => {
        state.set_action(action.name.clone());
        state.set_text(action.text.clone());
    })>
        <UiButtonBundle @use_state(action: String) @use_state(text: String) >
            <( UiTextBundle::new(&state.text(), 24, &theme) )/>
        </UiButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
