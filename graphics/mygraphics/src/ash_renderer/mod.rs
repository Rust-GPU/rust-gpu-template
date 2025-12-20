use crate::ash_renderer::device::MyDevice;
use crate::ash_renderer::renderer::MyRenderer;
use crate::ash_renderer::swapchain::MySwapchainManager;
use crate::util::enable_debug_layer;
use ash::util::read_spv;
use mygraphics_shaders::ShaderConstants;
use raw_window_handle::HasDisplayHandle as _;
use winit::event_loop::ActiveEventLoop;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub mod device;
pub mod global_descriptor_set;
pub mod render_pipeline;
pub mod renderer;
pub mod single_command_buffer;
pub mod swapchain;

pub fn main() -> anyhow::Result<()> {
    // runtime setup
    let event_loop = EventLoop::new()?;
    // FIXME(eddyb) incomplete `winit` upgrade, follow the guides in:
    // https://github.com/rust-windowing/winit/releases/tag/v0.30.0
    #[allow(deprecated)]
    let window = event_loop.create_window(
        winit::window::Window::default_attributes()
            .with_title("Rust GPU - ash")
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(1280),
                f64::from(720),
            )),
    )?;

    let extensions = ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?;
    let device = MyDevice::new(extensions, enable_debug_layer())?;
    let mut swapchain = MySwapchainManager::new(device.clone(), window)?;
    let mut renderer = MyRenderer::new(device.clone(), swapchain.surface_format.format)?;

    let start = std::time::Instant::now();
    let mut event_handler =
        move |event: Event<_>, event_loop_window_target: &ActiveEventLoop| match event {
            Event::AboutToWait => swapchain.render(|frame| {
                let extent = frame.extent;
                let shader_constants = ShaderConstants {
                    width: extent.width,
                    height: extent.height,
                    time: start.elapsed().as_secs_f32(),
                };

                renderer.render_frame(frame, &shader_constants)?;
                Ok(())
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
                    WindowEvent::Resized(_) => {
                        swapchain.should_recreate();
                    }
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

pub fn get_shaders() -> anyhow::Result<Vec<u32>> {
    // set in the build script
    const SPV_BYTES: &[u8] = include_bytes!(env!("SHADER_SPV_PATH"));
    Ok(read_spv(&mut std::io::Cursor::new(SPV_BYTES))?)
}
