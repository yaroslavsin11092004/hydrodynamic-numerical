use crate::raii_objects::{command_pool_raii::VulkanCommandPool, descripto_pool_raii::VulkanDescriptorPool, instance_raii::VulkanInstance};
use crate::raii_objects::{render_pass_raii::VulkanRenderPass};
use ash::{vk,  Device};
use std::{sync::Arc, ffi::{CStr}, cmp};
use glfw::{Glfw};

pub struct RippleViewContext {
    pub(crate) screen_width: u32,
    pub(crate) screen_height: u32,
    pub(crate) font_size: u32,

    pub(crate) vk_functions: Arc<Option<Device>>,
    pub(crate) command_pool: Option<VulkanCommandPool>,
    pub(crate) descriptor_pool: Option<VulkanDescriptorPool>,
    pub(crate) logical_device: vk::Device,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) app_instance: Option<VulkanInstance>,
    pub(crate) screen_render_pass: Option<VulkanRenderPass>,
    pub(crate) graphics_queue: vk::Queue
}

impl RippleViewContext {
    pub fn new(screen_width: u32, screen_height: u32, font_size: u32) -> Result<Self, String> {
        Ok(Self {
            screen_width,
            screen_height,
            font_size,
            vk_functions: Arc::new(None),
            command_pool: None,
            logical_device: vk::Device::null(),
            physical_device: vk::PhysicalDevice::null(),
            app_instance: None,
            screen_render_pass: None,
            graphics_queue: vk::Queue::null(),
            descriptor_pool: None
        })
    }
    fn eval_device_suitability(&self, device: vk::PhysicalDevice) -> u32 {
        let mut eval : u32 = 0;
        let device_prop = unsafe {
            self.app_instance.as_ref().expect("App Instance not initialized!").instance().get_physical_device_properties(device)
        };
        match device_prop.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => eval += 1000,
            vk::PhysicalDeviceType::INTEGRATED_GPU => eval += 100,
            vk::PhysicalDeviceType::VIRTUAL_GPU => eval += 50,
            _ => eval += 0
        };
        let device_name = unsafe {
            CStr::from_ptr(device_prop.device_name.as_ptr()).to_string_lossy().into_owned()
        };
        if device_name.contains("NVIDIA") || device_name.contains("RTX") || device_name.contains("GTX") || device_name.contains("GeForce") {
            eval += 500
        }
        else if device_name.contains("AMD") || device_name.contains("Radeon") {
            eval += 400
        } 
        else if device_name.contains("Intel") || device_name.contains("Iris") || device_name.contains("HD Graphics") || device_name.contains("UHD Graphics") {
            eval += 300
        };
        cmp::max(0, eval)
    }
    fn is_device_suitable(&self, device: vk::PhysicalDevice) -> bool {
    }
}
