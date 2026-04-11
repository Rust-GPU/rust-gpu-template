use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use crate::wgpu_renderer::state::State;

pub(crate) struct App {
    start: Instant,
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        let start = std::time::Instant::now();

        App { start, state: None }
    }

    pub fn try_resume(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let attribs = Window::default_attributes()
            .with_title("Rust GPU - wgpu")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = event_loop.create_window(attribs).map(Arc::new)?;

        let state = pollster::block_on(State::try_new(
            event_loop.owned_display_handle(),
            window.clone(),
        ))?;
        self.state = Some(state);

        window.request_redraw();

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(e) = self.try_resume(event_loop) {
            eprintln!("Failed to resume: {e:?}");
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            eprintln!("Failed to retrieve app state.");
            event_loop.exit();
            return;
        };

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render(self.start.elapsed().as_secs_f32());
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.resize(size);
            }
            _ => (),
        }
    }
}
