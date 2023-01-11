use crate::{imports::*, VMABuffer, VkInit};

pub struct RendererCreateInfo {
    pub initial_buffer_length: usize,
    pub frames_in_flight: usize,
    pub topology: PrimitiveTopology,
    pub vertex_code_path: String,
    pub fragment_code_path: String,
    pub additional_usage_index_buffer: BufferUsageFlags,
    pub additional_usage_vertex_buffer: BufferUsageFlags,
}

pub struct BaseRenderer {
    pub index_buffers: Vec<VMABuffer>,
    pub vertex_buffers: Vec<VMABuffer>,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: Pipeline,
    pub descriptor_pool: DescriptorPool,
    pub sampled_image_desc_set_layout: DescriptorSetLayout,
    pub sampled_image_desc_set: DescriptorSet,
    pub sampler: Sampler,
}

pub trait VertexConvert {
    fn convert_to_vertex_input_binding_desc() -> Vec<VertexInputBindingDescription>;
    fn convert_to_vertex_input_attrib_desc() -> Vec<VertexInputAttributeDescription>;
}

impl VkInit {
    pub fn create_base_renderer<Index, Vertex, Push>(
        &self,
        create_info: &RendererCreateInfo,
    ) -> Result<BaseRenderer>
    where
        Vertex: VertexConvert,
    {
        let vertex_input_binding_desc = Vertex::convert_to_vertex_input_binding_desc();
        let vertex_input_atrtibute_desc = Vertex::convert_to_vertex_input_attrib_desc();
        let vertex_input_state_info = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_input_binding_desc)
            .vertex_attribute_descriptions(&vertex_input_atrtibute_desc);

        let index_size = size_of::<Index>() * create_info.initial_buffer_length;
        let vertex_size = size_of::<Vertex>() * create_info.initial_buffer_length;

        let index_buffers = self.create_cpu_to_gpu_buffers(
            index_size,
            create_info.additional_usage_index_buffer | BufferUsageFlags::INDEX_BUFFER,
            create_info.frames_in_flight,
        )?;
        let vertex_buffers = self.create_cpu_to_gpu_buffers(
            vertex_size,
            create_info.additional_usage_vertex_buffer | BufferUsageFlags::VERTEX_BUFFER,
            create_info.frames_in_flight,
        )?;

        let vertex_input_assembly_state_info = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(create_info.topology)
            .primitive_restart_enable(false);

        let sampled_image_size = [DescriptorPoolSize {
            ty: DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
        }];

        let descriptor_pool_info = DescriptorPoolCreateInfo::builder()
            .pool_sizes(&sampled_image_size)
            .max_sets(1);

        let descriptor_pool = unsafe {
            self.device
                .create_descriptor_pool(&descriptor_pool_info, None)?
        };

        let sampled_image_bindings = [DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::FRAGMENT)
            .build()];

        let sampled_image_desc_set_layout_create_info =
            DescriptorSetLayoutCreateInfo::builder().bindings(&sampled_image_bindings);

        let sampled_image_desc_set_layout = unsafe {
            self.device
                .create_descriptor_set_layout(&sampled_image_desc_set_layout_create_info, None)
                .unwrap()
        };

        let sampled_image_desc_set_alloc_info = DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&[sampled_image_desc_set_layout])
            .build();

        let sampled_image_desc_set = unsafe {
            self.device
                .allocate_descriptor_sets(&sampled_image_desc_set_alloc_info)?[0]
        };

        let sampler_info = SamplerCreateInfo::builder()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE)
            .mipmap_mode(SamplerMipmapMode::LINEAR);

        let sampler = unsafe { self.device.create_sampler(&sampler_info, None)? };

        let mut vertex_spv_file =
            Cursor::new(std::fs::read(Path::new(&create_info.vertex_code_path))?);
        let mut frag_spv_file =
            Cursor::new(std::fs::read(Path::new(&create_info.fragment_code_path))?);

        let vertex_code = read_spv(&mut vertex_spv_file)?;
        let vertex_shader_info = ShaderModuleCreateInfo::builder().code(&vertex_code);

        let frag_code = read_spv(&mut frag_spv_file)?;
        let frag_shader_info = ShaderModuleCreateInfo::builder().code(&frag_code);

        let vertex_shader_module = unsafe {
            self.device
                .create_shader_module(&vertex_shader_info, None)?
        };

        let fragment_shader_module =
            unsafe { self.device.create_shader_module(&frag_shader_info, None)? };

        let push_constant_ranges = [PushConstantRange::builder()
            .offset(0)
            .size(size_of::<Push>() as u32)
            .stage_flags(ShaderStageFlags::VERTEX)
            .build()];

        let pipeline_layout_create_info = PipelineLayoutCreateInfo::builder()
            .set_layouts(&[sampled_image_desc_set_layout])
            .push_constant_ranges(&push_constant_ranges)
            .build();

        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        let shader_entry_name = CString::new("main")?;
        let shader_stage_create_infos = [
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::VERTEX)
                .module(vertex_shader_module)
                .name(&shader_entry_name)
                .build(),
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_module)
                .name(&shader_entry_name)
                .build(),
        ];

        let rasterizer_info = PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(CullModeFlags::NONE)
            .front_face(FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

        let viewports = [Viewport::default()];
        let scissors = [Default::default()];
        let viewport_info = PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let multisampling_info = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);

        let color_blend_attachments = [PipelineColorBlendAttachmentState::builder()
            .color_write_mask(
                ColorComponentFlags::R
                    | ColorComponentFlags::G
                    | ColorComponentFlags::B
                    | ColorComponentFlags::A,
            )
            .blend_enable(true)
            .color_blend_op(BlendOp::ADD)
            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(BlendOp::ADD)
            .src_alpha_blend_factor(BlendFactor::ONE_MINUS_DST_ALPHA)
            .dst_alpha_blend_factor(BlendFactor::ONE)
            .build()];

        let color_blending_info =
            PipelineColorBlendStateCreateInfo::builder().attachments(&color_blend_attachments);

        let depth_stencil_state_create_info = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(CompareOp::NEVER)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build();

        let dynamic_states = [DynamicState::SCISSOR, DynamicState::VIEWPORT];
        let dynamic_states_info =
            PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);

        let format = [self.info.surface_info.format.format];
        let mut pipeline_rendering_info = PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(&format)
            .build();

        let pipeline_info_builder = GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .rasterization_state(&rasterizer_info)
            .viewport_state(&viewport_info)
            .multisample_state(&multisampling_info)
            .color_blend_state(&color_blending_info)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .dynamic_state(&dynamic_states_info)
            .layout(pipeline_layout);

        let pipeline_info = pipeline_info_builder.push_next(&mut pipeline_rendering_info);
        let pipeline_info_final = pipeline_info.build();

        let pipeline = unsafe {
            self.device
                .create_graphics_pipelines(PipelineCache::null(), &[pipeline_info_final], None)
                .unwrap()[0]
        };

        unsafe {
            self.device
                .destroy_shader_module(vertex_shader_module, None);
            self.device
                .destroy_shader_module(fragment_shader_module, None);
        }

        Ok(BaseRenderer {
            vertex_buffers,
            index_buffers,
            pipeline_layout,
            pipeline,
            descriptor_pool,
            sampled_image_desc_set_layout,
            sampled_image_desc_set,
            sampler,
        })
    }

    pub fn destroy_base_renderer(&self, renderer: &BaseRenderer) -> Result<()> {
        unsafe {
            for buffer in &renderer.index_buffers {
                buffer.destroy(&self.allocator)?;
            }
            for buffer in &renderer.vertex_buffers {
                buffer.destroy(&self.allocator)?;
            }
            self.device
                .destroy_pipeline_layout(renderer.pipeline_layout, None);
            self.device.destroy_pipeline(renderer.pipeline, None);
            self.device
                .destroy_descriptor_pool(renderer.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(renderer.sampled_image_desc_set_layout, None);
            self.device.destroy_sampler(renderer.sampler, None);
        }

        Ok(())
    }
}

pub trait RendererBarriers {
    fn extend_compute_acquire_barrier(
        &mut self,
        buffer_memory_barriers: &mut Vec<BufferMemoryBarrier2>,
        image_memory_barriers: &mut Vec<ImageMemoryBarrier2>,
        frame: usize,
    ) -> Result<()>;

    fn extend_compute_release_barrier(
        &mut self,
        buffer_memory_barriers: &mut Vec<BufferMemoryBarrier2>,
        image_memory_barriers: &mut Vec<ImageMemoryBarrier2>,
        frame: usize,
    ) -> Result<()>;

    fn extend_before_input_barrier(
        &mut self,
        buffer_memory_barriers: &mut Vec<BufferMemoryBarrier2>,
        image_memory_barriers: &mut Vec<ImageMemoryBarrier2>,
        frame: usize,
    ) -> Result<()>;

    fn extend_after_input_barrier(
        &mut self,
        buffer_memory_barriers: &mut Vec<BufferMemoryBarrier2>,
        image_memory_barriers: &mut Vec<ImageMemoryBarrier2>,
        frame: usize,
    ) -> Result<()>;

    fn extend_graphics_acquire_barrier(
        &mut self,
        buffer_memory_barriers: &mut Vec<BufferMemoryBarrier2>,
        image_memory_barriers: &mut Vec<ImageMemoryBarrier2>,
        frame: usize,
    ) -> Result<()>;

    fn extend_graphics_release_barrier(
        &mut self,
        buffer_memory_barriers: &mut Vec<BufferMemoryBarrier2>,
        image_memory_barriers: &mut Vec<ImageMemoryBarrier2>,
        frame: usize,
    ) -> Result<()>;
}
