use super::vma_buffer::VMABuffer;
use crate::{imports::*, VkInit};

/// A single stage compute shader.
pub struct ComputeShader {
    pipeline: Pipeline,
    layout: PipelineLayout,
    desc_pool: DescriptorPool,
    desc_set_layout: DescriptorSetLayout,
    desc_sets: Vec<DescriptorSet>,
    group_sizes: [u32; 3],
}

impl VkInit {
    /// Only SSBOs are supported as bindings.
    ///
    /// Group sizes are read in as specialization constants: layout(local_size_x_id = 0, local_size_y_id = 1, local_size_z_id = 2) in;

    pub fn create_compute_shader<Push>(
        &self,
        ssbos: &[&VMABuffer],
        code: Vec<u32>,
        group_sizes: [u32; 3],
        additional_spec_consts: &[u32],
        base_debug_name: String,
    ) -> Result<ComputeShader, Error> {
        let module_info = ShaderModuleCreateInfo::builder().code(&code);
        let module = unsafe { self.device.create_shader_module(&module_info, None) }?;
        self.set_debug_object_name(
            module.as_raw(),
            ObjectType::SHADER_MODULE,
            format!("{base_debug_name}_Compute_Shader_Module"),
        )?;

        let mut spec_consts_data: Vec<u8> = vec![];
        spec_consts_data.extend(group_sizes[0].to_ne_bytes());
        spec_consts_data.extend(group_sizes[1].to_ne_bytes());
        spec_consts_data.extend(group_sizes[2].to_ne_bytes());

        for v in additional_spec_consts {
            spec_consts_data.extend(v.to_ne_bytes());
        }

        let spec_consts_map_entries: Vec<SpecializationMapEntry> = (0..spec_consts_data.len() / 4)
            .enumerate()
            .map(|(i, _)| SpecializationMapEntry {
                constant_id: i as u32,
                offset: i as u32 * 4,
                size: 4,
            })
            .collect();

        let spec_consts_info = SpecializationInfo::builder()
            .map_entries(&spec_consts_map_entries)
            .data(&spec_consts_data);

        let shader_entry_name = CString::new("main")?;
        let shader_stage_info = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::COMPUTE)
            .module(module)
            .specialization_info(&spec_consts_info)
            .name(&shader_entry_name);

        let push_constants_ranges = [PushConstantRange::builder()
            .offset(0)
            .size(size_of::<Push>() as u32)
            .stage_flags(ShaderStageFlags::COMPUTE)
            .build()];

        let pool_size = DescriptorPoolSize::builder()
            .ty(DescriptorType::STORAGE_BUFFER)
            .descriptor_count(ssbos.len() as u32)
            .build();

        let pool_sizes = [pool_size];
        let desc_pool_create_info = DescriptorPoolCreateInfo::builder()
            .max_sets(1)
            .pool_sizes(&pool_sizes)
            .build();

        let desc_pool = unsafe {
            self.device
                .create_descriptor_pool(&desc_pool_create_info, None)
        }?;
        self.set_debug_object_name(
            desc_pool.as_raw(),
            ObjectType::DESCRIPTOR_POOL,
            format!("{base_debug_name}_Descriptor_Pool"),
        )?;

        let mut layout_bindings: Vec<DescriptorSetLayoutBinding> = Vec::new();
        let mut descriptor_buffers: Vec<DescriptorBufferInfo> = Vec::new();

        for (index, vma_buffer) in ssbos.iter().enumerate() {
            let layout_binding = DescriptorSetLayoutBinding {
                binding: index as u32,
                descriptor_type: DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: ShaderStageFlags::COMPUTE,
                ..Default::default()
            };
            layout_bindings.push(layout_binding);

            let descriptor_buffer = DescriptorBufferInfo {
                buffer: vma_buffer.buffer,
                offset: 0,
                range: WHOLE_SIZE,
            };
            descriptor_buffers.push(descriptor_buffer);
        }

        let desc_set_layout_info = DescriptorSetLayoutCreateInfo::builder()
            .bindings(&layout_bindings)
            .build();

        let desc_set_layout = unsafe {
            self.device
                .create_descriptor_set_layout(&desc_set_layout_info, None)?
        };
        self.set_debug_object_name(
            desc_set_layout.as_raw(),
            ObjectType::DESCRIPTOR_SET_LAYOUT,
            format!("{base_debug_name}_SSBO_Desc_Layout"),
        )?;

        let desc_set_layouts = [desc_set_layout];
        let alloc_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(desc_pool)
            .set_layouts(&desc_set_layouts)
            .build();

        let desc_sets = unsafe { self.device.allocate_descriptor_sets(&alloc_info)? };
        for (i, set) in desc_sets.iter().enumerate() {
            self.set_debug_object_name(
                set.as_raw(),
                ObjectType::DESCRIPTOR_SET,
                format!("{base_debug_name}_SSBO_Set_{i}"),
            )?;
        }

        let mut write_sets: Vec<ash::vk::WriteDescriptorSet> = Vec::new();
        for (index, descriptor_buffer) in descriptor_buffers.iter().enumerate() {
            let write_set = ash::vk::WriteDescriptorSet {
                dst_set: desc_sets[0],
                dst_binding: index as u32,
                descriptor_count: 1,
                descriptor_type: ash::vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: descriptor_buffer,
                ..Default::default()
            };
            write_sets.push(write_set);
        }

        unsafe {
            self.device.update_descriptor_sets(&write_sets, &[]);
        }

        let pipeline_layout_info = PipelineLayoutCreateInfo::builder()
            .set_layouts(&desc_set_layouts)
            .push_constant_ranges(&push_constants_ranges)
            .build();

        let pipeline_layout = unsafe {
            self.device
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };
        self.set_debug_object_name(
            pipeline_layout.as_raw(),
            ObjectType::PIPELINE_LAYOUT,
            format!("{base_debug_name}_Pipeline_Layout"),
        )?;

        let pipeline_info = ComputePipelineCreateInfo::builder()
            .stage(*shader_stage_info)
            .layout(pipeline_layout);

        let pipeline = unsafe {
            match self.device.create_compute_pipelines(
                PipelineCache::null(),
                &[*pipeline_info],
                None,
            ) {
                Ok(pipeline) => pipeline[0],
                Err((_, e)) => return Err(e.into()),
            }
        };
        self.set_debug_object_name(
            pipeline.as_raw(),
            ObjectType::PIPELINE,
            format!("{base_debug_name}_Pipeline"),
        )?;

        Ok(ComputeShader {
            pipeline,
            layout: pipeline_layout,
            desc_pool,
            desc_set_layout,
            desc_sets,
            group_sizes,
        })
    }
}

impl ComputeShader {
    pub fn destroy(&self, vk_init: &crate::VkInit) -> Result<(), Error> {
        unsafe {
            vk_init.device.destroy_pipeline_layout(self.layout, None);
            vk_init.device.destroy_pipeline(self.pipeline, None);
            vk_init
                .device
                .destroy_descriptor_set_layout(self.desc_set_layout, None);
            vk_init.device.destroy_descriptor_pool(self.desc_pool, None);
        }
        Ok(())
    }

    pub fn bind(&self, device: &ash::Device, cmd_buffer: &CommandBuffer, constants: &[u8]) {
        unsafe {
            device.cmd_bind_pipeline(*cmd_buffer, PipelineBindPoint::COMPUTE, self.pipeline);
            device.cmd_bind_descriptor_sets(
                *cmd_buffer,
                PipelineBindPoint::COMPUTE,
                self.layout,
                0,
                &self.desc_sets,
                &[],
            );
            device.cmd_push_constants(
                *cmd_buffer,
                self.layout,
                ShaderStageFlags::COMPUTE,
                0,
                constants,
            );
        }
    }

    pub fn dispatch(
        &self,
        device: &ash::Device,
        cmd_buffer: &CommandBuffer,
        dispatch_x: u32,
        dispatch_y: u32,
        dispatch_z: u32,
    ) {
        unsafe {
            device.cmd_dispatch(
                *cmd_buffer,
                (dispatch_x / self.group_sizes[0]).max(1),
                (dispatch_y / self.group_sizes[1]).max(1),
                (dispatch_z / self.group_sizes[2]).max(1),
            );
        }
    }
}
