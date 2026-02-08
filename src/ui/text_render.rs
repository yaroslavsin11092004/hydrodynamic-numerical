use crate::ui::glyph::{AtlasPosition, GlyphData, strip_pack, Glyph};
use freetype as ft;
use log::warn;
use rayon::str::ParallelString;
use crate::engine::{base_element::RippleBase, ripple_view_context};
use std::{collections::HashMap, ffi::CString, sync::{Arc, RwLock}};
use ash::vk;
use glam;

#[derive(Default, Clone)]
pub struct TextAtlas {
    pub atlas_image: vk::Image,
    pub atlas_image_view: vk::ImageView,
    pub atlas_sampler: vk::Sampler,
    pub atlas_memory: vk::DeviceMemory,

    pub atlas_descriptor_set: vk::DescriptorSet 
}

pub struct Vertex {
    position: glam::Vec2,
    tex_coord: glam::Vec2 
}

pub struct RenderTextInfo {
    pub text: String,
    pub x: f32,
    pub y: f32, 
    pub scale: f32,
    pub color: glam::Vec4,
    pub atlas_id: u32
}

#[repr(C)]
pub struct TextPushConstants {
    pub projection: glam::Mat4,
    pub color: glam::Vec4,
    pub position: glam::Vec2,
    pub scale: f32,
    pub _padding: f32,
    pub uv_min: glam::Vec2,
    pub uv_max: glam::Vec2,
    pub glyph_size: glam::Vec2,
}
impl Vertex {
    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        let desc = vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX
        };
        desc
    }
    pub fn get_attribute_description() -> Vec<vk::VertexInputAttributeDescription> {
        let mut attributes = vec![vk::VertexInputAttributeDescription::default();2];
        attributes[0].location = 0;
        attributes[0].binding = 0;
        attributes[0].format = vk::Format::R32G32_SFLOAT;
        attributes[0].offset = std::mem::offset_of!(Vertex, position) as u32;

        attributes[1].location = 1;
        attributes[1].binding = 0;
        attributes[1].format = vk::Format::R32G32_SFLOAT;
        attributes[1].offset = std::mem::offset_of!(Vertex, tex_coord) as u32;
        attributes
    }
}

pub struct RippleText {
    base: Arc<RippleBase>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_layout: vk::DescriptorSetLayout,
    atlases: Vec<TextAtlas>,
    ft_library: ft::Library,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    atlases_glyphs: Vec<HashMap<u32, Glyph>>,
    projection: glam::Mat4,
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    secondary_command_buffer: vk::CommandBuffer
}
impl RippleText {
    pub fn create_projection(&mut self) {
        self.projection = glam::Mat4::from_cols(
            glam::Vec4::new(2.0 / self.base.state.read().unwrap().screen_width as f32, 0.0, 0.0, 0.0),
            glam::Vec4::new(0.0, -2.0 / self.base.state.read().unwrap().screen_height as f32, 0.0, 0.0),
            glam::Vec4::new(0.0, 0.0, 0.0, -1.0),
            glam::Vec4::new(-1.0, 1.0, 0.0, 1.0));
    }
    pub fn new(core: Arc<ripple_view_context::RippleViewCore>, state: Arc<RwLock<ripple_view_context::RippleViewState>>) -> Arc<RwLock<Self>> {
        let base = RippleBase::new(core.clone(), state.clone());
        let ft_library = ft::Library::init().expect("Failed to initialize FreeType!");
        Arc::new(RwLock::new(Self {base, descriptor_pool: vk::DescriptorPool::null(),descriptor_layout: vk::DescriptorSetLayout::null(), atlases: Vec::new(), ft_library, pipeline_layout: vk::PipelineLayout::null(), pipeline: vk::Pipeline::null(), 
            vertex_memory: vk::DeviceMemory::null(), vertex_buffer: vk::Buffer::null(), secondary_command_buffer: vk::CommandBuffer::null(),
            atlases_glyphs: Vec::new(), projection: glam::Mat4::ZERO}))
    }
    pub fn create_atlas(&mut self, font_path: &str, font_size: u32, atlas_width: u32, atlas_height: u32) {
        let face = self.ft_library.new_face(font_path, 0).expect("Failed to load font!");
        let mut hash_glyph: HashMap<u32, Glyph> = HashMap::new();
        face.set_pixel_sizes(0, font_size).expect("Failed to set pixel size!");
        let mut glyph_data: Vec<GlyphData> = Vec::new();
        for codepoint in 32..=126 {
            glyph_data.push(GlyphData::new(codepoint, &face).expect("Failed to load glyph!"));
        };
        for codepoint in 0x0410..=0x042F {
            glyph_data.push(GlyphData::new(codepoint, &face).expect("Failed to load glyph!"));
        };
        for codepoint in 0x0430..=0x044F {
            glyph_data.push(GlyphData::new(codepoint, &face).expect("Failed to load glyph!"));
        };
        let positions = strip_pack(&glyph_data, atlas_width, atlas_height);
        let mut atlas_pixels: Vec<u8> = vec![0; (atlas_width * atlas_height) as usize];
        for i in 0..glyph_data.len() {
            let glyph = &glyph_data[i];
            let pos = &positions[i];
            for y in 0..glyph.height {
                for x in 0..glyph.width {
                    let atlas_x = pos.x + x;
                    let atlas_y = pos.y + y;
                    let atlas_idx = atlas_y * atlas_width + atlas_x;
                    atlas_pixels[atlas_idx as usize] = glyph.get_pixel(x,y);
                }
            };
            let info: Glyph = Glyph {
                uv_min: glam::Vec2::new(pos.x as f32 / atlas_width as f32, (pos.y + glyph.height) as f32 / atlas_height as f32),
                uv_max: glam::Vec2::new((pos.x + glyph.width) as f32 / atlas_width as f32, pos.y as f32 / atlas_height as f32),
                width: glyph.width,
                height: glyph.height,
                bearing: glam::Vec2::new(glyph.bearing.0, -glyph.bearing.1),
                advance: glyph.advance
            };
            hash_glyph.insert(glyph.codepoint, info);
        }
        self.atlases_glyphs.push(hash_glyph);
        let image_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D {
                width: atlas_width,
                height: atlas_height,
                depth: 1 
            },
            mip_levels: 1,
            array_layers: 1,
            format: vk::Format::R8_UNORM,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage: vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        let atlas_image = unsafe {
            self.base.core.device.create_image(&image_info, None).expect("Failed to create atlas texture!")
        };
        let buffer_size = atlas_pixels.len() as vk::DeviceSize;
        let (staging_buffer, staging_memory) = self.base.create_buffer(buffer_size, vk::BufferUsageFlags::TRANSFER_SRC, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
        let data: *mut std::ffi::c_void = unsafe {
            self.base.core.device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty()).expect("Failed to map memory!")
        };
        unsafe {
            if !data.is_null() {
                std::ptr::copy_nonoverlapping(atlas_pixels.as_ptr(), data as *mut u8, buffer_size as usize);
            };
            self.base.core.device.unmap_memory(staging_memory)
        };
        let mem_requirements = unsafe {
            self.base.core.device.get_image_memory_requirements(atlas_image)
        };
        let allocate_info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            allocation_size: mem_requirements.size,
            memory_type_index: self.base.find_memory_type(mem_requirements.memory_type_bits, vk::MemoryPropertyFlags::DEVICE_LOCAL),
            ..Default::default()
        };
        let atlas_memory = unsafe {
            self.base.core.device.allocate_memory(&allocate_info, None).expect("Failed to allocate texture memory!")
        };
        unsafe { self.base.core.device.bind_image_memory(atlas_image, atlas_memory, 0).unwrap() };
        let command_buffer_alloc = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            command_buffer_count: 1,
            command_pool: self.base.state.read().unwrap().command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            ..Default::default()
        };
        let command_buffer = unsafe { self.base.core.device.allocate_command_buffers(&command_buffer_alloc).expect("Failed to allocate command buffer for atlas!")};
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe { self.base.core.device.begin_command_buffer(command_buffer[0], &begin_info).expect("Failed to begin command buffer atlas!") };
        let barrier_to_transfer = vk::ImageMemoryBarrier {
            s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: atlas_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                base_array_layer: 0,
                level_count: 1,
                layer_count: 1,
            },
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            ..Default::default()
        };
        unsafe { 
            self.base.core.device.cmd_pipeline_barrier(command_buffer[0], vk::PipelineStageFlags::TOP_OF_PIPE, 
                vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier_to_transfer])
        }; 
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers{
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                layer_count: 1,
                base_array_layer: 0
            },
            image_offset: vk::Offset3D {
                x: 0,
                y: 0,
                z: 0 
            },
            image_extent: vk::Extent3D {
                width: atlas_width,
                height: atlas_height,
                depth: 1 
            }
        };
        unsafe { self.base.core.device.cmd_copy_buffer_to_image(command_buffer[0], staging_buffer, atlas_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[region]) };
        let barrier_to_shader = vk::ImageMemoryBarrier {
            s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image: atlas_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                base_mip_level: 0,
                layer_count: 1,
                level_count: 1 
            },
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            ..Default::default()
        };
        unsafe {
            self.base.core.device.cmd_pipeline_barrier(command_buffer[0], vk::PipelineStageFlags::TRANSFER, 
                vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[barrier_to_shader]);
            self.base.core.device.end_command_buffer(command_buffer[0]).expect("Failed to finish command buffer for atlas!")
        };
        let fence_info = vk::FenceCreateInfo {
            s_type: vk::StructureType::FENCE_CREATE_INFO,
            ..Default::default()
        };
        let fence = unsafe { self.base.core.device.create_fence(&fence_info, None).expect("Failed to create fence!") };
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            command_buffer_count: 1,
            p_command_buffers: command_buffer.as_ptr(),
            ..Default::default()
        };
        unsafe {
            self.base.core.device.queue_submit(self.base.core.graphics_queue, &[submit_info], fence).expect("Failed to submit create atlas image!");
            self.base.core.device.wait_for_fences(&[fence], true, u64::MAX).expect("Failed to wait for fence!");
            self.base.core.device.reset_fences(&[fence]).expect("Failed to reset fence!");
            self.base.core.device.destroy_buffer(staging_buffer, None);
            self.base.core.device.free_memory(staging_memory, None);
            self.base.core.device.free_command_buffers(self.base.state.read().unwrap().command_pool, &command_buffer);
            self.base.core.device.destroy_fence(fence, None)
        };

        let image_view_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image: atlas_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::R8_UNORM,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                base_mip_level: 0,
                layer_count: 1,
                level_count: 1
            },
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::R,
                b: vk::ComponentSwizzle::R,
                a: vk::ComponentSwizzle::ONE 
            },
            ..Default::default()
        };
        let atlas_image_view = unsafe { self.base.core.device.create_image_view(&image_view_info, None).expect("Failed to create atlas image view!") };
        let sampler = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 1.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            ..Default::default()
        };
        let atlas_sampler = unsafe { self.base.core.device.create_sampler(&sampler, None).expect("Failed to create atlas sampler!") };
        let atlas = TextAtlas {
            atlas_image,
            atlas_memory,
            atlas_image_view,
            atlas_sampler,
            atlas_descriptor_set: vk::DescriptorSet::null()
        };
        self.atlases.push(atlas);
    }
    pub fn create_descriptors(&mut self) {
        let layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            p_immutable_samplers: std::ptr::null(),
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            ..Default::default()
        };
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            binding_count: 1,
            p_bindings: &layout_binding,
            ..Default::default()
        };
        self.descriptor_layout = unsafe {
            self.base.core.device.create_descriptor_set_layout(&layout_info, None).expect("Failed to create descriptor set layout!")
        };
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: self.atlases.len() as u32 
        };
        let pool_info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            pool_size_count: 1,
            p_pool_sizes: &pool_size,
            max_sets: self.atlases.len() as u32,
            ..Default::default()
        };
        self.descriptor_pool = unsafe {
            self.base.core.device.create_descriptor_pool(&pool_info, None).expect("Failed to create descriptor pool!")
        };
        for atlas in &mut self.atlases {
            let descriptor_set_alloc_info = vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                descriptor_pool: self.descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &self.descriptor_layout,
                ..Default::default()
            };
            atlas.atlas_descriptor_set = unsafe {
                self.base.core.device.allocate_descriptor_sets(&descriptor_set_alloc_info).expect("Failed to allocate descriptor set!")[0]
            };
            let image_info = vk::DescriptorImageInfo {
                sampler: atlas.atlas_sampler,
                image_view: atlas.atlas_image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
            };
            let desc_write = vk::WriteDescriptorSet{
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                dst_set: atlas.atlas_descriptor_set,
                dst_binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                dst_array_element: 0,
                descriptor_count: 1,
                p_image_info: &image_info,
                ..Default::default()
            };
            unsafe { self.base.core.device.update_descriptor_sets(&[desc_write], &[]) };
        }
        let vertices = &[
            Vertex { position: glam::Vec2::new(0.0, 0.0), tex_coord: glam::Vec2::new(0.0, 0.0) },
            Vertex { position: glam::Vec2::new(1.0, 0.0), tex_coord: glam::Vec2::new(1.0, 0.0) },
            Vertex { position: glam::Vec2::new(1.0, 1.0), tex_coord: glam::Vec2::new(1.0, 1.0) },
            Vertex { position: glam::Vec2::new(1.0, 1.0), tex_coord: glam::Vec2::new(1.0, 1.0) },
            Vertex { position: glam::Vec2::new(0.0, 1.0), tex_coord: glam::Vec2::new(0.0, 1.0) },
            Vertex { position: glam::Vec2::new(0.0, 0.0), tex_coord: glam::Vec2::new(0.0, 0.0) }
        ];
        let vertex_size = (std::mem::size_of::<Vertex>() * vertices.len()) as vk::DeviceSize;
        let (staging_buffer, staging_memory) = self.base.create_buffer(vertex_size, vk::BufferUsageFlags::TRANSFER_SRC, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
        let data: *mut u8 = unsafe {
            self.base.core.device.map_memory(staging_memory, 0, vertex_size, vk::MemoryMapFlags::empty()).expect("Failed to map memory!") as *mut u8
        };
        unsafe {
            let bytes = std::slice::from_raw_parts(vertices.as_ptr() as *const u8, vertex_size as usize);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, vertex_size as usize);
            self.base.core.device.unmap_memory(staging_memory)
        };
        (self.vertex_buffer, self.vertex_memory) = self.base.create_buffer(vertex_size, vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST, vk::MemoryPropertyFlags::DEVICE_LOCAL);
        self.base.copy_buffer(staging_buffer, self.vertex_buffer, vertex_size);
        unsafe {
            self.base.core.device.destroy_buffer(staging_buffer, None);
            self.base.core.device.free_memory(staging_memory, None)
        };
    }
    pub fn create_pipeline(&mut self, vertex_path: &str, frag_path: &str) {
        let vertex_shader_code = RippleBase::read_shader_file(vertex_path).expect("Failed to read vertex shader file!");
        let frag_shader_code = RippleBase::read_shader_file(frag_path).expect("Failed to read frag shader file!");
        let vertex_shader = self.base.create_shader_module(&vertex_shader_code);
        let frag_shader = self.base.create_shader_module(&frag_shader_code);
        let dynamic_states = &[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };
        let pname = CString::new("main").unwrap();
        let vertex_stage_info = vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            stage: vk::ShaderStageFlags::VERTEX,
            module: vertex_shader,
            p_name: pname.as_ptr(),
            ..Default::default()
        };
        let frag_stage_info = vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: frag_shader,
            p_name: pname.as_ptr(),
            ..Default::default()
        };
        let shader_stages = &[vertex_stage_info, frag_stage_info];
        let bind_description = Vertex::get_binding_description();
        let attribute_description = Vertex::get_attribute_description();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &bind_description,
            vertex_attribute_description_count: attribute_description.len() as u32,
            p_vertex_attribute_descriptions: attribute_description.as_ptr(),
            ..Default::default()
        };
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        let viewport_info = vk::PipelineViewportStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_viewports: std::ptr::null(),
            viewport_count: 1,
            scissor_count: 1,
            p_scissors: std::ptr::null(),
            ..Default::default()
        };
        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            depth_bias_clamp: 0.0,
            depth_bias_enable: vk::FALSE,
            depth_bias_slope_factor: 0.0,
            depth_bias_constant_factor: 0.0,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            rasterizer_discard_enable: vk::FALSE,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::CLOCKWISE,
            ..Default::default()
        };
        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            min_sample_shading: 1.0,
            p_sample_mask: std::ptr::null(),
            alpha_to_one_enable: vk::FALSE,
            alpha_to_coverage_enable: vk::FALSE,
            ..Default::default()
        };
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            color_blend_op: vk::BlendOp::ADD,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_write_mask: vk::ColorComponentFlags::R | vk::ColorComponentFlags::G | vk::ColorComponentFlags::B | vk::ColorComponentFlags::A
        };
        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            logic_op: vk::LogicOp::COPY,
            logic_op_enable: vk::FALSE,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            depth_test_enable: vk::FALSE,
            depth_write_enable: vk::FALSE,
            depth_compare_op: vk::CompareOp::LESS,
            depth_bounds_test_enable: vk::FALSE,
            stencil_test_enable: vk::FALSE,
            ..Default::default()
        };
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<TextPushConstants>() as u32,
        };
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_set_layouts: &self.descriptor_layout,
            set_layout_count: 1,
            push_constant_range_count: 1,
            p_push_constant_ranges: &push_constant_range,
            ..Default::default()
        };
        self.pipeline_layout = unsafe {
            self.base.core.device.create_pipeline_layout(&pipeline_layout_info, None).expect("Failed to text pipeline layout!") 
        };
        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
            stage_count: 2,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_info,
            p_dynamic_state: &dynamic_state,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            p_rasterization_state: &rasterizer,
            p_depth_stencil_state: &depth_stencil,
            layout: self.pipeline_layout,
            render_pass: self.base.state.read().unwrap().screen_render_pass,
            subpass: 0,
            base_pipeline_index: -1,
            base_pipeline_handle: vk::Pipeline::null(),
            ..Default::default()
        };
        self.pipeline = unsafe {
            self.base.core.device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None).expect("Failed to create graphics pipeline!")[0]
        };
        unsafe {
            self.base.core.device.destroy_shader_module(vertex_shader, None);
            self.base.core.device.destroy_shader_module(frag_shader, None)
        };
    }
    pub fn render_text(&mut self, text_info: &[RenderTextInfo]) {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            command_pool: self.base.state.read().unwrap().command_pool,
            level: vk::CommandBufferLevel::SECONDARY,
            command_buffer_count: 1,
            ..Default::default()
        };
        self.secondary_command_buffer = unsafe {
            self.base.core.device.allocate_command_buffers(&alloc_info).expect("Failed to allocate secondary text command buffers!")[0]
        };
        let inheritance_info = vk::CommandBufferInheritanceInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_INHERITANCE_INFO,
            render_pass: self.base.state.read().unwrap().screen_render_pass,
            subpass: 0,
            ..Default::default()
        };
        let begin_info = vk::CommandBufferBeginInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
            p_inheritance_info: &inheritance_info,
             flags: vk::CommandBufferUsageFlags::RENDER_PASS_CONTINUE,
            ..Default::default()
        };
        unsafe {
            self.base.core.device.begin_command_buffer(self.secondary_command_buffer, &begin_info).expect("Failed to record text command buffer!");
            self.base.core.device.cmd_bind_pipeline(self.secondary_command_buffer,vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            self.base.core.device.cmd_bind_vertex_buffers(self.secondary_command_buffer, 0, &[self.vertex_buffer], &[0]);
            text_info.iter().for_each(|info| {
                self.base.core.device.cmd_bind_descriptor_sets(self.secondary_command_buffer, 
                    vk::PipelineBindPoint::GRAPHICS, self.pipeline_layout, 0, 
                    &[self.atlases[info.atlas_id as usize].atlas_descriptor_set], &[]);
                let mut cursor: glam::Vec2 = glam::Vec2::new(info.x, info.y);
                let codes: Vec<u32> = info.text.chars().map(|ch| ch as u32).collect();
                codes.iter().for_each(|codepoint| {
                    if self.atlases_glyphs[info.atlas_id as usize].contains_key(codepoint) {
                        let glyph_info = self.atlases_glyphs[info.atlas_id as usize].get(codepoint).unwrap();
                        let push = TextPushConstants {
                            scale: info.scale,
                            projection: self.projection,
                            color: info.color,
                            _padding: 0.0,
                            position: glam::Vec2::new(cursor.x + glyph_info.bearing.x * info.scale, cursor.y - glyph_info.bearing.y * info.scale),
                            uv_min: glyph_info.uv_min,
                            uv_max: glyph_info.uv_max,
                            glyph_size: glam::Vec2::new(glyph_info.width as f32, glyph_info.height as f32)
                        };
                        let bytes = std::slice::from_raw_parts(&push as *const TextPushConstants as *const u8, std::mem::size_of::<TextPushConstants>());
                        self.base.core.device.cmd_push_constants(self.secondary_command_buffer, self.pipeline_layout, vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT, 0, bytes);
                        self.base.core.device.cmd_draw(self.secondary_command_buffer, 6,1,0,0);
                        cursor.x += (glyph_info.advance / 64) as f32 * info.scale;
                    };
                });
            });
            self.base.core.device.end_command_buffer(self.secondary_command_buffer).expect("Failed to end record text command buffer!");
        };
    }
    pub fn re_render(&mut self, text_info: &[RenderTextInfo]) {
        unsafe {
            self.base.core.device.reset_command_buffer(self.secondary_command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("Failed to reset text command buffer!");
        };
            self.render_text(text_info);
    }
    pub fn get_secondary_buffer(&self) -> vk::CommandBuffer {
        self.secondary_command_buffer
    }
    pub fn free(&self) {
        unsafe {
            self.base.core.device.destroy_descriptor_set_layout(self.descriptor_layout, None);
            self.atlases.iter().for_each(|atlas| {
                self.base.core.device.destroy_image(atlas.atlas_image, None);
                self.base.core.device.destroy_image_view(atlas.atlas_image_view, None);
                self.base.core.device.destroy_sampler(atlas.atlas_sampler, None);
                self.base.core.device.free_memory(atlas.atlas_memory, None);
                self.base.core.device.free_descriptor_sets(self.descriptor_pool, &[atlas.atlas_descriptor_set])
                    .expect("Failed to free descriptor set atlas!");
            });
            self.base.core.device.free_command_buffers(self.base.state.read().unwrap().command_pool, &[self.secondary_command_buffer]);
            self.base.core.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.base.core.device.destroy_pipeline(self.pipeline, None);
            self.base.core.device.destroy_buffer(self.vertex_buffer, None);
            self.base.core.device.free_memory(self.vertex_memory, None);
        }
    }
}
