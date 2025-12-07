use mygraphics_shaders::ShaderConstants;
use wgpu::{
    ColorTargetState, ColorWrites, Device, FragmentState, FrontFace, MultisampleState,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, PushConstantRange,
    RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptorPassthrough,
    ShaderRuntimeChecks, ShaderStages, TextureFormat, VertexState,
};

pub struct MyRenderPipeline {
    pipeline: RenderPipeline,
}

impl MyRenderPipeline {
    pub fn new(device: &Device, out_format: TextureFormat) -> anyhow::Result<Self> {
        // Workaround in wgpu 27.0.1 where the macro expansion of `include_spirv_raw!` doesn't compile
        // see https://github.com/gfx-rs/wgpu/pull/8250
        // let module = unsafe {
        //     device.create_shader_module_passthrough(include_spirv_raw!(env!("SHADER_SPV_PATH")))
        // };
        let module = unsafe {
            device.create_shader_module_passthrough(ShaderModuleDescriptorPassthrough {
                label: Some(env!("SHADER_SPV_PATH")),
                entry_point: "".to_owned(),
                num_workgroups: (0, 0, 0),
                runtime_checks: ShaderRuntimeChecks::unchecked(),
                spirv: Some(wgpu::util::make_spirv_raw(include_bytes!(env!(
                    "SHADER_SPV_PATH"
                )))),
                dxil: None,
                msl: None,
                hlsl: None,
                glsl: None,
                wgsl: None,
            })
        };

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("MyRenderPipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<ShaderConstants>() as u32,
            }],
        });

        Ok(Self {
            pipeline: device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("MyRenderPipeline"),
                layout: Some(&layout),
                vertex: VertexState {
                    module: &module,
                    entry_point: Some("main_vs"),
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
                    entry_point: Some("main_fs"),
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
