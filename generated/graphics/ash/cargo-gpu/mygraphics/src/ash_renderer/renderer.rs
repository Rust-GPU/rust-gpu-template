use crate::ash_renderer::device::MyDevice;
use crate::ash_renderer::get_shaders;
use crate::ash_renderer::global_descriptor_set::{GlobalDescriptorSet, GlobalDescriptorSetLayout};
use crate::ash_renderer::render_pipeline::MyRenderPipelineManager;
use crate::ash_renderer::single_command_buffer::SingleCommandBuffer;
use crate::ash_renderer::swapchain::DrawFrame;
use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{AllocationCreateDesc, AllocationScheme};
use mygraphics_shaders::ShaderConstants;
use std::sync::Arc;

/// The renderer manages our command buffer and submits the commands, using [`MyRenderPipeline`] for drawing.
pub struct MyRenderer {
    pub device: Arc<MyDevice>,
    pub global_descriptor_set_layout: Arc<GlobalDescriptorSetLayout>,
    pub pipeline: MyRenderPipelineManager,
    pub command: SingleCommandBuffer,
}

impl MyRenderer {
    pub fn new(device: Arc<MyDevice>, out_format: vk::Format) -> anyhow::Result<Self> {
        let global_descriptor_set_layout = GlobalDescriptorSetLayout::new(device.clone())?;
        let pipeline = MyRenderPipelineManager::new(
            device.clone(),
            global_descriptor_set_layout.clone(),
            out_format,
            get_shaders()?,
        )?;
        let command = SingleCommandBuffer::new(pipeline.device.clone())?;
        Ok(Self {
            device,
            global_descriptor_set_layout,
            pipeline,
            command,
        })
    }

    pub fn render_frame(
        &mut self,
        frame: DrawFrame,
        shader_constants: &ShaderConstants,
    ) -> anyhow::Result<()> {
        unsafe {
            let device = &self.device;
            let pipeline = self.pipeline.get_pipeline()?;
            let cmd = self.command.cmd;

            let buffer = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(size_of::<ShaderConstants>() as u64)
                    .usage(vk::BufferUsageFlags::STORAGE_BUFFER),
                None,
            )?;
            let mut allocation = device.borrow_allocator().allocate(&AllocationCreateDesc {
                name: "ShaderConstants",
                requirements: device.get_buffer_memory_requirements(buffer),
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;
            {
                let mapped =
                    &mut allocation.mapped_slice_mut().unwrap()[..size_of::<ShaderConstants>()];
                mapped.copy_from_slice(bytemuck::bytes_of(shader_constants));
            }
            device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;

            let descriptor_set =
                GlobalDescriptorSet::new(&self.global_descriptor_set_layout, buffer)?;

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
                pipeline.render(device, cmd, frame.image_view, frame.extent, &descriptor_set)?;
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

            // free resources of the frame
            // For a production renderer, you'd obviously don't want to immediately wait for the previous frame to
            // finish rendering, but start recording the next one already while waiting. For simplicity's sake,
            // we just wait immediately.
            self.device.device_wait_idle()?;
            drop(descriptor_set);
            self.device.destroy_buffer(buffer, None);
            drop(allocation);
            Ok(())
        }
    }
}
