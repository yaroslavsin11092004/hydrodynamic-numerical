use ash::vk;
use std::sync::{Arc, RwLock};
use crate::{engine::ripple_view_context};
pub trait RippleElementStatic {
    fn send_framebuffer_resize(&mut self);
    fn get_secondary_command_buffers(&mut self) -> Vec<vk::CommandBuffer>;
}
pub struct RippleBase {
    pub(crate) core: Arc<ripple_view_context::RippleViewCore>,
    pub(crate) state: Arc<RwLock<ripple_view_context::RippleViewState>>
}
impl RippleBase {
    pub fn new(core: Arc<ripple_view_context::RippleViewCore>, state: Arc<RwLock<ripple_view_context::RippleViewState>>) -> Arc<Self> {
        Arc::new(Self { core, state })
    }
    pub fn read_shader_file(filepath: &str) -> std::io::Result<Vec<u32>> {
        let bytes = std::fs::read(filepath).map_err(|e| format!("Failed to read shader file '{}' : {}", filepath, e)).unwrap();
        let words = bytes.chunks_exact(4).map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap())).collect();
        Ok(words)
    }
    pub fn create_shader_module(&self, code: &[u32]) -> vk::ShaderModule {
        let shader_info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
            code_size: code.len() * 4,
            p_code: code.as_ptr(),
            ..Default::default()
        };
        let module = unsafe {
            self.core.device.create_shader_module(&shader_info, None).expect("Failed to create shader!")
        };
        module
    }
    pub fn find_memory_type(&self, filter: u32, properties: vk::MemoryPropertyFlags) -> u32 {
        let physical_device_memory_properties = unsafe {
            self.core.instance.get_physical_device_memory_properties(self.core.physical_device)
        };
        for i in 0..physical_device_memory_properties.memory_type_count {
            if physical_device_memory_properties.memory_types[i as usize].property_flags.contains(properties) && (filter & (1 << i)) != 0 {
                return i as u32;
            }
        }
        panic!("Failed to find suitable memory type!");
    }
    pub fn create_buffer(&self, size: vk::DeviceSize, usage: vk::BufferUsageFlags, properties: vk::MemoryPropertyFlags) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size:size, 
            usage: usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer = unsafe {
            self.core.device.create_buffer(&buffer_info, None).expect("Failed to create buffer!")
        };
        let mem_requirements = unsafe {
            self.core.device.get_buffer_memory_requirements(buffer)
        };
        let allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            memory_type_index: self.find_memory_type(mem_requirements.memory_type_bits, properties),
            allocation_size: mem_requirements.size,
            ..Default::default()
        };
        let memory = unsafe {
            self.core.device.allocate_memory(&allocate_info, None).expect("Failed to allocate buffer memory!")
        };
        unsafe {
            self.core.device.bind_buffer_memory(buffer, memory, 0).expect("Failed to bind buffer memory!")
        };
        (buffer, memory)
    }
    pub fn copy_buffer(&self, src_buffer: vk::Buffer, dst_buffer: vk::Buffer, size: vk::DeviceSize) {
        let command_allocate_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            command_pool: self.state.read().unwrap().command_pool,
            command_buffer_count: 1,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };
        let command_buffer = unsafe {
            self.core.device.allocate_command_buffers(&command_allocate_info).expect("Failed to allocate command buffer for copy!")
        };
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe {
            self.core.device.begin_command_buffer(command_buffer[0], &begin_info).expect("Failed to begin copy command buffer!")
        };
        let copy_region = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: size
        };
        unsafe {
            self.core.device.cmd_copy_buffer(command_buffer[0], src_buffer, dst_buffer, &[copy_region]);
            self.core.device.end_command_buffer(command_buffer[0]).expect("Failed to end copy command buffer!")
        };
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            command_buffer_count: 1,
            p_command_buffers: command_buffer.as_ptr(),
            ..Default::default()
        };
        unsafe {
            self.core.device.queue_submit(self.core.graphics_queue, &[submit_info], vk::Fence::null()).expect("Failed to submit copy buffer!");
            self.core.device.queue_wait_idle(self.core.graphics_queue).expect("Failed to wait graphics queue!");
            self.core.device.free_command_buffers(self.state.read().unwrap().command_pool, &command_buffer)
        };
    }
}
