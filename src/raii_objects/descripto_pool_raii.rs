use ash::{vk,Device};
use std::sync::Arc;
pub struct VulkanDescriptorPool {
    descriptor_pool: vk::DescriptorPool,
    device: Arc<Option<Device>>
}
impl VulkanDescriptorPool {
    pub fn new(device: Arc<Option<Device>>, max_sets: u32, size: vk::DescriptorPoolSize) -> Result<Self, String> {
        let pool_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: 1,
            p_pool_sizes: &size, 
            max_sets: max_sets,
            ..Default::default()
        };
        let descriptor_pool = unsafe {
            device.as_ref().as_ref().unwrap().create_descriptor_pool(&pool_info, None).expect("Failed to create descriptor pool!")
        };
        Ok(Self {descriptor_pool, device })
    }
    pub fn descriptor_pool(&self) -> vk::DescriptorPool {
        self.descriptor_pool
    }
}
impl Drop for VulkanDescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device.as_ref().as_ref().unwrap().destroy_descriptor_pool(self.descriptor_pool, None)
        }
    }
}
