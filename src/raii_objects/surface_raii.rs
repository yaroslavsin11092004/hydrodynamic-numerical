use ash::{vk, Entry, Instance};
use glfw::{PWindow};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct VulkanSurface {
    surface : vk::SurfaceKHR,
    loader : ash::khr::surface::Instance
}
impl VulkanSurface {
    pub fn new(window: &PWindow, instance: &Instance, entry : &Entry) -> Result<Self, String> {
        let surface = unsafe {
            ash_window::create_surface(entry, instance, window.display_handle().unwrap().as_raw(), window.window_handle().unwrap().as_raw(), None)
                .expect("Failed to create surface!")
        };
        let loader = ash::khr::surface::Instance::new(entry, instance);
        Ok(Self { surface, loader })
    }
    pub fn surface(&self) -> &vk::SurfaceKHR {
        &self.surface
    }
    pub fn loader(&self) -> &ash::khr::surface::Instance {
        &self.loader
    }
}
impl Drop for VulkanSurface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.surface, None);
        }
    }
}
