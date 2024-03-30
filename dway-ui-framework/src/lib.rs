#![feature(round_char_boundary)]

pub mod animation;
pub mod assets;
pub mod diagnostics;
pub mod input;
pub mod prelude;
pub mod render;
pub mod shader;
pub mod theme;
pub mod widgets;

use crate::{
    prelude::*,
    render::mesh::{UiMeshHandle, UiMeshMaterialPlugin, UiMeshTransform},
    widgets::svg::{SvgLayout, SvgMagerial},
};
use bevy::ui::UiSystem;
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_svg::SvgPlugin;
pub use dway_ui_derive::*;

pub struct UiFrameworkPlugin;
impl Plugin for UiFrameworkPlugin {
    fn build(&self, app: &mut App) {
        use UiFrameworkSystems::*;
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        if !app.is_plugin_added::<ShapePlugin>() {
            app.add_plugins(ShapePlugin);
        }
        app.add_plugins((
            assets::UiAssetsPlugin,
            theme::ThemePlugin,
            theme::flat::FlatThemePlugin::default(),
            render::mesh::UiMeshPlugin,
            shader::ShaderFrameworkPlugin,
            render::mesh::UiMeshMaterialPlugin::<ColorMaterial>::default(),
            animation::AnimationPlugin,
        ))
        .add_plugins((
            widgets::slider::UiSliderPlugin,
            widgets::scroll::UiScrollPlugin,
            widgets::inputbox::UiInputBoxPlugin,
            UiMeshMaterialPlugin::<SvgMagerial>::default(),
        ))
        .register_type::<UiCheckBox>()
        .register_type::<UiCheckBoxState>()
        .register_type::<UiSlider>()
        .register_type::<UiButton>()
        .register_type::<UiSvg>()
        .register_type::<UiPopup>()
        .register_type::<UiMeshHandle>()
        .register_type::<UiMeshTransform>()
        .register_type::<SvgLayout>()
        .register_type::<input::UiInput>()
        .register_type::<animation::Animation>()
        .init_asset::<SvgMagerial>()
        .register_type::<input::MousePosition>()
        .init_resource::<input::MousePosition>()
        .register_type::<input::UiFocusState>()
        .init_resource::<input::UiFocusState>()
        .add_event::<input::UiFocusEvent>()
        .register_type::<input::UiFocusEvent>()
        .register_system(delay_destroy)
        .register_system(delay_destroy_up)
        .add_systems(
            PreUpdate,
            (
                input::update_mouse_position
                    .run_if(on_event::<CursorMoved>())
                    .in_set(InputSystems),
                update_ui_input.in_set(InputSystems),
                widgets::button::process_ui_button_event.in_set(WidgetInputSystems),
                widgets::checkbox::process_ui_checkbox_event.in_set(WidgetInputSystems),
                widgets::inputbox::process_ui_inputbox_event.in_set(WidgetInputSystems),
            ),
        )
        .add_systems(
            PostUpdate,
            (
                widgets::svg::uisvg_update_system.in_set(UpdateWidgets),
                widgets::shape::after_process_shape
                    .in_set(ProcessMesh)
                    .after(bevy_prototype_lyon::plugin::BuildShapes),
                widgets::popup::update_popup.in_set(UpdatePopup),
            ),
        )
        .configure_sets(
            PreUpdate,
            (
                InputSystems.after(bevy::ui::UiSystem::Focus),
                WidgetInputSystems,
            )
                .chain(),
        )
        .configure_sets(
            PostUpdate,
            (UpdateWidgets, UpdatePopup, UpdateTheme, ApplyAnimation)
                .before(UiSystem::Layout)
                .chain(),
        )
        .add_plugins((
            RoundedUiRectMaterial::plugin(),
            UiCircleMaterial::plugin(),
            RoundedUiImageMaterial::plugin(),
            RoundedBlockMaterial::plugin(),
            RoundedBorderBlockMaterial::plugin(),
            HollowBlockMaterial::plugin(),
            ButtonMaterial::plugin(),
            RoundedRainbowBlockMaterial::plugin(),
            Fake3dButton::plugin(),
            CheckboxMaterial::plugin(),
            RoundedInnerShadowBlockMaterial::plugin(),
            ArcMaterial::plugin(),
            AssetAnimationPlugin::<CheckboxMaterial>::default(),
        ));
    }
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum UiFrameworkSystems {
    InputSystems,
    WidgetInputSystems,
    UpdateWidgets,
    UpdatePopup,
    UpdateTheme,
    ApplyAnimation,
    ProcessMesh,
}

#[cfg(test)]
pub mod tests {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    use bevy::{
        app::AppExit,
        core::FrameCount,
        ecs::system::BoxedSystem,
        render::{camera::RenderTarget, view::screenshot::ScreenshotManager},
        window::{PresentMode, WindowRef},
        winit::WinitPlugin,
    };
    use image::{DynamicImage, GenericImageView};

    use super::*;

    pub fn assert_image_eq(image: &Image, dest: &Path, tmp: &Path) {
        let src_image = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_raw(image.width(), image.height(), image.data.clone()).unwrap(),
        );
        let dest_image = image::open(dest).unwrap();
        match compare_image(&src_image, &dest_image, tmp).unwrap() {
            Some(diff) => {
                let mut output_image_path = tmp.to_owned();
                output_image_path.push("screenshot.png");
                src_image.save(&output_image_path).unwrap();
                panic!("image is different. \nexcept: {dest:?}\nscreenshot: {output_image_path:?}\ndiff image: {diff:?}");
            }
            None => {
                std::fs::remove_dir_all(tmp).unwrap();
            }
        };
    }

    pub fn compare_image(
        src_image: &DynamicImage,
        dest_image: &DynamicImage,
        tmp: &Path,
    ) -> Result<Option<PathBuf>, anyhow::Error> {
        'l: {
            if src_image.width() == dest_image.width() && src_image.height() == src_image.height() {
                let width = src_image.width();
                let height = src_image.height();
                for y in 0..height {
                    for x in 0..width {
                        let src_pixel =
                            Vec4::from_array(src_image.get_pixel(x, y).0.map(|m| m as f32 / 256.0));
                        let dest_pixel = Vec4::from_array(
                            dest_image.get_pixel(x, y).0.map(|m| m as f32 / 256.0),
                        );
                        let diff = (src_pixel - dest_pixel).abs().max_element();
                        if diff > 4.0 / 256.0 {
                            break 'l;
                        }
                    }
                }
                return Ok(None);
            }
        }
        let diff_image = image_diff::diff(dest_image, src_image)?;
        let mut tmp = tmp.to_owned();
        tmp.push("diff.png");
        diff_image.save(&tmp)?;
        Ok(Some(tmp))
    }

    pub struct UnitTestPluginSystemArgs<'l> {
        pub plugin: &'l UnitTestPlugin,
        pub window_entity: Entity,
        pub camera_entity: Entity,
    }

    pub struct UnitTestPlugin {
        pub name: String,
        pub image_path: PathBuf,
        pub image_size: Vec2,
        pub setup: Box<dyn Fn(UnitTestPluginSystemArgs) -> BoxedSystem + Send + Sync + 'static>,
        pub plugin: Box<dyn Fn(UnitTestPluginSystemArgs, &mut App) + Send + Sync + 'static>,
        pub output_dir: PathBuf,
    }

    impl Plugin for UnitTestPlugin {
        fn build(&self, app: &mut App) {
            let name = self.name.clone();
            let title = format!("unit test: {}", &self.name);
            let size = self.image_size;
            let window_entity = app
                .world
                .spawn(Window {
                    title: title.clone(),
                    name: Some(title),
                    visible: false,
                    present_mode: PresentMode::AutoVsync,
                    prevent_default_event_handling: false,
                    resolution: (size.x, size.y).into(),
                    ..Default::default()
                })
                .id();
            let camera_entity = app
                .world
                .spawn(Camera2dBundle {
                    camera: Camera {
                        target: RenderTarget::Window(WindowRef::Entity(window_entity)),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .id();
            app.add_systems(
                Startup,
                (self.setup)(UnitTestPluginSystemArgs {
                    plugin: self,
                    window_entity,
                    camera_entity,
                }),
            );
            (self.plugin)(
                UnitTestPluginSystemArgs {
                    plugin: self,
                    window_entity,
                    camera_entity,
                },
                app,
            );
            let image_path = self.image_path.clone();
            let output_path = self.output_dir.clone();
            app.add_systems(
                Last,
                move |frame: Res<FrameCount>,
                      tests_suite: ResMut<TestSuiteResource>,
                      mut snapshot_manager: ResMut<ScreenshotManager>| {
                let name = name.clone();
                let image_path = image_path.clone();
                let output_path = output_path.clone();
                let state = tests_suite.unit_tests.clone();
                if frame.0 > 32 && matches!(state.lock().unwrap().get(&name).unwrap(), UnitTestState::Padding){
                    if let Err(e) =
                        snapshot_manager.take_screenshot(window_entity, move |image| {
                                let dest: &Path = &image_path;
                                let tmp: &Path = &output_path;
                                let src_image = image::DynamicImage::ImageRgba8(
                                    image::RgbaImage::from_raw(image.width(), image.height(), image.data.clone()).unwrap(),
                                );
                                match image::open(dest).map_err(|e|e.into()).and_then(|dest_image|compare_image(&src_image, &dest_image, tmp)){
                                    Ok(Some(diff)) => {
                                        let mut output_image_path = tmp.to_owned();
                                        output_image_path.push("screenshot.png");
                                        src_image.save(&output_image_path).unwrap();
                                        state.lock().unwrap().insert(name, UnitTestState::Err(anyhow::anyhow!("image is different. \nexcept: {dest:?}\nscreenshot: {output_image_path:?}\ndiff image: {diff:?}")));
                                    },
                                    Ok(None) => {
                                        let _ = std::fs::remove_dir_all(tmp);
                                        state.lock().unwrap().insert(name, UnitTestState::Ok);
                                    },
                                    Err(e) => {
                                        let mut output_image_path = tmp.to_owned();
                                        output_image_path.push("screenshot.png");
                                        src_image.save(&output_image_path).unwrap();
                                        state.lock().unwrap().insert(name, UnitTestState::Err(anyhow::anyhow!("failed to compare image: {e} \nexcept: {dest:?}\nscreenshot: {output_image_path:?}")));
                                    },
                                }
                        })
                    {
                        error!("failed to take snapshot: {e}");
                    };
                }
                },
            );
        }
        fn is_unique(&self) -> bool {
            false
        }
    }

    pub enum UnitTestState {
        Padding,
        Ok,
        Err(anyhow::Error),
    }

    #[derive(Resource)]
    pub struct TestSuiteResource {
        pub name: String,
        pub unit_tests: Arc<Mutex<HashMap<String, UnitTestState>>>,
    }

    pub fn run_test_plugins(name: &str, tests: Vec<UnitTestPlugin>) {
        let title = format!("dway_ui_framework unit test ({name})");
        let mut app = App::default();
        let test_states = Arc::new(Mutex::new(HashMap::from_iter(
            tests
                .iter()
                .map(|t| (t.name.clone(), UnitTestState::Padding)),
        )));
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: title.clone(),
                        name: Some(title.clone()),
                        visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(WinitPlugin {
                    run_on_any_thread: true,
                })
                .add(crate::UiFrameworkPlugin),
        )
        .insert_resource(ClearColor(Color::WHITE))
        .insert_resource(TestSuiteResource {
            name: name.to_owned(),
            unit_tests: test_states.clone(),
        })
        .add_systems(
            Last,
            move |frame: Res<FrameCount>,
                  mut exit_event: EventWriter<AppExit>,
                  tests_suite: ResMut<TestSuiteResource>| {
                if tests_suite
                    .unit_tests
                    .lock()
                    .unwrap()
                    .values()
                    .all(|s| matches!(s, UnitTestState::Ok | UnitTestState::Err(_)))
                    || frame.0 > 256
                {
                    exit_event.send(AppExit);
                }
            },
        );
        for plugin in tests {
            app.add_plugins(plugin);
        }
        app.run();
        for (name, test_stat) in test_states.lock().unwrap().iter() {
            match test_stat {
                UnitTestState::Padding => {
                    error!("unit test {name}: timeout");
                }
                UnitTestState::Ok => {
                    info!("unit test {name}: Ok");
                }
                UnitTestState::Err(e) => {
                    error!("unit test {name}: failed: {e}");
                }
            }
        }
        if !test_states
            .lock()
            .unwrap()
            .iter()
            .all(|(_, r)| matches!(r, UnitTestState::Ok))
        {
            std::thread::sleep(Duration::from_secs(1));
            panic!("test failed");
        }
    }
}
