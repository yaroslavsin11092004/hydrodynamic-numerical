use ash::{vk, Entry, Instance};
use std::ffi::{CString};
use glfw::{Glfw};

pub struct VulkanInstance {
    entry : Entry,
    instance : Instance
}

impl VulkanInstance {
    pub fn new(glfw : &Glfw,  app_name : &str, engine_name : &str) -> Result<Self, String> {
        let entry = unsafe { Entry::load().map_err(|e| format!("Failed to load Vulkan: {}", e))? };
        let app_name_cstr = CString::new(app_name).unwrap();
        let engine_name_cstr = CString::new(engine_name).unwrap();
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            api_version: vk::make_api_version(0,1,0,0),
            engine_version: vk::make_api_version(0,1,0,0),
            p_application_name: app_name_cstr.as_ptr(),
            p_engine_name: engine_name_cstr.as_ptr(),
            ..Default::default()
        };
        let extensions = glfw.get_required_instance_extensions().expect("Failed to get glfw Vulkan extensions");
        let extension_names = extensions.iter().map(|ext| ext.as_ptr() as *const i8).collect::<Vec<_>>();
        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_application_info: &app_info,
            enabled_extension_count: extensions.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            enabled_layer_count: 0,
            ..Default::default()
        };
        let instance = unsafe {
            entry.create_instance(&create_info, None)
                .map_err(|e| format!("Failed to create instance: {}", e)).unwrap()
        };
        Ok(Self { entry, instance })
    }
    pub fn entry(&self) -> &Entry {
        &self.entry
    }
    pub fn instance(&self) -> &Instance {
        &self.instance
    }
}
impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
        println!("Vulkan instance destroy!");
    }
}
