use ash::{vk, Device, Instance};
use std::sync::Arc;
pub struct VulkanBuffer {
    buffer : vk::Buffer,
    memory : vk::DeviceMemory,
    device : Arc<Device>
}
impl VulkanBuffer {
    pub fn new(device : Arc<Device>,instance: &Instance, physical_device: vk::PhysicalDevice,  size : vk::DeviceSize, usage : vk::BufferUsageFlags, properties : vk::MemoryPropertyFlags) -> Result<Self, String> {
        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size: size,
            usage: usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let  buffer = unsafe { 
            device.create_buffer(&buffer_info, None).unwrap()
        };
        let mem_req = unsafe { 
            device.get_buffer_memory_requirements(buffer)
        };
        let mem_prop = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        let mut mem_index : u32 = 0;
        for i in 0..mem_prop.memory_type_count {
            if mem_req.memory_type_bits & (i << 1) != 0 && mem_prop.memory_types[i as usize].property_flags.contains(properties) {
                mem_index = i;
                break;
            }
        }
        let memory_allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            memory_type_index: mem_index,
            allocation_size: mem_req.size,
            ..Default::default()
        };
        let memory = unsafe {
            device.allocate_memory(&memory_allocate_info, None).expect("Failed to allocate buffer memory!")
        };
        unsafe { 
            device.bind_buffer_memory(buffer, memory, 0).expect("Failed to bind buffer memory!")
        };
        Ok(Self { buffer, memory, device })
    }
    pub fn buffer(&self) -> &vk::Buffer {
        &self.buffer
    }
}

pub trait BufferMethods {
    fn copy_buffer(&self, command_pool : vk::CommandPool, queue: vk::Queue, dst_buffer : vk::Buffer, size : vk::DeviceSize) -> ();
}
impl BufferMethods for VulkanBuffer {
    fn copy_buffer(&self,command_pool : vk::CommandPool, queue: vk::Queue, dst_buffer : vk::Buffer, size : vk::DeviceSize) -> () {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            command_pool: command_pool,
            command_buffer_count: 1,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };
        let command_buffer = unsafe { 
            self.device.allocate_command_buffers(&command_buffer_alloc_info).expect("Failed to allocate command buffer for copy buffers!")
        };
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe {
            self.device.begin_command_buffer(command_buffer[0], &begin_info).expect("Failed to begin copy command buffer!")
        };
        let copy_region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: size
        };
        unsafe {
            self.device.cmd_copy_buffer(command_buffer[0], *self.buffer(), dst_buffer, &[copy_region]);
            self.device.end_command_buffer(command_buffer[0]).expect("Failed to end copy command buffer!")
        };
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            command_buffer_count: 1,
            p_command_buffers: command_buffer.as_ptr(),
            ..Default::default()
        };
        let fence_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            ..Default::default()
        };
        let fence = unsafe {
            self.device.create_fence(&fence_info, None).unwrap()
        };
        unsafe {
            self.device.queue_submit(queue, &[submit_info], fence).expect("Failed to submit copy buffer!");
            self.device.wait_for_fences(&[fence], true, u64::MAX).expect("Failed to wait copying buffer!");
            self.device.reset_fences(&[fence]).unwrap();
            self.device.free_command_buffers(command_pool, &command_buffer);
            self.device.destroy_fence(fence, None);
        };
    }
}
impl Drop for VulkanBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None)
        }
    }
}
