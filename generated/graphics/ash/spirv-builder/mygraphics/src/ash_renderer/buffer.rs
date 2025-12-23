use crate::ash_renderer::device::MyDevice;
use ash::vk;
use bytemuck::NoUninit;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use std::borrow::Cow;
use std::sync::Arc;

pub struct MyBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Allocation,
    pub name: String,
    destroyed: bool,
}

#[derive(Clone)]
pub struct BufferCreateInfo<'a> {
    pub usage: vk::BufferUsageFlags,
    pub location: MemoryLocation,
    pub name: Option<Cow<'a, str>>,
}

impl MyBuffer {
    pub fn from_data<T: NoUninit>(
        device: &Arc<MyDevice>,
        info: BufferCreateInfo<'_>,
        data: &T,
    ) -> anyhow::Result<Self> {
        Self::from_slice(device, info, bytemuck::bytes_of(data))
    }

    pub fn from_slice<T: NoUninit>(
        device: &Arc<MyDevice>,
        info: BufferCreateInfo<'_>,
        data: &[T],
    ) -> anyhow::Result<Self> {
        unsafe {
            let buffer = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(size_of_val(data) as u64)
                    .usage(vk::BufferUsageFlags::STORAGE_BUFFER),
                None,
            )?;
            let name = info.name.map(|a| a.into_owned()).unwrap_or_default();
            let mut allocation = device.borrow_allocator().allocate(&AllocationCreateDesc {
                name: &name,
                requirements: device.get_buffer_memory_requirements(buffer),
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;
            device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;
            let mapped = &mut allocation.mapped_slice_mut().unwrap()[..size_of_val(data)];
            mapped.copy_from_slice(bytemuck::cast_slice(data));
            Ok(Self {
                buffer,
                allocation,
                name,
                destroyed: false,
            })
        }
    }

    /// Destroy this buffer
    ///
    /// # Safety
    /// Buffer must not be in use
    pub unsafe fn destroy(&mut self, device: &Arc<MyDevice>) {
        if !self.destroyed {
            self.destroyed = true;
            unsafe {
                device.destroy_buffer(self.buffer, None);
            }
        }
    }
}

impl Drop for MyBuffer {
    fn drop(&mut self) {
        if !self.destroyed {
            panic!("dropping Buffer {} without destroying it", &self.name);
        }
    }
}
