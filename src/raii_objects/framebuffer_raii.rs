use ash::{Device, vk};
use std::sync::Arc;
pub struct VulkanFramebuffer {
    framebuffer: vk::Framebuffer,

    device: Arc<Device>
}
impl VulkanFramebuffer {
    pub fn new(device: Arc<Device>, render_pass: vk::RenderPass, width: u32, height: u32, image_view: vk::ImageView) -> Result<Self, String> {
        let framebuffer_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            render_pass: render_pass,
            attachment_count: 1,
            p_attachments: &image_view,
            width: width,
            height: height,
            layers: 1,
            ..Default::default()
        };
        let framebuffer = unsafe {
            device.create_framebuffer(&framebuffer_info, None).expect("Failed to create framebuffer!")
        };
        Ok(Self{framebuffer, device})
    }
    pub fn framebuffer(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}
impl Drop for VulkanFramebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_framebuffer(self.framebuffer, None)
        }
    }
}
