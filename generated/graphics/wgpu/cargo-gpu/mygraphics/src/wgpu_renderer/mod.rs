use crate::wgpu_renderer::swapchain::MySwapchainManager;
use anyhow::Context;
use mygraphics_shaders::ShaderConstants;
use std::sync::Arc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

mod render_pipeline;
mod renderer;
mod swapchain;

pub fn main() -> anyhow::Result<()> {
    pollster::block_on(main_inner())
}

pub async fn main_inner() -> anyhow::Result<()> {
    // env_logger::init();
    let event_loop = EventLoop::new()?;
    // FIXME(eddyb) incomplete `winit` upgrade, follow the guides in:
    // https://github.com/rust-windowing/winit/releases/tag/v0.30.0
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop.create_window(
            winit::window::Window::default_attributes()
                .with_title("Rust GPU - wgpu")
                .with_inner_size(winit::dpi::LogicalSize::new(
                    f64::from(1280),
                    f64::from(720),
                )),
        )?,
    );

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
    let surface = instance.create_surface(window.clone())?;
    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface)).await?;

    let required_features = wgpu::Features::PUSH_CONSTANTS;
    let required_limits = wgpu::Limits {
        max_push_constant_size: 128,
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

    let mut swapchain = MySwapchainManager::new(adapter.clone(), device.clone(), window, surface);
    let renderer = renderer::MyRenderer::new(device, queue, swapchain.format())?;

    let start = std::time::Instant::now();
    let mut event_handler =
        move |event: Event<_>, event_loop_window_target: &ActiveEventLoop| match event {
            Event::AboutToWait => swapchain.render(|render_target| {
                renderer.render(
                    &ShaderConstants {
                        time: start.elapsed().as_secs_f32(),
                        width: render_target.texture().width(),
                        height: render_target.texture().height(),
                    },
                    render_target,
                );
            }),
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::KeyboardInput {
                        event:
                            winit::event::KeyEvent {
                                logical_key:
                                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
                                state: winit::event::ElementState::Pressed,
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => event_loop_window_target.exit(),
                    WindowEvent::Resized(_) => swapchain.should_recreate(),
                    _ => {}
                }
                Ok(())
            }
            _ => {
                event_loop_window_target.set_control_flow(ControlFlow::Poll);
                Ok(())
            }
        };

    // FIXME(eddyb) incomplete `winit` upgrade, follow the guides in:
    // https://github.com/rust-windowing/winit/releases/tag/v0.30.0
    #[allow(deprecated)]
    event_loop.run(move |event, event_loop_window_target| {
        event_handler(event, event_loop_window_target).unwrap();
    })?;
    Ok(())
}
