use anyhow::Context;
use std::sync::Arc;
use wgpu::{Adapter, Device, Surface, SurfaceError, TextureFormat, TextureView};
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct MySwapchainManager<'a> {
    adapter: Adapter,
    device: Device,
    window: Arc<Window>,
    surface: Surface<'a>,
    format: TextureFormat,

    // state below
    active: Option<ActiveConfiguration>,
    should_recreate: bool,
}

pub struct ActiveConfiguration {
    size: PhysicalSize<u32>,
}

impl<'a> MySwapchainManager<'a> {
    pub fn new(
        adapter: Adapter,
        device: Device,
        window: Arc<Window>,
        surface: Surface<'a>,
    ) -> Self {
        let caps = surface.get_capabilities(&adapter);
        Self {
            adapter,
            device,
            window,
            surface,
            format: caps.formats[0],
            active: None,
            should_recreate: true,
        }
    }

    #[inline]
    pub fn should_recreate(&mut self) {
        self.should_recreate = true;
    }

    pub fn format(&self) -> TextureFormat {
        self.format
    }

    pub fn render<R>(&mut self, f: impl FnOnce(TextureView) -> R) -> anyhow::Result<R> {
        let size = self.window.inner_size();
        if let Some(active) = &self.active {
            if active.size != size {
                self.should_recreate();
            }
        } else {
            self.should_recreate();
        }

        const RECREATE_ATTEMPTS: u32 = 10;
        for _ in 0..RECREATE_ATTEMPTS {
            if self.should_recreate {
                self.should_recreate = false;
                self.configure_surface(size)?;
            }

            match self.surface.get_current_texture() {
                Ok(surface_texture) => {
                    if surface_texture.suboptimal {
                        self.should_recreate = true;
                    }
                    let output_view =
                        surface_texture
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor {
                                format: Some(self.format),
                                ..wgpu::TextureViewDescriptor::default()
                            });
                    let r = f(output_view);
                    surface_texture.present();
                    return Ok(r);
                }
                Err(SurfaceError::Outdated | SurfaceError::Lost) => {
                    self.should_recreate = true;
                }
                Err(e) => {
                    anyhow::bail!("get_current_texture() failed: {e}")
                }
            };
        }
        anyhow::bail!(
            "looped {RECREATE_ATTEMPTS} times trying to acquire swapchain image and failed repeatedly!"
        );
    }

    fn configure_surface(&mut self, size: PhysicalSize<u32>) -> anyhow::Result<()> {
        let mut surface_config = self
            .surface
            .get_default_config(&self.adapter, size.width, size.height)
            .with_context(|| {
                format!(
                    "Incompatible adapter for surface, returned capabilities: {:?}",
                    self.surface.get_capabilities(&self.adapter)
                )
            })?;

        // force srgb surface format
        surface_config.view_formats.push(self.format);
        // limit framerate to vsync
        surface_config.present_mode = wgpu::PresentMode::AutoVsync;
        self.surface.configure(&self.device, &surface_config);

        self.active = Some(ActiveConfiguration { size });
        Ok(())
    }
}
