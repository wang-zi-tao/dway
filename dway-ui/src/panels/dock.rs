use dway_ui_framework::theme::ThemeComponent;

use crate::{panels::PanelButtonBundle, popups::dock_launcher, prelude::*, widgets::applist::AppListUI};

#[derive(Component)]
pub struct Dock {
    pub screen: Entity,
}

impl Dock {
    pub fn new(screen: Entity) -> Self {
        Self { screen }
    }
}

dway_widget!{
Dock=>
@global(theme: Theme)
@global(callbacks: CallbackTypeRegister)
@global(asset_server: AssetServer)
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
<Node 
    ThemeComponent 
    BlockStyle=(BlockStyle::Blur) 
    GlobalZIndex=(crate::zindex::DOCK)>
    <AppListUI/>
    <(PanelButtonBundle::new(&theme,&mut assets!(RoundedUiRectMaterial)))
        @on_event((callbacks.system(dock_launcher::open_popup))) >
        <(UiSvg::new(theme.icon("apps", &asset_server))) @style="w-48 h-48" @id="apps"/>
    </PanelButtonBundle>
</Node>
}
