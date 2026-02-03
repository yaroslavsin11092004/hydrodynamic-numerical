use ash::{vk,Device,Instance};
use std::{cmp, ptr, sync::Arc};

use glfw::{PWindow};

pub struct VulkanSwapchain {
    swapchain: vk::SwapchainKHR,
    swapchain_format : vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    swapchain_loader: ash::khr::swapchain::Device,

    device: Arc<Device>
}

impl VulkanSwapchain {
    pub fn new(device: Arc<Device>, instance : &ash::khr::surface::Instance, dev_instance: &Instance, window: PWindow, surface: vk::SurfaceKHR, physical_device: vk::PhysicalDevice) -> Result<Self, String> {
        let capabilities = unsafe {
            ash::khr::surface::Instance::get_physical_device_surface_capabilities(instance, physical_device, surface).unwrap()
        };
        let formats : Vec<vk::SurfaceFormatKHR> = unsafe {
            ash::khr::surface::Instance::get_physical_device_surface_formats(instance, physical_device, surface).unwrap()
        };
        let present_modes = unsafe {
            ash::khr::surface::Instance::get_physical_device_surface_present_modes(instance, physical_device, surface).unwrap()
        };
        let mut swapchain_format = vk::SurfaceFormatKHR::default();
        for f in &formats {
            if f.format == vk::Format::R8G8B8A8_UNORM && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
                swapchain_format = *f;
                break;
            }
        };
        let mut present_mode = vk::PresentModeKHR::default();
        for &m in &present_modes {
            if m == vk::PresentModeKHR::MAILBOX {
                present_mode = m;
                break;
            }
        };
        let mut swapchain_extent = vk::Extent2D::default();
        if capabilities.current_extent.width != u32::MAX {
            swapchain_extent = capabilities.current_extent;
        } else {
            let mut width = 0;
            let mut height = 0;
            (width, height) = window.get_framebuffer_size();
            let actual_extent = vk::Extent2D {
                width: width as u32,
                height: height as u32 
            };
            swapchain_extent.width = cmp::max(capabilities.min_image_extent.width, cmp::min(capabilities.max_image_extent.width, actual_extent.width));
            swapchain_extent.height = cmp::max(capabilities.min_image_extent.height, cmp::min(capabilities.max_image_extent.height, actual_extent.height));
        };
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && capabilities.max_image_count < image_count {
            image_count = capabilities.max_image_count
        };
        let create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            surface: surface,
            min_image_count: image_count,
            image_format: swapchain_format.format,
            image_color_space: swapchain_format.color_space,
            image_extent: swapchain_extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            pre_transform: capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: present_mode,
            clipped: vk::TRUE,
            ..Default::default()
        };
        let swapchain_loader = ash::khr::swapchain::Device::new(dev_instance, &device);
        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&create_info, None).expect("Failed to create swapchain!")
        };
        let swapchain_images = unsafe {
            swapchain_loader.get_swapchain_images(swapchain).expect("Failed to get swapchain images!")
        };

        let mut swapchain_image_views: Vec<vk::ImageView> = vec![vk::ImageView::null(); swapchain_images.len()];
        let component_mapping = vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            ..Default::default()
        };
        let subresources = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_array_layer: 0,
            base_mip_level: 0,
            layer_count: 1,
            level_count: 1,
        };
        for i in 0..swapchain_images.len() {
            let image_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                image: swapchain_images[i],
                view_type: vk::ImageViewType::TYPE_2D,
                format: swapchain_format.format,
                components: component_mapping,
                subresource_range: subresources,
                ..Default::default()
            };
            swapchain_image_views[i] = unsafe {
                device.create_image_view(&image_view_info, None).expect("Failed to create image view!")
            };
        };
        Ok(Self { swapchain, swapchain_format: swapchain_format.format, swapchain_extent, swapchain_images,swapchain_image_views, swapchain_loader, device })
    }
    pub fn cleanup(&mut self) {
        unsafe {
            for &image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(image_view, None)
            };
            self.swapchain_image_views.clear();
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
    pub fn recreate(&mut self, device: Arc<Device>, instance: &ash::khr::surface::Instance, dev_instance: &Instance, window: PWindow, surface: vk::SurfaceKHR, physical_device: vk::PhysicalDevice) -> Result<(), vk::Result> {
        unsafe {
            device.device_wait_idle()?
        };
        self.cleanup();
        *self = VulkanSwapchain::new(device, instance, dev_instance, window, surface, physical_device).expect("Failed to recreate swapchain!");
        Ok(())
    }
    pub fn swapchain(&self) -> vk::SwapchainKHR {
        self.swapchain
    }
    pub fn swapchain_images(&self) -> &Vec<vk::Image> {
        &self.swapchain_images
    }
    pub fn swapchain_image_views(&self) -> &Vec<vk::ImageView> {
        &self.swapchain_image_views
    }
    pub fn swapchain_format(&self) -> vk::Format {
        self.swapchain_format
    }
    pub fn swapchain_extent(&self) -> vk::Extent2D {
        self.swapchain_extent
    }
}
impl Drop for VulkanSwapchain {
    fn drop(&mut self) {
        self.cleanup();
    }
}
