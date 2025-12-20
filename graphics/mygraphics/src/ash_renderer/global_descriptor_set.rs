use crate::ash_renderer::device::MyDevice;
use ash::vk;
use std::sync::Arc;

pub struct GlobalDescriptorSetLayout {
    pub device: Arc<MyDevice>,
    pub layout: vk::DescriptorSetLayout,
}

impl GlobalDescriptorSetLayout {
    pub fn new(device: Arc<MyDevice>) -> anyhow::Result<Arc<Self>> {
        unsafe {
            Ok(Arc::new(Self {
                layout: device.create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&[
                        vk::DescriptorSetLayoutBinding::default()
                            .binding(0)
                            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                            .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
                            .descriptor_count(1),
                    ]),
                    None,
                )?,
                device,
            }))
        }
    }
}

impl Drop for GlobalDescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_set_layout(self.layout, None);
        }
    }
}

/// This implementation of descriptor sets is kept simple on purpose, even at the cost of being quite inefficient.
/// Don't use this as a reference on how it should be done!
pub struct GlobalDescriptorSet {
    pub layout: Arc<GlobalDescriptorSetLayout>,
    pub pool: vk::DescriptorPool,
    pub set: vk::DescriptorSet,
}

impl GlobalDescriptorSet {
    /// # Safety
    /// * `shader_constants` must not be dropped before `GlobalDescriptorSet` is dropped
    /// * you must only drop this `GlobalDescriptorSet` when it is unused, e.g. by GPU execution
    pub unsafe fn new(
        layout: &Arc<GlobalDescriptorSetLayout>,
        shader_constants: vk::Buffer,
    ) -> anyhow::Result<Self> {
        unsafe {
            let device = &layout.device;
            let pool = device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(&[vk::DescriptorPoolSize::default()
                        .ty(vk::DescriptorType::STORAGE_BUFFER)
                        .descriptor_count(1)])
                    .max_sets(1),
                None,
            )?;
            let set = device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout.layout]),
            )?[0];
            device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .buffer_info(&[vk::DescriptorBufferInfo::default()
                        .buffer(shader_constants)
                        .offset(0)
                        .range(vk::WHOLE_SIZE)])],
                &[],
            );
            Ok(Self {
                layout: layout.clone(),
                pool,
                set,
            })
        }
    }
}

impl Drop for GlobalDescriptorSet {
    fn drop(&mut self) {
        let device = &self.layout.device;
        unsafe {
            device
                .reset_descriptor_pool(self.pool, vk::DescriptorPoolResetFlags::empty())
                .ok();
            device.destroy_descriptor_pool(self.pool, None);
        }
    }
}
