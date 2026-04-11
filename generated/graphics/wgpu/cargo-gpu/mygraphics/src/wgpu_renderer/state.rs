use anyhow::Context;
use mygraphics_shaders::ShaderConstants;
use std::sync::Arc;
use wgpu::{
    CurrentSurfaceTexture, Features, Instance, InstanceDescriptor, Limits, Surface, TextureFormat,
};
use winit::dpi::PhysicalSize;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use crate::wgpu_renderer::renderer::MyRenderer;

pub struct State {
    instance: Instance,

    window: Arc<Window>,

    size: PhysicalSize<u32>,
    surface: Surface<'static>,
    surface_format: TextureFormat,

    renderer: MyRenderer,
}

impl State {
    pub async fn try_new(
        display_handle: OwnedDisplayHandle,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let instance = Instance::new(InstanceDescriptor::new_with_display_handle_from_env(
            Box::new(display_handle),
        ));
        let surface = instance.create_surface(window.clone())?;
        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, Some(&surface)).await?;

        let required_features = Features::IMMEDIATES;
        let required_limits = Limits {
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

        let size = window.inner_size();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let renderer = MyRenderer::new(device, queue, surface_format)?;
        
        let state = State {
            instance,
            window,
            size,
            surface,
            surface_format,
            renderer,
        };
        state.configure_surface();

        Ok(state)
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn render(&mut self, time: f32) {
        let surface_texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return,
            CurrentSurfaceTexture::Suboptimal(_) | CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            CurrentSurfaceTexture::Validation => {
                unreachable!("No error scope registered, so validation errors will panic")
            }
            CurrentSurfaceTexture::Lost => {
                self.surface = self.instance.create_surface(self.window.clone()).unwrap();
                self.configure_surface();
                return;
            }
        };

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let PhysicalSize { width, height } = self.size;
        self.renderer.render(
            &ShaderConstants {
                width,
                height,
                time,
            },
            texture_view,
        );

        self.window.pre_present_notify();
        surface_texture.present();
    }

    pub fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };

        self.surface
            .configure(&self.renderer.device, &surface_config);
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.configure_surface();
    }
}
