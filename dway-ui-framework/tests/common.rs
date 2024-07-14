use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use bevy::{
    app::AppExit,
    core::FrameCount,
    ecs::system::BoxedSystem,
    render::{
        camera::RenderTarget,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{self, NodeRunError, RenderGraphContext},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d,
            ImageCopyBuffer, ImageDataLayout, Maintain, MapMode,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{GpuImage, Image},
        view::screenshot::ScreenshotManager,
        Render, RenderApp, RenderSet,
    },
    window::{PresentMode, WindowRef},
    winit::{WakeUp, WinitPlugin},
};
use crossbeam_channel::{Receiver, Sender};
use dway_ui_framework::{prelude::*, *};
use image::{DynamicImage, GenericImageView, RgbaImage};
use rayon::iter::IntoParallelRefIterator;

pub fn image_diff(src_image: &RgbaImage, dest_image: &RgbaImage) -> RgbaImage {
    assert_eq!(src_image.width(), dest_image.width());
    assert_eq!(src_image.height(), dest_image.height());
    let mut output_image = RgbaImage::new(src_image.width(), src_image.height());
    for (src_pixel, dest_pixel) in
        Iterator::zip(src_image.enumerate_pixels(), dest_image.enumerate_pixels())
    {
        assert_eq!(src_pixel.0, dest_pixel.0);
        assert_eq!(src_pixel.1, dest_pixel.1);
        if src_pixel.2 != dest_pixel.2 {
            output_image.get_pixel_mut(src_pixel.0, src_pixel.1).0 = [255, 0, 0, 255];
        }
    }
    output_image
}

pub fn compare_image(
    src_image: &RgbaImage,
    dest_image: &RgbaImage,
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
                    let dest_pixel =
                        Vec4::from_array(dest_image.get_pixel(x, y).0.map(|m| m as f32 / 256.0));
                    let diff = (src_pixel - dest_pixel).abs().max_element();
                    if diff > 4.0 / 256.0 {
                        break 'l;
                    }
                }
            }
            return Ok(None);
        }
    }
    let diff_image = image_diff(&src_image, &dest_image);
    let mut tmp = tmp.to_owned();
    tmp.push("diff.png");
    diff_image.save(&tmp)?;
    Ok(Some(tmp))
}

#[derive(Component)]
struct ImageCopier {
    buffer: Buffer,
    src_image: Handle<Image>,
    rx: Receiver<RgbaImage>,
    tx: Sender<RgbaImage>,
    processing: AtomicBool,
}

#[derive(Component)]
struct ExtractedImageCopier {
    buffer: Buffer,
    src_image: Handle<Image>,
    tx: Sender<RgbaImage>,
}

impl ExtractComponent for ImageCopier {
    type Out = ExtractedImageCopier;
    type QueryData = &'static ImageCopier;
    type QueryFilter = ();

    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        item.processing.store(true, Ordering::Relaxed);
        Some(ExtractedImageCopier {
            buffer: item.buffer.clone(),
            src_image: item.src_image.clone(),
            tx: item.tx.clone(),
        })
    }
}

impl ImageCopier {
    pub fn new(
        src_image: Handle<Image>,
        size: Extent3d,
        render_device: &RenderDevice,
    ) -> ImageCopier {
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;

        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (tx, rx) = crossbeam_channel::bounded(1);

        ImageCopier {
            buffer: cpu_buffer,
            src_image,
            tx,
            rx,
            processing: AtomicBool::new(false),
        }
    }
}

struct CopyImageNode {
    coiper_query: QueryState<&'static ExtractedImageCopier>,
}

impl FromWorld for CopyImageNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            coiper_query: QueryState::new(world),
        }
    }
}

impl render_graph::Node for CopyImageNode {
    fn update(&mut self, world: &mut World) {
        self.coiper_query.update_archetypes(world)
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let gpu_images = world
            .get_resource::<RenderAssets<bevy::render::texture::GpuImage>>()
            .unwrap();

        for image_copier in self.coiper_query.iter_manual(world) {
            let gpu_image = gpu_images.get(&image_copier.src_image).unwrap();
            let mut encoder = render_context
                .render_device()
                .create_command_encoder(&CommandEncoderDescriptor::default());
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (gpu_image.size.x as usize
                    / gpu_image.texture_format.block_dimensions().0 as usize)
                    * gpu_image.texture_format.block_copy_size(None).unwrap() as usize,
            );

            encoder.copy_texture_to_buffer(
                gpu_image.texture.as_image_copy(),
                ImageCopyBuffer {
                    buffer: &image_copier.buffer,
                    layout: ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                Extent3d {
                    width: gpu_image.size.x,
                    height: gpu_image.size.y,
                    depth_or_array_layers: 1,
                },
            );

            let render_queue = world.get_resource::<RenderQueue>().unwrap();
            render_queue.submit(std::iter::once(encoder.finish()));
        }

        Ok(())
    }
}

structstruck::strike! {
    #[derive(Component, Clone, ExtractComponent)]
    pub struct UnitTest{
        pub name: String,
        pub image_path: PathBuf,
        pub image_size: Vec2,
        pub output_dir: PathBuf,
        pub setup: SystemId<
        pub struct UnitTestParams{
            pub camera: Entity,
            pub window: Entity,
        }>,
    }
}

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<UnitTest>::default(),
            ExtractComponentPlugin::<ImageCopier>::default(),
        ))
        .add_systems(Last, wait_render);
        let render_app = app.sub_app_mut(RenderApp);

        render_app.add_systems(Render, receive_image_from_buffer.in_set(RenderSet::Cleanup));
    }
}

fn receive_image_from_buffer(
    copier_qeruy: Query<(&mut ExtractedImageCopier)>,
    render_device: Res<RenderDevice>,
    gpu_images: ResMut<RenderAssets<GpuImage>>,
) {
    for image_copier in copier_qeruy.iter() {
        let buffer_slice = image_copier.buffer.slice(..);
        let (s, r) = crossbeam_channel::bounded(1);
        buffer_slice.map_async(MapMode::Read, move |r| match r {
            // This will execute once the gpu is ready, so after the call to poll()
            Ok(r) => s.send(r).expect("Failed to send map update"),
            Err(err) => panic!("Failed to map buffer {err}"),
        });
        render_device.poll(Maintain::wait()).panic_on_timeout();
        r.recv().expect("Failed to receive the map_async message");
        let vec = buffer_slice.get_mapped_range().to_vec();

        let gpu_image = gpu_images.get(&image_copier.src_image).unwrap();
        let output_image =
            image::RgbaImage::from_raw(gpu_image.size.x, gpu_image.size.y, vec).unwrap();
        let _ = image_copier.tx.send(output_image);
        image_copier.buffer.unmap();
    }
}

fn wait_render(
    copier_qeruy: Query<(Entity, &mut ImageCopier, &UnitTest)>,
    mut test_result: ResMut<TestSuiteResource>,
    mut commands: Commands,
) {
    for (entity, copie, unit_test) in copier_qeruy.iter() {
        if !copie.processing.load(Ordering::Relaxed) {
            continue;
        }

        let tmp = &unit_test.output_dir;
        let src_image = copie.rx.recv().unwrap();
        let dest = &unit_test.image_path;
        let result = match image::open(dest)
            .map(|image| image.into_rgba8())
            .map_err(|e| e.into())
            .and_then(|dest_image| compare_image(&src_image, &dest_image, &unit_test.output_dir))
        {
            Ok(Some(diff)) => {
                let mut output_image_path = tmp.to_owned();
                output_image_path.push("screenshot.png");
                src_image.save(&output_image_path).unwrap();
                UnitTestState::Err(anyhow::anyhow!("image is different. \nexcept: {dest:?}\nscreenshot: {output_image_path:?}\ndiff image: {diff:?}"))
            }
            Ok(None) => {
                let _ = std::fs::remove_dir_all(tmp);
                UnitTestState::Ok
            }
            Err(e) => {
                let mut output_image_path = tmp.to_owned();
                output_image_path.push("screenshot.png");
                src_image.save(&output_image_path).unwrap();
                UnitTestState::Err(anyhow::anyhow!("failed to compare image: {e} \nexcept: {dest:?}\nscreenshot: {output_image_path:?}"))
            }
        };
        test_result
            .unit_tests
            .insert(unit_test.name.clone(), result);
    }
}

pub struct TestPluginsSet;
impl PluginGroup for TestPluginsSet {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let group = DefaultPlugins.build();
        group.set(WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            close_when_requested: false,
        })
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
    pub unit_tests: HashMap<String, UnitTestState>,
}

// pub fn run_test_plugins(name: &str, tests: Vec<UnitTestPlugin>) {
//     let title = format!("dway_ui_framework unit test ({name})");
//     let mut app = App::default();
//     let test_states = Arc::new(Mutex::new(HashMap::from_iter(
//         tests
//             .iter()
//             .map(|t| (t.name.clone(), UnitTestState::Padding)),
//     )));
//     app.add_plugins(
//         DefaultPlugins
//             .build()
//             .set(WindowPlugin {
//                 primary_window: Some(Window {
//                     title: title.clone(),
//                     name: Some(title.clone()),
//                     visible: false,
//                     ..default()
//                 }),
//                 ..default()
//             })
//             .set({
//                 let mut plugin = WinitPlugin::<WakeUp>::default();
//                 plugin.run_on_any_thread = true;
//                 plugin
//             })
//             .add(crate::UiFrameworkPlugin),
//     )
//     .insert_resource(ClearColor(Color::WHITE))
//     .insert_resource(TestSuiteResource {
//         name: name.to_owned(),
//         unit_tests: test_states.clone(),
//     })
//     .add_systems(
//         Last,
//         move |frame: Res<FrameCount>,
//               mut exit_event: EventWriter<AppExit>,
//               tests_suite: ResMut<TestSuiteResource>| {
//             if tests_suite
//                 .unit_tests
//                 .lock()
//                 .unwrap()
//                 .values()
//                 .all(|s| matches!(s, UnitTestState::Ok | UnitTestState::Err(_)))
//                 || frame.0 > 256
//             {
//                 exit_event.send(AppExit::Success);
//             }
//         },
//     );
//     for plugin in tests {
//         app.add_plugins(plugin);
//     }
//     app.run();
//     for (name, test_stat) in test_states.lock().unwrap().iter() {
//         match test_stat {
//             UnitTestState::Padding => {
//                 error!("unit test {name}: timeout");
//             }
//             UnitTestState::Ok => {
//                 info!("unit test {name}: Ok");
//             }
//             UnitTestState::Err(e) => {
//                 error!("unit test {name}: failed: {e}");
//             }
//         }
//     }
//     if !test_states
//         .lock()
//         .unwrap()
//         .iter()
//         .all(|(_, r)| matches!(r, UnitTestState::Ok))
//     {
//         std::thread::sleep(Duration::from_secs(1));
//         panic!("test failed");
//     }
// }
