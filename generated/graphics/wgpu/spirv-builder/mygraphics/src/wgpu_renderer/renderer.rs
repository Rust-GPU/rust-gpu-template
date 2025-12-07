use crate::wgpu_renderer::render_pipeline::MyRenderPipeline;
use mygraphics_shaders::ShaderConstants;
use wgpu::wgt::CommandEncoderDescriptor;
use wgpu::{
    Color, Device, LoadOp, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat, TextureView,
};

pub struct MyRenderer {
    pub device: Device,
    pub queue: Queue,
    pipeline: MyRenderPipeline,
}

impl MyRenderer {
    pub fn new(device: Device, queue: Queue, out_format: TextureFormat) -> anyhow::Result<Self> {
        Ok(Self {
            pipeline: MyRenderPipeline::new(&device, out_format)?,
            device,
            queue,
        })
    }

    pub fn render(&self, shader_constants: &ShaderConstants, output: TextureView) {
        let mut cmd = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("main draw"),
            });

        let mut rpass = cmd.begin_render_pass(&RenderPassDescriptor {
            label: Some("main renderpass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &output,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        self.pipeline.draw(&mut rpass, shader_constants);
        drop(rpass);

        self.queue.submit(std::iter::once(cmd.finish()));
    }
}
