use ash::vk::{self, IndirectCommandsStreamNV};
use std::{cmp, ptr, sync::Arc};
use crate::engine::{ripple_view_context::RippleViewCore, hardware_functions};
use glfw::{PWindow, Glfw};

pub struct VulkanSwapchain {
    core: Arc<RippleViewCore>,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_format : vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>
}

impl VulkanSwapchain {
    pub fn new(core: Arc<RippleViewCore>, window: &PWindow) -> Result<Self, String> {
        let capabilities = unsafe {
            core.surface.loader().get_physical_device_surface_capabilities(core.physical_device, core.surface.surface()).unwrap()
        };
        let formats : Vec<vk::SurfaceFormatKHR> = unsafe {
            core.surface.loader().get_physical_device_surface_formats(core.physical_device, core.surface.surface()).unwrap()
        };
        let present_modes = unsafe {
            core.surface.loader().get_physical_device_surface_present_modes(core.physical_device, core.surface.surface()).unwrap()
        };
        let mut swapchain_format = formats[0];
        for f in &formats {
            if f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
                swapchain_format = *f;
                break;
            }
        };
        let mut present_mode = vk::PresentModeKHR::FIFO;
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
            let (width, height) = window.get_framebuffer_size();
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
        let queue_family_indeces = hardware_functions::find_queue_families(&core.instance, core.physical_device, core.surface.loader(), core.surface.surface());
        let mut indeces: Vec<u32> = Vec::new();
        let mut sharing_mode = vk::SharingMode::default();
        if queue_family_indeces.graphics_family.unwrap() != queue_family_indeces.present_family.unwrap() {
            sharing_mode = vk::SharingMode::CONCURRENT;
            indeces.push(queue_family_indeces.graphics_family.unwrap());
            indeces.push(queue_family_indeces.present_family.unwrap())
        } else {
            sharing_mode = vk::SharingMode::EXCLUSIVE;
        }
        let create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            surface: core.surface.surface(),
            min_image_count: image_count,
            image_format: swapchain_format.format,
            image_color_space: swapchain_format.color_space,
            image_extent: swapchain_extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: sharing_mode,
            queue_family_index_count: indeces.len() as u32,
            p_queue_family_indices: indeces.as_ptr(),
            pre_transform: capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: present_mode,
            clipped: vk::TRUE,
            ..Default::default()
        };
        let swapchain_loader = ash::khr::swapchain::Device::new(&core.instance, &core.device);
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
                core.device.create_image_view(&image_view_info, None).expect("Failed to create image view!")
            };
        };
        Ok(Self { swapchain, swapchain_format: swapchain_format.format, swapchain_extent, swapchain_images,swapchain_image_views, swapchain_loader, core })
    }
    pub fn cleanup(&mut self) {
        unsafe {
            for &image_view in &self.swapchain_image_views {
                self.core.device.destroy_image_view(image_view, None)
            };
            self.swapchain_image_views.clear();
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
    pub fn recreate(&mut self, glfw: &mut Glfw,  window: &mut PWindow) -> Result<(), vk::Result> {
        let (mut w,mut h) = window.get_framebuffer_size();
        while w == 0 || h == 0 {
            (w,h) = window.get_framebuffer_size();
            println!("{}-{}", w, h);
            glfw.poll_events()
        };
        unsafe {
            self.core.device.device_wait_idle()?;
            for &image_view in &self.swapchain_image_views {
                self.core.device.destroy_image_view(image_view, None)
            };
            self.swapchain_image_views.clear();
        };
        let capabilities = unsafe {
            self.core.surface.loader().get_physical_device_surface_capabilities(self.core.physical_device, self.core.surface.surface()).unwrap()
        };
        let formats = unsafe {
            self.core.surface.loader().get_physical_device_surface_formats(self.core.physical_device, self.core.surface.surface()).unwrap()
        };
        let present_modes = unsafe {
            self.core.surface.loader().get_physical_device_surface_present_modes(self.core.physical_device, self.core.surface.surface()).unwrap()
        };
        let mut swapchain_format = formats[0];
        for f in &formats {
            if f.format == vk::Format::B8G8R8A8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
                swapchain_format = *f;
                break;
            }
        };
        let mut present_mode = vk::PresentModeKHR::FIFO;
        for &m in &present_modes {
            if m == vk::PresentModeKHR::MAILBOX {
                present_mode = m;
                break;
            }
        };
        let mut swapchain_extent = vk::Extent2D::default();
        if capabilities.current_extent.width != u32::MAX {
            swapchain_extent = capabilities.current_extent 
        } else {
            let (width, height) = window.get_framebuffer_size();
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
        let queue_family_indeces = hardware_functions::find_queue_families(&self.core.instance, self.core.physical_device, self.core.surface.loader(), self.core.surface.surface());
        let mut indeces: Vec<u32> = Vec::new();
        let mut sharing_mode = vk::SharingMode::default();
        if queue_family_indeces.graphics_family.unwrap() != queue_family_indeces.present_family.unwrap() {
            sharing_mode = vk::SharingMode::CONCURRENT;
            indeces.push(queue_family_indeces.graphics_family.unwrap());
            indeces.push(queue_family_indeces.present_family.unwrap())
        } else {
            sharing_mode = vk::SharingMode::EXCLUSIVE;
        }
        let create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            old_swapchain: self.swapchain,
            surface: self.core.surface.surface(),
            min_image_count: image_count,
            image_format: swapchain_format.format,
            image_color_space: swapchain_format.color_space,
            image_extent: swapchain_extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: sharing_mode,
            queue_family_index_count: indeces.len() as u32,
            p_queue_family_indices: indeces.as_ptr(),
            pre_transform: capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            clipped: vk::TRUE,
            present_mode: present_mode,
            ..Default::default()
        };
        let new_swapchain = unsafe {
            self.swapchain_loader.create_swapchain(&create_info, None).expect("Failed to create swapchain!")
        };
        unsafe {
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        };
        self.swapchain = new_swapchain;
        self.swapchain_images = unsafe {
            self.swapchain_loader.get_swapchain_images(self.swapchain).expect("Failed to get swapchain images!")
        };
        self.swapchain_extent = swapchain_extent;
        self.swapchain_format = swapchain_format.format;
        self.swapchain_image_views = vec![vk::ImageView::null(); self.swapchain_images.len()];
        for i in 0..self.swapchain_image_views.len() {
            let image_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                image: self.swapchain_images[i],
                view_type: vk::ImageViewType::TYPE_2D,
                format: self.swapchain_format,
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    ..Default::default()
                },
                subresource_range: vk::ImageSubresourceRange{
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    base_array_layer: 0,
                    level_count: 1,
                    layer_count: 1
                },
                ..Default::default()
            };
            self.swapchain_image_views[i] = unsafe {
                self.core.device.create_image_view(&image_view_info, None).expect("Failed to create swapchain image views!")
            };
        };
        Ok(())
    }
    pub fn loader(&self) -> &ash::khr::swapchain::Device {
        &self.swapchain_loader
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
