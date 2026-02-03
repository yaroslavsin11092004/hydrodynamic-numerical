use ash::{vk, Device, Instance};
use std::sync::Arc;
pub struct VulkanCommandPool {
    command_pool: vk::CommandPool,
    device: Arc<Device>
}
impl VulkanCommandPool {
    pub fn new(device: Arc<Device>,instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<Self, String> {
        let mut index : u32 = 0;
        let queue_families = unsafe {
            instance.get_physical_device_queue_family_properties(physical_device)
        };
        for i in 0..queue_families.len() {
            if queue_families[i].queue_flags == vk::QueueFlags::GRAPHICS {
                index = i as u32;
                break
            };
        };
        let pool_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            queue_family_index: index,
            ..Default::default()
        };
        let command_pool = unsafe {
            device.create_command_pool(&pool_info, None).expect("Failed to create command pool!")
        };
        Ok(Self { command_pool, device })
    }
    pub fn command_pool(&self) -> vk::CommandPool {
        self.command_pool
    }
}
impl Drop for VulkanCommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None)
        }
    }
}
