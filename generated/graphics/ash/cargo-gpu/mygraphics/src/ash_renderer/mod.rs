use crate::ash_renderer::device::MyDevice;
use crate::ash_renderer::renderer::MyRenderer;
use crate::ash_renderer::swapchain::MySwapchainManager;
use crate::util::enable_debug_layer;
use ash::util::read_spv;
use mygraphics_shaders::ShaderConstants;
use raw_window_handle::HasDisplayHandle;
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

pub mod buffer;
pub mod device;
pub mod global_descriptor_set;
pub mod render_pipeline;
pub mod renderer;
pub mod single_command_buffer;
pub mod swapchain;

pub fn main() -> anyhow::Result<()> {
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
            self.0 = Some(State::new(event_loop).unwrap());
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
    swapchain: MySwapchainManager,
}

impl State {
    fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Rust GPU - ash")
                    .with_inner_size(LogicalSize::new(1280, 720)),
            )?,
        );

        let extensions =
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?;
        let device = MyDevice::new(extensions, enable_debug_layer())?;
        let swapchain = MySwapchainManager::new(device.clone(), window.clone())?;
        let renderer = MyRenderer::new(device.clone(), swapchain.surface_format.format)?;
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
                self.swapchain.render(|frame| {
                    let extend = frame.extent;
                    let shader_constants = ShaderConstants {
                        width: extend.width,
                        height: extend.height,
                        time: self.start.elapsed().as_secs_f32(),
                    };
                    self.renderer.render_frame(frame, &shader_constants)
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

pub fn get_shaders() -> anyhow::Result<Vec<u32>> {
    // set in the build script
    const SPV_BYTES: &[u8] = include_bytes!(env!("SHADER_SPV_PATH"));
    Ok(read_spv(&mut std::io::Cursor::new(SPV_BYTES))?)
}
