use crate::wgpu_renderer::app::App;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod render_pipeline;
mod renderer;
mod state;

pub fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
