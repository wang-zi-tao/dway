use dway_ui_framework::theme::ThemeComponent;

use crate::{
    panels::PanelButtonBundle, popups::{launcher, panel_settings, volume_control}, prelude::*, widgets::{
        clock::Clock, notifys::NotifyButton, system_monitor::PanelSystemMonitor,
        windowtitle::WindowTitle, workspacelist::WorkspaceListUI,
    }
};

#[derive(Component)]
pub struct Panel {
    pub screen: Entity,
}

impl Panel {
    pub fn new(screen: Entity) -> Self {
        Self { screen }
    }
}

dway_widget! {
Panel=>
@global(theme: Theme)
@global(callbacks: CallbackTypeRegister)
@global(asset_server: AssetServer)
@global(mut assets_rounded_ui_rect_material: Assets<RoundedUiRectMaterial>)
<Node
    ThemeComponent
    BlockStyle=(BlockStyle::Blur)
    GlobalZIndex=(crate::zindex::PANEL)
    @style="absolute full"
    @id="panel">
    <Node @style="absolute flex-row left-4  align-items:center" @id="left">
        <(PanelButtonBundle::new(&theme,&mut assets!(RoundedUiRectMaterial)))
            @on_event((callbacks.system(launcher::open_popup))->self) @style="m-4">
            <(UiSvg::new(theme.icon("dashboard", &asset_server))) @style="w-24 h-24" @id="dashboard"/>
        </PanelButtonBundle>
        <WindowTitle/>
    </Node>
    <Node @style="absolute flex-row right-4 align-items:center" @id="right">
        <Clock/>
        <PanelSystemMonitor @id="system_monitor" @style="h-full"/>
        <NotifyButton @id="notify"/>
        <(PanelButtonBundle::new(&theme,&mut assets!(RoundedUiRectMaterial)))
            @on_event((callbacks.system(volume_control::open_popup))->self)  @style="m-4">
            <(UiSvg::new(theme.icon("volume_on", &asset_server))) @style="w-24 h-24" @id="volume"/>
        </PanelButtonBundle>
        <(PanelButtonBundle::new(&theme,&mut assets!(RoundedUiRectMaterial)))
            @on_event((callbacks.system(panel_settings::open_popup))->self)  @style="m-4">
            <(UiSvg::new(theme.icon("settings", &asset_server))) @style="w-24 h-24" @id="settings"/>
        </PanelButtonBundle>
    </Node>
    <Node @style="absolute w-full h-full justify-center items-center" @id="center">
        <Node @style="flex-row m-0 h-90%" >
            <WorkspaceListUI @id="workspace_list" />
        </Node>
    </Node>
</>
}
