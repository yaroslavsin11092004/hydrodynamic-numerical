use ash::{vk,  Device, Entry, Instance};
use std::ffi::CString;
use std::{ffi::{CStr}, cmp, collections::hash_set};
use glfw::{Glfw};
#[derive(Default, Debug)]
pub struct QueueFamilyIndeces {
    pub(crate) graphics_family: Option<u32>,
    pub(crate) present_family: Option<u32>
}

impl QueueFamilyIndeces {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}
#[derive(Default)]
pub struct SwapchainSupportDetails {
    pub(crate) capabilities: vk::SurfaceCapabilitiesKHR,
    pub(crate) formats: Vec<vk::SurfaceFormatKHR>,
    pub(crate) present_modes: Vec<vk::PresentModeKHR>
}
pub fn find_queue_families(instance: &ash::Instance, device: vk::PhysicalDevice, surface_instance: &ash::khr::surface::Instance, surface: vk::SurfaceKHR)->QueueFamilyIndeces {
    let queue_families = unsafe {
        instance.get_physical_device_queue_family_properties(device)
    };
    let mut family_indeces = QueueFamilyIndeces::default();
    for i in 0..queue_families.len(){
        if queue_families[i].queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            family_indeces.graphics_family = Some(i as u32)
        };
        let present_support = unsafe {
            surface_instance.get_physical_device_surface_support(device, i as u32, surface).unwrap()
        };
        if present_support {
            family_indeces.present_family = Some(i as u32)
        };
        if family_indeces.is_complete() {
            break 
        };
    };
    family_indeces
}
fn eval_device_suitability(instance: &ash::Instance, device: vk::PhysicalDevice) -> u32 {
    let mut eval : u32 = 0;
    let device_prop = unsafe {
        instance.get_physical_device_properties(device)
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

fn check_device_extensions_support(instance: &ash::Instance, device: vk::PhysicalDevice) -> bool {
    let available_extensions = unsafe { instance.enumerate_device_extension_properties(device).unwrap() };
    let mut required_extensions = hash_set::HashSet::new();
    let name = unsafe { CStr::from_ptr(ash::khr::swapchain::NAME.as_ptr()).to_string_lossy().into_owned() };
    required_extensions.insert(name);
    for &ext in &available_extensions {
        let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()).to_string_lossy().into_owned() };
        required_extensions.remove(&name);
    };
    required_extensions.is_empty()
}
fn query_swapchain_support(device: vk::PhysicalDevice, instance_surface: &ash::khr::surface::Instance, surface: vk::SurfaceKHR) ->SwapchainSupportDetails {
    let mut details = SwapchainSupportDetails::default();
    details.capabilities = unsafe {
        instance_surface.get_physical_device_surface_capabilities(device, surface).unwrap()
    };
    details.formats = unsafe {
        instance_surface.get_physical_device_surface_formats(device,surface).unwrap()
    };
    details.present_modes = unsafe {
        instance_surface.get_physical_device_surface_present_modes(device, surface).unwrap()
    };
    details
}
fn is_device_suitability(instance: &ash::Instance, device: vk::PhysicalDevice, instance_surface: &ash::khr::surface::Instance, surface: vk::SurfaceKHR) -> bool {
    if device == vk::PhysicalDevice::null() {
        return false
    };
    let indeces = find_queue_families(instance, device, instance_surface, surface);
    let extensions_supported = check_device_extensions_support(instance, device);
    let mut swapchain_suitability = false;
    if extensions_supported {
        let details = query_swapchain_support(device, instance_surface, surface);
        swapchain_suitability = !details.present_modes.is_empty() && !details.formats.is_empty()
    };
    indeces.is_complete() && extensions_supported && swapchain_suitability
}
pub fn pick_physical_devices(instance: &ash::Instance, instance_surface: &ash::khr::surface::Instance, surface: vk::SurfaceKHR) -> vk::PhysicalDevice {
    let devices = unsafe {
        instance.enumerate_physical_devices().unwrap()
    };
    let mut cur_eval: u32 = 0;
    let mut physical_device = vk::PhysicalDevice::null();
    devices.iter().for_each(|device| {
        if is_device_suitability(instance, *device, &instance_surface, surface) {
            let eval = eval_device_suitability(instance, *device);
            if eval > cur_eval {
                cur_eval = eval;
                physical_device = *device;
            };
        };
    });
    physical_device
}
pub fn create_logical_device(instance: &ash::Instance, instance_surface: &ash::khr::surface::Instance, surface: vk::SurfaceKHR, physical_device: vk::PhysicalDevice) -> (ash::Device, vk::Queue, vk::Queue) {
    let family_indeces = find_queue_families(instance, physical_device, instance_surface, surface);
    let mut queue_infos : Vec<vk::DeviceQueueCreateInfo> = Vec::new();
    let mut unique_queue_families = hash_set::HashSet::new();
    unique_queue_families.insert(family_indeces.graphics_family.unwrap());
    unique_queue_families.insert(family_indeces.present_family.unwrap());
    let queue_prior : f32 = 1.0;
    for &queue_family in &unique_queue_families {
        let queue_info = vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            queue_family_index: queue_family,
            queue_count: 1,
            p_queue_priorities: &queue_prior,
            ..Default::default()
        };
        queue_infos.push(queue_info);
    };
    let device_features = vk::PhysicalDeviceFeatures{
        ..Default::default()
    };
    let device_extensions = ash::khr::swapchain::NAME;
    let device_info = vk::DeviceCreateInfo {
        s_type: vk::StructureType::DEVICE_CREATE_INFO,
        queue_create_info_count: queue_infos.len() as u32,
        p_queue_create_infos: queue_infos.as_ptr(),
        p_enabled_features: &device_features,
        enabled_extension_count: 1,
        pp_enabled_extension_names: &device_extensions.as_ptr(),
        ..Default::default()
    };
    let vk_functions = unsafe {
        instance.create_device(physical_device, &device_info, None).unwrap()
    };
    let present_queue = unsafe { 
        vk_functions.get_device_queue(family_indeces.present_family.unwrap(), 0)
    };
    let graphics_queue = unsafe {
        vk_functions.get_device_queue(family_indeces.graphics_family.unwrap(), 0)
    };
    (vk_functions, graphics_queue, present_queue)
}

