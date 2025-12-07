use mygraphics_shaders::ShaderConstants;
use wgpu::{
    include_spirv, ColorTargetState, ColorWrites, Device, FragmentState, FrontFace,
    MultisampleState, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, TextureFormat, VertexState,
};

pub struct MyRenderPipeline {
    pipeline: RenderPipeline,
}

impl MyRenderPipeline {
    pub fn new(device: &Device, out_format: TextureFormat) -> anyhow::Result<Self> {
        let module = device.create_shader_module(include_spirv!(env!("SHADER_SPV_PATH")));
        Ok(Self {
            pipeline: device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("MyRenderPipeline"),
                layout: None,
                vertex: VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: out_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            }),
        })
    }

    pub fn draw(&self, rpass: &mut RenderPass<'_>, shader_constants: &ShaderConstants) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_push_constants(
            ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(shader_constants),
        );
        rpass.draw(0..3, 0..1);
    }
}
