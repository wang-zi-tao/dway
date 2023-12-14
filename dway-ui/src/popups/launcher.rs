use dway_server::apps::DesktopEntriesSet;

use crate::{
    animation,
    framework::{
        animation::despawn_animation,
        button::{UiButtonEvent, UiButtonEventKind},
        svg::UiSvgBundle,
    },
    panels::PanelButtonBundle,
    prelude::*,
    widgets::popup::{
        delay_destroy, delay_destroy_up, PopupEvent, PopupEventKind, UiPopup, UiPopupAddonBundle,
    },
};

#[derive(Component, Default)]
pub struct LauncherUI;

pub fn delay_destroy_launcher(In(event): In<PopupEvent>, mut commands: Commands) {
    if PopupEventKind::Closed == event.kind {
        commands.entity(event.entity).insert(despawn_animation(
            animation!(Tween 0.5 secs:BackOut->TransformScaleLens(Vec3::ONE=>Vec3::X)),
        ));
    }
}

dway_widget! {
LauncherUI=>
@bundle{{pub popup: UiPopupAddonBundle}}
@global(theme:Theme)
@global(entries: DesktopEntriesSet)
@plugin{{
    app.register_system(open_popup);
    app.register_system(delay_destroy_launcher);
}}
<MiniNodeBundle
Animator<_>=(animation!(0.5 secs:BackIn->TransformScaleLens(Vec3::X=>Vec3::ONE)))
@material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup"), 16.0))
@style="flex-col m-4">
    <MiniNodeBundle @style="min-h-600">
        // <MiniNodeBundle @id="left_bar" @style="w-34% m-4 min-h-600"
        //     @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup")*0.9, 16.0))
        // >
        // </MiniNodeBundle>
        <MiniNodeBundle @id="right_block" @style="m-4 min-h-600"
            @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup")*0.9, 16.0))
        >
        </MiniNodeBundle>
    </MiniNodeBundle>
    <MiniNodeBundle @id="bottom_bar" @style="m-4 min-w-512 justify-content:space-evenly"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("panel-popup")*0.9, 16.0))
    >
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="user_icon">
            <(UiSvgBundle::new(theme.icon("user"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="lock_button">
            <(UiSvgBundle::new(theme.icon("lock"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="logout_button">
            <(UiSvgBundle::new(theme.icon("logout"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="reboot_button">
            <(UiSvgBundle::new(theme.icon("restart"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
        <( PanelButtonBundle::new(this_entity,&theme,&mut assets_rounded_ui_rect_material) ) @style="w-32 h-32" @id="poweroff_button">
            <(UiSvgBundle::new(theme.icon("power"))) @style="w-32 h-32"/>
        </PanelButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}

#[dexterous_developer_setup(LauncherUI)]
fn reloadable(app: &mut ReloadableAppContents) {
    app.add_systems(Update, launcher_ui_render.in_set(LauncherUISystems::Render));
}

pub fn open_popup(In(event): In<UiButtonEvent>, theme: Res<Theme>, mut commands: Commands) {
    if event.kind == UiButtonEventKind::Pressed {
        commands
            .spawn(LauncherUIBundle {
                popup: UiPopupAddonBundle {
                    popup: UiPopup {
                        callback: Some(theme.system(delay_destroy_launcher)),
                        ..default()
                    },
                    ..default()
                },
                style: style!("absolute top-110% left-0"),
                ..default()
            })
            .set_parent(event.button);
    }
}

pub struct LauncherUIPluginHotReload;
impl Plugin for LauncherUIPluginHotReload {
    fn build(&self, app: &mut App) {
        app.setup_reloadable_elements::<reloadable>();
        app.register_type::<LauncherUIWidget>();
        app.register_system(open_popup);
        app.register_system(delay_destroy_launcher);
    }
}
