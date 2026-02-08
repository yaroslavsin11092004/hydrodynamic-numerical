use crate::{engine::ripple_view_context, raii_objects::{swapchain_raii, framebuffer_raii}};
use crate::ui::{text_render};
use std::{sync::{Arc, RwLock}};
use glam;
use ash::vk;
use glfw::{Glfw, PWindow, fail_on_errors, GlfwReceiver, WindowEvent};
use log::warn;
pub struct RippleView {
    glfw: Glfw,
    window: PWindow,
    events: GlfwReceiver<(f64, WindowEvent)>,
    current_frame: usize,
    framebuffer_resize: bool,
    pub(crate) context: ripple_view_context::RippleViewContext,
    swapchain: swapchain_raii::VulkanSwapchain,
    swapchain_framebuffers: Vec<framebuffer_raii::VulkanFramebuffer>,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphore: Vec<vk::Semaphore>,
    render_finish_semaphore: Vec<vk::Semaphore>,
    in_flight_fence: Vec<vk::Fence>,
    images_in_flight: Vec<vk::Fence>,

    ripple_text: Arc<RwLock<text_render::RippleText>>
}

impl RippleView {
    pub fn new(screen_width: u32, screen_height: u32, font_size: u32) -> Result<Self, String> {
        let mut glfw = glfw::init(fail_on_errors!()).unwrap();
        glfw.window_hint(glfw::WindowHint::Resizable(false));
        glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
        let (mut window, events) = glfw.create_window(screen_width, screen_height, "RippleView Engine", glfw::WindowMode::Windowed).expect("Failed to create window!");
        window.set_key_polling(true);
        window.set_framebuffer_size_polling(false);
        let context = ripple_view_context::RippleViewContext::new(&glfw, &window, screen_width, screen_height, font_size);
        let swapchain = swapchain_raii::VulkanSwapchain::new(context.core.clone(), &window).unwrap();
        context.state.write().unwrap().create_render_pass(&context.core.device, swapchain.swapchain_format());
        let mut swapchain_framebuffers: Vec<framebuffer_raii::VulkanFramebuffer> = Vec::new();
        for i in 0..swapchain.swapchain_image_views().len() {
            swapchain_framebuffers.push(framebuffer_raii::VulkanFramebuffer::new(context.core.clone(), context.state.read().unwrap().screen_render_pass, 
                swapchain.swapchain_extent().width, swapchain.swapchain_extent().height, swapchain.swapchain_image_views()[i]).unwrap());
        };
        context.state.write().unwrap().screen_width = swapchain.swapchain_extent().width;
        context.state.write().unwrap().screen_height = swapchain.swapchain_extent().height;
        context.state.write().unwrap().create_command_pool(&context.core.device, &context.core.instance, context.core.physical_device, context.core.surface.loader(), context.core.surface.surface());
        let command_buffer_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: swapchain_framebuffers.len() as u32,
            command_pool: context.state.read().unwrap().command_pool,
            ..Default::default()
        };
        ////////////////СВИНЬЯ В ЗАГОНЕ НЕ ТРОГАТЬ!!!!/////////////////////////////////////////////
        //                                                                                      //
        let ripple_text = text_render::RippleText::new(context.core.clone(), context.state.clone());
        ripple_text.write().unwrap().create_projection();
        ripple_text.write().unwrap().create_atlas("/home/yaroslavsinyakov/source/rust/Progonka/src/assets/fonts/Roboto/static/Roboto-Italic.ttf", font_size, 512,512);
        ripple_text.write().unwrap().create_atlas("/home/yaroslavsinyakov/source/rust/Progonka/src/assets/fonts/Roboto/static/Roboto-Bold.ttf", 32, 512,512);
        ripple_text.write().unwrap().create_atlas("/home/yaroslavsinyakov/source/rust/Progonka/src/assets/fonts/Roboto/static/Roboto-Thin.ttf", 28, 512, 512);
        ripple_text.write().unwrap().create_descriptors();
        ripple_text.write().unwrap().create_pipeline("/home/yaroslavsinyakov/source/rust/Progonka/src/shaders/text_shader.vert.spv", 
            "/home/yaroslavsinyakov/source/rust/Progonka/src/shaders/text_shader.frag.spv");
        let label = text_render::RenderTextInfo {
            text: String::from("Hello world!"),
            atlas_id: 0,
            x: 10.0,
            y: 10.0,
            scale: 1.0,
            color: glam::Vec4::new(1.0, 1.0, 0.0, 1.0)
        };
        let label1 = text_render::RenderTextInfo {
            text: String::from("Алгоритм прогонки"),
            atlas_id: 1,
            x: 10.0,
            y: 60.0,
            scale: 1.0,
            color: glam::Vec4::new(1.0, 0.2, 0.0, 1.0)
        };
        let label2 = text_render::RenderTextInfo {
            text: String::from("This is Win!"),
            atlas_id: 2,
            x: 10.0,
            y: 500.0,
            scale: 1.0,
            color: glam::Vec4::new(0.5, 1.0, 0.3, 1.0)
        };
        ripple_text.write().unwrap().render_text(&mut [label, label1, label2]);
        //                                                                                    //
        ///////////////////////////////////////////////////////////////////////////////////////

        let command_buffers = unsafe {
            context.core.device.allocate_command_buffers(&command_buffer_info).expect("Failed to allocate command buffer for swapchain!")
        };
        let mut image_available_semaphore: Vec<vk::Semaphore> = vec![vk::Semaphore::null(); 2];
        let mut render_finish_semaphore: Vec<vk::Semaphore> = vec![vk::Semaphore::null();2];
        let mut in_flight_fence: Vec<vk::Fence> = vec![vk::Fence::null(); 2];
        let images_in_flight: Vec<vk::Fence> = vec![vk::Fence::null(); swapchain.swapchain_images().len()];
        let semaphore_info = vk::SemaphoreCreateInfo{
            s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
            ..Default::default()
        };
        let fence_info = vk::FenceCreateInfo{
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };
        for i in 0..2 {
            image_available_semaphore[i] = unsafe {
                context.core.device.create_semaphore(&semaphore_info, None).expect("Failed to create swapchain semaphore!")
            };
            render_finish_semaphore[i] = unsafe {
                context.core.device.create_semaphore(&semaphore_info, None).expect("Failed to create render finish semaphore!")
            };
            in_flight_fence[i] = unsafe {
                context.core.device.create_fence(&fence_info, None).expect("Failed to create in flight fence!")
            };
        };
        Ok(Self { context, window,events, glfw, swapchain, swapchain_framebuffers, command_buffers, image_available_semaphore, render_finish_semaphore, in_flight_fence, images_in_flight, current_frame: 0, framebuffer_resize: false, ripple_text })
    }
    fn update_image_command_buffer(&self, index: usize,  secondary_buffers: &[vk::CommandBuffer]) {
        let clear_colors = &[
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                }
            }
        ];
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe {
            self.context.core.device.reset_command_buffer(self.command_buffers[index], vk::CommandBufferResetFlags::empty()).expect("Failed to reset command buffer!");
            self.context.core.device.begin_command_buffer(self.command_buffers[index], &begin_info).expect("Failed to begin command buffer!")
        };
        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            render_pass: self.context.state.read().unwrap().screen_render_pass,
            framebuffer: self.swapchain_framebuffers[index].framebuffer(),
            render_area: vk::Rect2D {
                offset: vk::Offset2D {
                    x: 0,
                    y: 0
                },
                extent: self.swapchain.swapchain_extent()
            },
            clear_value_count: 1,
            p_clear_values: clear_colors.as_ptr(),
            ..Default::default()
        };
        unsafe {
            self.context.core.device.cmd_begin_render_pass(self.command_buffers[index], &render_pass_begin_info, vk::SubpassContents::SECONDARY_COMMAND_BUFFERS);
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.swapchain_extent().width as f32,
                height: self.swapchain.swapchain_extent().height as f32,
                ..Default::default()
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D {
                    x: 0,
                    y: 0 
                },
                extent: self.swapchain.swapchain_extent()
            };
            self.context.core.device.cmd_set_viewport(self.command_buffers[index], 0, &[viewport]);
            self.context.core.device.cmd_set_scissor(self.command_buffers[index], 0, &[scissor]);
            self.context.core.device.cmd_execute_commands(self.command_buffers[index], secondary_buffers);
            self.context.core.device.cmd_end_render_pass(self.command_buffers[index]);
            self.context.core.device.end_command_buffer(self.command_buffers[index]).expect("Failed to end command_buffers")
        };
    }
    fn cleanup_link_swapchain_resuoures(&mut self) {
        self.swapchain_framebuffers.clear();
    }
    fn recreate_link_framebuffer_resourses(&mut self) {
         unsafe {
        self.context.core.device.device_wait_idle()
            .expect("Failed to wait for device idle before recreation!");
        }
        self.cleanup_link_swapchain_resuoures();
        self.swapchain.recreate(&mut self.glfw, &mut self.window).expect("Failed to recreate swapchain!");
        for i in 0..self.swapchain.swapchain_images().len() {
            self.swapchain_framebuffers.push(framebuffer_raii::VulkanFramebuffer::new(self.context.core.clone(), self.context.state.read().unwrap().screen_render_pass, self.swapchain.swapchain_extent().width, self.swapchain.swapchain_extent().height, self.swapchain.swapchain_image_views()[i]).expect("Failed to create framebuffer!"));
        };
        self.context.state.write().unwrap().screen_width = self.swapchain.swapchain_extent().width;
        self.context.state.write().unwrap().screen_height = self.swapchain.swapchain_extent().height;
                let label = text_render::RenderTextInfo {
            text: String::from("Hello world!"),
            atlas_id: 0,
            x: 10.0,
            y: 10.0,
            scale: 1.0,
            color: glam::Vec4::new(1.0, 1.0, 0.0, 1.0)
        };
        let label1 = text_render::RenderTextInfo {
            text: String::from("Алгоритм прогонки"),
            atlas_id: 1,
            x: 10.0,
            y: 60.0,
            scale: 1.0,
            color: glam::Vec4::new(1.0, 0.2, 0.0, 1.0)
        };
        let label2 = text_render::RenderTextInfo {
            text: String::from("This is Win!"),
            atlas_id: 2,
            x: 1000.0,
            y: 300.0,
            scale: 1.0,
            color: glam::Vec4::new(0.5, 1.0, 0.3, 1.0)
        };
        self.ripple_text.write().unwrap().create_projection();
        self.ripple_text.write().unwrap().re_render(&[label, label1, label2]);
    }
    pub fn main_loop(&mut self) {
        while !self.window.should_close() {
            self.glfw.poll_events();
            let event_to_process: Vec<_> = glfw::flush_messages(&self.events).collect();
            for (_,event) in event_to_process {
                match event {
                     glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _) => {
                        self.window.set_should_close(true);
                     },
                    _ => {}
                }
            };
            self.render_frame();
        };
        unsafe {
            self.context.core.device.device_wait_idle().expect("Failed to wait idle!")
        };
    }
    fn render_frame(&mut self) {
        unsafe {
            self.context.core.device.wait_for_fences(&[self.in_flight_fence[self.current_frame]], true, u64::MAX).expect("Failed to wait for fence!")
        };
        let (w,h) = self.window.get_framebuffer_size();
        if w == 0 || h == 0 { return; };
        let acquire_result = unsafe {
            self.swapchain.loader().acquire_next_image(self.swapchain.swapchain(), u64::MAX, self.image_available_semaphore[self.current_frame], vk::Fence::null())
        };
        let image_index = match acquire_result {
            Ok((index, was_suboptimal)) => {
                if was_suboptimal {
                    warn!("Swapchain is suboptimal!");
                }
                index 
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                unsafe { self.context.core.device.device_wait_idle().ok() };
                self.recreate_link_framebuffer_resourses();
                return;
            },
            Err(vk::Result::TIMEOUT) => {
                warn!("Swapchain acquire timeout!");
                return;
            },
            Err(vk::Result::NOT_READY) => {
                warn!("Swapchain not ready!");
                return;
            },
            Err(err) => {
                panic!("Failed to acquire swapchain image: {:?}", err);
            }
        };
        if self.images_in_flight[image_index as usize] != vk::Fence::null() {
            unsafe { self.context.core.device.wait_for_fences(&[self.images_in_flight[image_index as usize]], true, u64::MAX).unwrap() }
        };
        self.update_image_command_buffer(image_index as usize, &[self.ripple_text.read().unwrap().get_secondary_buffer()]);
        self.images_in_flight[image_index as usize] = self.in_flight_fence[self.current_frame];
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: 1,
            p_wait_semaphores: &self.image_available_semaphore[self.current_frame],
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            command_buffer_count: 1,
            p_command_buffers: &self.command_buffers[image_index as usize],
            signal_semaphore_count: 1,
            p_signal_semaphores: &self.render_finish_semaphore[self.current_frame],
            ..Default::default()
        };
        unsafe {
            self.context.core.device.reset_fences(&[self.in_flight_fence[self.current_frame]]).unwrap();
            self.context.core.device.queue_submit(self.context.core.graphics_queue, &[submit_info], self.in_flight_fence[self.current_frame]).expect("Failed to submit draw command buffer!")
        };
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_swapchains: &self.swapchain.swapchain(),
            swapchain_count: 1,
            p_image_indices: &image_index,
            ..Default::default()
        };
        match unsafe {
            self.swapchain.loader().queue_present(self.context.core.present_queue, &present_info)
        } {
            Ok(suboptimal) => {
                if suboptimal {
                    warn!("Present result is suboptimal!");
                    self.recreate_link_framebuffer_resourses();
                    return;
                }
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_link_framebuffer_resourses();
                return;
            },
            Err(err) => {
                log::error!("Failed to present swapchain image: {:?}", err);
                if err == vk::Result::ERROR_SURFACE_LOST_KHR || 
                    err == vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT {
                        self.recreate_link_framebuffer_resourses();
                }
            }
        };
        self.current_frame = (self.current_frame + 1) % 2;
    }
}

impl Drop for RippleView {
    fn drop(&mut self) {
        unsafe {
            self.context.core.device.device_wait_idle().ok();
            self.ripple_text.read().unwrap().free();
            self.context.core.device.free_command_buffers(self.context.state.read().unwrap().command_pool, &self.command_buffers);
            for i in 0..self.render_finish_semaphore.len() {
                self.context.core.device.destroy_semaphore(self.render_finish_semaphore[i], None)
            };
            for i in 0..self.image_available_semaphore.len() {
                self.context.core.device.destroy_semaphore(self.image_available_semaphore[i], None)
            };
            for i in 0..self.in_flight_fence.len() {
                self.context.core.device.destroy_fence(self.in_flight_fence[i], None)
            };
            self.swapchain_framebuffers.clear();
            self.swapchain_framebuffers.iter_mut().for_each(|f| {
                f.free();
            });
            self.swapchain.cleanup();
            self.context.core.surface.free();
            self.context.free();
        };
    }
}
