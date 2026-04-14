use crate::wgpu_renderer::renderer::MyRenderer;
use crate::wgpu_renderer::swapchain::MySwapchainManager;
use anyhow::Context;
use mygraphics_shaders::ShaderConstants;
use pollster::block_on;
use std::sync::Arc;
use std::time::Instant;
use winit::event_loop::EventLoop;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

mod render_pipeline;
mod renderer;
mod swapchain;

pub fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
pub struct App(Option<State>);

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.0.is_none() {
            self.0 = Some(block_on(State::new(event_loop)).unwrap());
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let state = self.0.as_mut().unwrap();
        state.window_event(event_loop, id, event).unwrap();
    }
}

struct State {
    start: Instant,
    window: Arc<Window>,
    renderer: MyRenderer,
    swapchain: MySwapchainManager<'static>,
}

impl State {
    async fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Rust GPU - wgpu")
                    .with_inner_size(LogicalSize::new(1280, 720)),
            )?,
        );

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(
                Box::new(event_loop.owned_display_handle()),
            ));
        let surface = instance.create_surface(window.clone())?;
        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface)).await?;

        let required_features = wgpu::Features::IMMEDIATES;
        let required_limits = wgpu::Limits {
            max_immediate_size: 128,
            ..Default::default()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features,
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: Default::default(),
            })
            .await
            .context("Failed to create device")?;

        let swapchain = MySwapchainManager::new(
            instance.clone(),
            adapter.clone(),
            device.clone(),
            window.clone(),
            surface,
        );
        let renderer = MyRenderer::new(device, queue, swapchain.format())?;
        Ok(Self {
            start: Instant::now(),
            window,
            swapchain,
            renderer,
        })
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        match event {
            WindowEvent::RedrawRequested => {
                self.swapchain.render(|render_target| {
                    self.renderer.render(
                        &ShaderConstants {
                            time: self.start.elapsed().as_secs_f32(),
                            width: render_target.texture().width(),
                            height: render_target.texture().height(),
                        },
                        render_target,
                    )
                })?;
                self.window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            }
            | WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => self.swapchain.should_recreate(),
            _ => (),
        }
        Ok(())
    }
}
