use ash::{vk};
use std::sync::Arc;

use crate::engine::ripple_view_context::RippleViewCore;
pub struct VulkanFramebuffer {
    core: Arc<RippleViewCore>,
    framebuffer: vk::Framebuffer,
}
impl VulkanFramebuffer {
    pub fn new(core: Arc<RippleViewCore>, render_pass: vk::RenderPass, width: u32, height: u32, image_view: vk::ImageView) -> Result<Self, String> {
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
            core.device.create_framebuffer(&framebuffer_info, None).expect("Failed to create framebuffer!")
        };
        Ok(Self{framebuffer, core})
    }
    pub fn framebuffer(&self) -> vk::Framebuffer {
        self.framebuffer
    }
    pub fn free(&mut self) {
        unsafe {
            self.core.device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}
