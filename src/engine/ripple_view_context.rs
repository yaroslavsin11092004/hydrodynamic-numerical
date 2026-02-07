use ash::{vk,  Device, Entry, Instance};
use std::ffi::CString;
use std::sync::RwLock;
use std::{sync::Arc};
use glfw::{Glfw, PWindow};
use crate::{engine::hardware_functions, raii_objects::surface_raii};

pub struct RippleViewCore {
    pub(crate) instance: ash::Instance,
    pub(crate) device: ash::Device,
    pub(crate) surface: surface_raii::VulkanSurface,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) entry: Entry,
    pub(crate) present_queue: vk::Queue,
    pub(crate) graphics_queue: vk::Queue
}
impl RippleViewCore {
    pub fn new(glfw: &Glfw, window: &PWindow) -> Arc<Self> {
        let entry = unsafe { Entry::load().expect("Failed to initialize vulkan functions!") };
        let app_name = CString::new("RippleView Application").unwrap();
        let engine_name = CString::new("No engine").unwrap();
        let app_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            api_version: vk::make_api_version(0,1,0,0),
            engine_version: vk::make_api_version(0,1,0,0),
            p_application_name: app_name.as_ptr(),
            p_engine_name: engine_name.as_ptr(),
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
            entry.create_instance(&create_info, None).expect("Failed to create application instance!")
        };
        let surface = surface_raii::VulkanSurface::new(window, &instance, &entry).expect("Failed to create surface!");
        let physical_device = hardware_functions::pick_physical_devices(&instance, surface.loader(), surface.surface());
        let logical_device_and_queues = hardware_functions::create_logical_device(&instance, surface.loader(), surface.surface(), physical_device);
        Arc::new(Self { instance, device: logical_device_and_queues.0, physical_device, entry, present_queue: logical_device_and_queues.2, surface, graphics_queue: logical_device_and_queues.1 })
    }
}
impl Drop for RippleViewCore {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None)
        };
    }
}

pub struct RippleViewState {
    pub(crate) screen_width: u32,
    pub(crate) screen_height: u32,
    pub(crate) font_size: u32,
    pub(crate) screen_render_pass: vk::RenderPass,
    pub(crate) command_pool: vk::CommandPool
}

impl RippleViewState {
    pub fn new(screen_width: u32, screen_height: u32, font_size: u32) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self { screen_width, screen_height, font_size, screen_render_pass: vk::RenderPass::null(), command_pool: vk::CommandPool::null() }))
    }
    pub fn create_render_pass(&mut self, device: &ash::Device, sch_format: vk::Format) {
        let color_attachment = vk::AttachmentDescription {
            format: sch_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };
        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        };
        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            ..Default::default()
        };
        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::NONE,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };
        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };
        self.screen_render_pass = unsafe {
            device.create_render_pass(&render_pass_info, None).expect("Failed to create render pass!")
        };
    }
    pub fn create_command_pool(&mut self,device: &ash::Device, instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface_instance: &ash::khr::surface::Instance, surface: vk::SurfaceKHR) {
        let queue_family_index = hardware_functions::find_queue_families(instance, physical_device, surface_instance, surface);
        assert!(queue_family_index.is_complete(), "Failed to find queue family indeces!");
        let pool_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            queue_family_index: queue_family_index.graphics_family.unwrap(),
            ..Default::default()
        };
        self.command_pool = unsafe {
            device.create_command_pool(&pool_info, None).expect("Failed to create command pool!")
        };
    }
}

#[derive(Clone)]
pub struct RippleViewContext {
    pub(crate) core: Arc<RippleViewCore>,
    pub(crate) state: Arc<RwLock<RippleViewState>>
}

impl RippleViewContext {
    pub fn new(glfw: &Glfw, window: &PWindow, screen_width: u32, screen_height: u32, font_size: u32) -> Self {
        let core = RippleViewCore::new(glfw, window);
        let state = RippleViewState::new(screen_width, screen_height, font_size);
        Self { core, state }
    }
}
impl Drop for RippleViewContext {
    fn drop(&mut self) {
        unsafe {
            self.core.device.destroy_command_pool(self.state.read().unwrap().command_pool, None);
            self.core.device.destroy_render_pass(self.state.read().unwrap().screen_render_pass, None)
        };
    }
}
