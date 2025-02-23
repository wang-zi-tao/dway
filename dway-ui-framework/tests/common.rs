use std::{
    collections::HashMap,
    path::{absolute, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    core::FrameCount,
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    ecs::system::SystemId,
    prelude::*,
    render::{
        camera::RenderTarget,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d,
            ImageCopyBuffer, ImageDataLayout, Maintain, MapMode, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::{GpuImage},
        Render, RenderApp, RenderSet,
    },
    window::PresentMode,
    winit::WinitPlugin,
};
use crossbeam_channel::{Receiver, Sender};
use dway_ui_framework::prelude::*;
use image::RgbaImage;

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
) -> Result<Option<RgbaImage>, anyhow::Error> {
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
    Ok(Some(diff_image))
}

#[derive(Component)]
struct ImageCopier {
    buffer: Buffer,
    src_image: Handle<Image>,
    rx: Receiver<Option<RgbaImage>>,
    tx: Sender<Option<RgbaImage>>,
    processing: AtomicBool,
}

#[derive(Component)]
struct ExtractedImageCopier {
    buffer: Buffer,
    src_image: Handle<Image>,
    tx: Sender<Option<RgbaImage>>,
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
    pub fn new(src_image: Handle<Image>, size: Vec2, render_device: &RenderDevice) -> ImageCopier {
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(size.x as usize) * 4;

        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * size.y as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (tx, rx) = crossbeam_channel::bounded(1);

        ImageCopier {
            src_image,
            tx,
            rx,
            processing: AtomicBool::new(false),
            buffer: cpu_buffer,
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

pub struct UnitTestParams {
    pub camera: Entity,
    pub window: Entity,
}

#[derive(Component)]
pub struct UnitTest {
    pub name: String,
    pub image_path: PathBuf,
    pub image_size: Vec2,
    pub setup: SystemId<In<UnitTestParams>>,
}

fn start_unit_test(
    mut unit_test_qeruy: Query<(Entity, &mut UnitTest)>,
    mut commands: Commands,
    mut test_suit: ResMut<TestSuite>,
    mut images: ResMut<Assets<Image>>,
    render_device: Res<RenderDevice>,
) {
    for (entity, unit_test) in &mut unit_test_qeruy {
        test_suit
            .unit_tests
            .insert(unit_test.name.clone(), UnitTestState::Padding);

        let image_size = Extent3d {
            width: unit_test.image_size.x as u32,
            height: unit_test.image_size.y as u32,
            ..default()
        };
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size: image_size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(image_size);
        let image_handle = images.add(image);

        let title = format!("unit test: {}", &unit_test.name);
        let size = unit_test.image_size;
        let window_entity = commands
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
        let camera_entity = commands
            .spawn(Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Image(image_handle.clone()),
                    clear_color: ClearColorConfig::Custom(Color::WHITE),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id();
        commands.entity(entity).insert(ImageCopier::new(
            image_handle,
            unit_test.image_size,
            &render_device,
        ));

        let params = UnitTestParams {
            camera: camera_entity,
            window: window_entity,
        };
        commands.run_system_with_input(unit_test.setup, params);
    }
}

fn receive_image_from_buffer(
    copier_qeruy: Query<&mut ExtractedImageCopier>,
    render_device: Res<RenderDevice>,
    gpu_images: ResMut<RenderAssets<GpuImage>>,
    frame: Res<FrameCount>,
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
        image_copier.buffer.unmap();

        let gpu_image = gpu_images.get(&image_copier.src_image).unwrap();
        let output_image =
            image::RgbaImage::from_raw(gpu_image.size.x, gpu_image.size.y, vec).unwrap();
        let result = if frame.0 < 64 || output_image.get_pixel(16, 16).0 != [255, 255, 255, 255] {
            None
        } else {
            Some(output_image)
        };
        let _ = image_copier.tx.send(result);
    }
}

fn wait_render(
    copier_qeruy: Query<(Entity, &mut ImageCopier, &UnitTest)>,
    mut test_suit: ResMut<TestSuite>,
    mut commands: Commands,
) {
    for (entity, copie, unit_test) in copier_qeruy.iter() {
        if !copie.processing.load(Ordering::Relaxed) {
            continue;
        }

        let output_image = match copie.rx.recv_timeout(Duration::from_secs_f32(1.0 / 10.0)) {
            Ok(Some(output_image)) => output_image,
            Ok(None) => {
                continue;
            }
            Err(_e) => {
                warn!("render timeout");
                continue;
            }
        };

        let dest = absolute(&unit_test.image_path).unwrap();

        let result = match image::open(&dest)
            .map(|image| image.into_rgba8())
            .map_err(|e| e.into())
            .and_then(|dest_image| compare_image(&output_image, &dest_image))
        {
            Ok(Some(diff)) => {
                let mut diff_image_path = test_suit.tmp.to_owned();
                diff_image_path.push(format!("{}_diff.png", &unit_test.name));
                diff.save(&diff_image_path).unwrap();

                let mut output_image_path = test_suit.tmp.to_owned();
                output_image_path.push(format!("{}.png", &unit_test.name));
                output_image.save(&output_image_path).unwrap();
                UnitTestState::Err(anyhow::anyhow!("image is different. \nexcept: {dest:?}\nscreenshot: {output_image_path:?}\ndiff image: {diff_image_path:?}"))
            }
            Ok(None) => UnitTestState::Ok,
            Err(e) => {
                output_image.save(&unit_test.image_path).unwrap();
                UnitTestState::Err(anyhow::anyhow!(
                    "failed to compare image: {e} \nexcept: {dest:?}\nscreenshot: {:?}",
                    &unit_test.image_path
                ))
            }
        };
        info!("unit test ({}): {:?}", &unit_test.name, result);
        test_suit.unit_tests.insert(unit_test.name.clone(), result);

        commands.entity(entity).despawn_recursive();
    }
}

fn check_exit(
    frame: Res<FrameCount>,
    mut exit_event: EventWriter<AppExit>,
    test_suit: ResMut<TestSuite>,
) {
    let finished = test_suit
        .unit_tests
        .values()
        .all(|s| matches!(s, UnitTestState::Ok | UnitTestState::Err(_)));
    let success = test_suit
        .unit_tests
        .values()
        .all(|s| matches!(s, UnitTestState::Ok));
    if finished || frame.0 > 96 {
        exit_event.send(if success {
            let _ = std::fs::remove_dir_all(&test_suit.tmp);
            AppExit::Success
        } else {
            AppExit::error()
        });
        for (name, test_stat) in test_suit.unit_tests.iter() {
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
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct ImageCopy;

pub struct TestPlugin;
impl Plugin for TestPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ExtractComponentPlugin::<ImageCopier>::default(),))
            .insert_resource(ClearColor(Color::WHITE))
            .add_systems(Startup, start_unit_test)
            .add_systems(Last, (wait_render, check_exit).chain());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, receive_image_from_buffer.after(RenderSet::Render));

        let node = CopyImageNode::from_world(render_app.world_mut());
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        if let Some(graph2d) = graph.get_sub_graph_mut(Core2d) {
            graph2d.add_node(ImageCopy, node);
            graph2d.add_node_edge(Node2d::Upscaling, ImageCopy);
        }
    }

    fn finish(&self, _app: &mut App) {
    }
}

pub struct TestPluginsSet;
impl PluginGroup for TestPluginsSet {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let group = DefaultPlugins.build();
        group
            .disable::<WinitPlugin>()
            .add(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(
                1.0 / 60.0,
            )))
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                close_when_requested: false,
            })
            .add(dway_ui_framework::UiFrameworkPlugin)
            .add(TestPlugin)
    }
}

#[derive(Debug)]
pub enum UnitTestState {
    Padding,
    Ok,
    Err(anyhow::Error),
}

#[derive(Resource)]
pub struct TestSuite {
    pub name: String,
    pub tmp: PathBuf,
    pub unit_tests: HashMap<String, UnitTestState>,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        let tmp = tempdir::TempDir::new(name).unwrap();
        Self {
            name: name.to_string(),
            tmp: tmp.into_path(),
            unit_tests: Default::default(),
        }
    }
}
