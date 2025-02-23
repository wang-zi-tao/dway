use std::{borrow::Cow, collections::HashMap, sync::Arc};

use anyhow::Result;
use bevy::{log, tasks::block_on};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Receiver, Sender},
};
use tracing::{debug, error, error_span, info, info_span, warn};
use wgpu::{MemoryHints, SurfaceTargetUnsafe};
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{self, ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::NamedKey,
    window::{Window, WindowId},
};

use crate::ClientOperate;

#[derive(Debug)]
enum AppEvent {
    Operate(ClientOperate),
    Event(WindowEvent),
    Quit,
}

async fn run_window(
    window: Window,
    render: Arc<AppRender>,
    mut rx: Receiver<AppEvent>,
) -> Result<()> {
    let mut size = window.inner_size();

    let instance = &render.wgpu_instance;
    let surface = instance.create_surface(&window)?;
    let adapter = (instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        // Request an adapter which can render to our surface
        compatible_surface: Some(&surface),
    }))
    .await
    .unwrap();

    // Create the logical device and command queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: MemoryHints::Performance,
            },
            None,
        )
        .await?;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(swapchain_format.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let mut config = surface
        .get_default_config(&adapter, size.width, size.height)
        .unwrap();
    surface.configure(&device, &config);

    info!("window launched");
    let mut frame_count = 0;

    while let Some(request) = rx.recv().await {
        let _span = error_span!("window event", event = ?request).entered();
        match request {
            AppEvent::Operate(o) => match o {
                ClientOperate::Quit => {
                    break;
                }
                _ => {}
            },
            AppEvent::Event(e) => match e {
                WindowEvent::RedrawRequested => {
                    let _span = info_span!("redraw").entered();
                    let frame = surface.get_current_texture()?;
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        rpass.set_pipeline(&render_pipeline);
                        rpass.draw(0..3, 0..1);
                    }

                    queue.submit(Some(encoder.finish()));
                    frame.present();
                    debug!("present");

                    // window.request_redraw();
                }
                WindowEvent::CloseRequested => {
                    break;
                }
                _ => {}
            },
            AppEvent::Quit => {
                break;
            }
        }
    }

    Ok(())
}

struct App {
    windows: HashMap<WindowId, Sender<AppEvent>>,
    render: Arc<AppRender>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            windows: Default::default(),
            render: Arc::new(Default::default()),
        }
    }
}

pub struct AppRender {
    wgpu_instance: wgpu::Instance,
}

impl Default for AppRender {
    fn default() -> Self {
        let wgpu_instance = Default::default();
        Self { wgpu_instance }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.windows.is_empty() {
            let window = event_loop
                .create_window(Window::default_attributes())
                .unwrap();
            let render = self.render.clone();
            let (tx, rx) = channel(256);
            self.windows.insert(window.id(), tx);
            tokio::spawn(async move {
                if let Err(e) = run_window(window, render, rx).await {
                    println!("{e}");
                };
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let tx = &self.windows[&id];
        if let Err(_e) = tx.blocking_send(AppEvent::Event(event.clone())) {
            self.windows.remove(&id);
        }
        match &event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: winit::keyboard::Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            _ => {}
        }
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }
}

pub fn winit_app() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
