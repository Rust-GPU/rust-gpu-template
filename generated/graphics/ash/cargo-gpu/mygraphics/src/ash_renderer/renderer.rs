use crate::ash_renderer::device::MyDevice;
use crate::ash_renderer::render_pipeline::MyRenderPipelineManager;
use crate::ash_renderer::single_command_buffer::SingleCommandBuffer;
use crate::ash_renderer::swapchain::DrawFrame;
use ash::vk;
use mygraphics_shaders::ShaderConstants;
use std::sync::Arc;

/// The renderer manages our command buffer and submits the commands, using [`MyRenderPipeline`] for drawing.
pub struct MyRenderer {
    pub device: Arc<MyDevice>,
    pub pipeline: MyRenderPipelineManager,
    pub command: SingleCommandBuffer,
}

impl MyRenderer {
    pub fn new(pipeline: MyRenderPipelineManager) -> anyhow::Result<Self> {
        Ok(Self {
            command: SingleCommandBuffer::new(pipeline.device.clone())?,
            device: pipeline.device.clone(),
            pipeline,
        })
    }

    pub fn render_frame(
        &mut self,
        frame: DrawFrame,
        push_constants: ShaderConstants,
    ) -> anyhow::Result<()> {
        unsafe {
            let device = &self.device;
            let pipeline = self.pipeline.get_pipeline()?;
            let cmd = self.command.cmd;

            device.reset_command_pool(self.command.pool, vk::CommandPoolResetFlags::default())?;

            {
                device.begin_command_buffer(
                    cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )?;
                device.cmd_pipeline_barrier2(
                    cmd,
                    &vk::DependencyInfo::default().image_memory_barriers(&[
                        vk::ImageMemoryBarrier2::default()
                            .image(frame.image)
                            .src_access_mask(vk::AccessFlags2::NONE)
                            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .old_layout(vk::ImageLayout::UNDEFINED)
                            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .subresource_range(
                                vk::ImageSubresourceRange::default()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .base_mip_level(0)
                                    .level_count(1)
                                    .base_array_layer(0)
                                    .layer_count(1),
                            ),
                    ]),
                );
                pipeline.render(device, cmd, frame.image_view, frame.extent, push_constants)?;
                device.cmd_pipeline_barrier2(
                    cmd,
                    &vk::DependencyInfo::default().image_memory_barriers(&[
                        vk::ImageMemoryBarrier2::default()
                            .image(frame.image)
                            .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                            .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                            .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .dst_access_mask(vk::AccessFlags2::NONE)
                            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
                            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                            .subresource_range(
                                vk::ImageSubresourceRange::default()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .base_mip_level(0)
                                    .level_count(1)
                                    .base_array_layer(0)
                                    .layer_count(1),
                            ),
                    ]),
                );
                device.end_command_buffer(cmd)?;
            }

            device.queue_submit2(
                device.main_queue,
                &[vk::SubmitInfo2::default()
                    .wait_semaphore_infos(&[vk::SemaphoreSubmitInfo::default()
                        .semaphore(frame.acquire_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)])
                    .command_buffer_infos(&[
                        vk::CommandBufferSubmitInfo::default().command_buffer(cmd)
                    ])
                    .signal_semaphore_infos(&[vk::SemaphoreSubmitInfo::default()
                        .semaphore(frame.draw_finished_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)])],
                frame.draw_finished_fence,
            )?;
            Ok(())
        }
    }
}
