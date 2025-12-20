use crate::ash_renderer::device::MyDevice;
use anyhow::Context;
use ash::vk;
use mygraphics_shaders::ShaderConstants;
use std::sync::Arc;

/// Manages the creation and recreation of [`MyRenderPipeline`], whenever new shader code ([`Self::set_shader_code`])
/// is submitted
pub struct MyRenderPipelineManager {
    pub device: Arc<MyDevice>,
    color_out_format: vk::Format,
    shader_code: Vec<u32>,
    pipeline: Option<MyRenderPipeline>,
    should_recreate: bool,
}

pub struct MyRenderPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
}

impl MyRenderPipelineManager {
    pub fn new(
        device: Arc<MyDevice>,
        color_out_format: vk::Format,
        shader_code: Vec<u32>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            device,
            color_out_format,
            shader_code,
            pipeline: None,
            should_recreate: true,
        })
    }

    #[inline]
    pub fn set_shader_code(&mut self, shader_code: Vec<u32>) {
        self.shader_code = shader_code;
        self.should_recreate();
    }

    #[inline]
    pub fn should_recreate(&mut self) {
        self.should_recreate = true;
    }

    pub fn get_pipeline(&mut self) -> anyhow::Result<&MyRenderPipeline> {
        if self.should_recreate {
            self.rebuild_pipeline()?;
        }
        Ok(self.pipeline.as_ref().unwrap())
    }

    /// Update shaders and rebuild the pipeline
    fn rebuild_pipeline(&mut self) -> anyhow::Result<()> {
        unsafe {
            self.destroy_pipeline()?;

            let shader_module = self.device.create_shader_module(
                &vk::ShaderModuleCreateInfo::default().code(&self.shader_code),
                None,
            )?;

            let pipeline_layout = self.device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default().push_constant_ranges(&[
                    vk::PushConstantRange::default()
                        .offset(0)
                        .size(size_of::<ShaderConstants>() as u32)
                        .stage_flags(vk::ShaderStageFlags::ALL),
                ]),
                None,
            )?;

            let mut pipelines = self
                .device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::GraphicsPipelineCreateInfo::default()
                        .stages(&[
                            vk::PipelineShaderStageCreateInfo {
                                module: shader_module,
                                p_name: c"main_vs".as_ptr(),
                                stage: vk::ShaderStageFlags::VERTEX,
                                ..Default::default()
                            },
                            vk::PipelineShaderStageCreateInfo {
                                module: shader_module,
                                p_name: c"main_fs".as_ptr(),
                                stage: vk::ShaderStageFlags::FRAGMENT,

                                ..Default::default()
                            },
                        ])
                        .vertex_input_state(&vk::PipelineVertexInputStateCreateInfo::default())
                        .input_assembly_state(&vk::PipelineInputAssemblyStateCreateInfo {
                            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                            ..Default::default()
                        })
                        .rasterization_state(&vk::PipelineRasterizationStateCreateInfo {
                            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                            line_width: 1.0,
                            ..Default::default()
                        })
                        .multisample_state(&vk::PipelineMultisampleStateCreateInfo {
                            rasterization_samples: vk::SampleCountFlags::TYPE_1,
                            ..Default::default()
                        })
                        .depth_stencil_state(&vk::PipelineDepthStencilStateCreateInfo::default())
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::default().attachments(&[
                                vk::PipelineColorBlendAttachmentState {
                                    blend_enable: 0,
                                    src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
                                    dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
                                    color_blend_op: vk::BlendOp::ADD,
                                    src_alpha_blend_factor: vk::BlendFactor::ZERO,
                                    dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                                    alpha_blend_op: vk::BlendOp::ADD,
                                    color_write_mask: vk::ColorComponentFlags::RGBA,
                                },
                            ]),
                        )
                        .dynamic_state(
                            &vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
                                vk::DynamicState::VIEWPORT,
                                vk::DynamicState::SCISSOR,
                            ]),
                        )
                        .viewport_state(
                            &vk::PipelineViewportStateCreateInfo::default()
                                .scissor_count(1)
                                .viewport_count(1),
                        )
                        .layout(pipeline_layout)
                        .push_next(
                            &mut vk::PipelineRenderingCreateInfo::default()
                                .color_attachment_formats(&[self.color_out_format]),
                        )],
                    None,
                )
                .map_err(|(_, e)| e)
                .context("Unable to create graphics pipeline")?;

            // A single `pipeline_info` results in a single pipeline.
            assert_eq!(pipelines.len(), 1);
            self.pipeline = pipelines.pop().map(|pipeline| MyRenderPipeline {
                pipeline,
                pipeline_layout,
            });

            // shader modules are allowed to be deleted after the pipeline has been created
            self.device.destroy_shader_module(shader_module, None);
            Ok(())
        }
    }

    unsafe fn destroy_pipeline(&mut self) -> anyhow::Result<()> {
        unsafe {
            if let Some(pipeline) = self.pipeline.take() {
                // Figuring out when the pipeline stops being used is hard, so we take this shortcut
                self.device.device_wait_idle()?;

                self.device.destroy_pipeline(pipeline.pipeline, None);
                self.device
                    .destroy_pipeline_layout(pipeline.pipeline_layout, None);
            }
            Ok(())
        }
    }
}

impl MyRenderPipeline {
    pub fn render(
        &self,
        device: &MyDevice,
        cmd: vk::CommandBuffer,
        color_out: vk::ImageView,
        extent: vk::Extent2D,
        push_constants: ShaderConstants,
    ) -> anyhow::Result<()> {
        unsafe {
            let render_area = vk::Rect2D {
                offset: vk::Offset2D::default(),
                extent,
            };

            device.cmd_begin_rendering(
                cmd,
                &vk::RenderingInfo::default()
                    .render_area(render_area)
                    .layer_count(1)
                    .color_attachments(&[vk::RenderingAttachmentInfo::default()
                        .image_view(color_out)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0.0, 0.0, 0.0, 0.0],
                            },
                        })
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)]),
            );
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_set_viewport(
                cmd,
                0,
                &[vk::Viewport {
                    // contains a y-flip
                    x: 0.0,
                    y: extent.height as f32,
                    width: extent.width as f32,
                    height: -(extent.height as f32),
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            device.cmd_set_scissor(cmd, 0, &[render_area]);
            device.cmd_push_constants(
                cmd,
                self.pipeline_layout,
                vk::ShaderStageFlags::ALL,
                0,
                bytemuck::bytes_of(&push_constants),
            );
            device.cmd_draw(cmd, 3, 1, 0, 0);
            device.cmd_end_rendering(cmd);
            Ok(())
        }
    }
}

impl Drop for MyRenderPipelineManager {
    fn drop(&mut self) {
        unsafe {
            self.destroy_pipeline().ok();
        }
    }
}
