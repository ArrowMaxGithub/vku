use super::vma_buffer::VMABuffer;
use crate::imports::Result;
use crate::{imports::*, VkDestroy};

pub struct ComputeShader {
    pipeline: Pipeline,
    layout: PipelineLayout,
    desc_pool: DescriptorPool,
    desc_set_layout: DescriptorSetLayout,
    desc_sets: Vec<DescriptorSet>,
    group_size_x: u32,
    group_size_y: u32,
    group_size_z: u32,
}

impl ComputeShader {
    pub fn from_src<Push>(
        device: impl AsRef<Device>,
        ssbos: &[&VMABuffer],
        shader_path: String,
        group_size_x: u32,
        group_size_y: u32,
        group_size_z: u32,
        additional_spec_consts: &[u32],
    ) -> Result<ComputeShader> {
        let mut spv_file = Cursor::new(std::fs::read(Path::new(&shader_path))?);
        let code = read_spv(&mut spv_file)?;
        let module_info = ShaderModuleCreateInfo::builder().code(&code);
        let module = unsafe {device.as_ref().create_shader_module(&module_info, None)}?;

        let mut spec_consts_data: Vec<u8> = vec![];
        spec_consts_data.extend(group_size_x.to_ne_bytes());
        spec_consts_data.extend(group_size_y.to_ne_bytes());
        spec_consts_data.extend(group_size_z.to_ne_bytes());

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
            .descriptor_count(ssbos.len() as u32);

        let desc_pool_create_info = DescriptorPoolCreateInfo::builder()
            .max_sets(1)
            .pool_sizes(&[*pool_size])
            .build();

        let desc_pool = unsafe {device.as_ref().create_descriptor_pool(&desc_pool_create_info, None)}?;

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

        let desc_set_layout = unsafe {device.as_ref().create_descriptor_set_layout(&desc_set_layout_info, None)?};

        let alloc_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(desc_pool)
            .set_layouts(&[desc_set_layout])
            .build();

        let desc_sets = unsafe {device.as_ref().allocate_descriptor_sets(&alloc_info)?};

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

        unsafe {device.as_ref().update_descriptor_sets(&write_sets, &[]);}

        let layout_info = PipelineLayoutCreateInfo::builder()
            .set_layouts(&[desc_set_layout])
            .push_constant_ranges(&push_constants_ranges)
            .build();

        let layout = unsafe {device.as_ref().create_pipeline_layout(&layout_info, None)?};

        let pipeline_info = ComputePipelineCreateInfo::builder()
            .stage(*shader_stage_info)
            .layout(layout);

        let pipeline = unsafe {device.as_ref()
            .create_compute_pipelines(PipelineCache::null(), &[*pipeline_info], None)
            .unwrap()[0]};

        unsafe {device.as_ref().destroy_shader_module(module, None);}

        Ok(Self {
            pipeline,
            layout,
            desc_pool,
            desc_set_layout,
            desc_sets,
            group_size_x,
            group_size_y,
            group_size_z,
        })
    }

    pub fn bind(&self, device: &Device, cmd_buffer: &CommandBuffer, constants: &[u8]) {
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
        device: &Device,
        cmd_buffer: &CommandBuffer,
        dispatch_x: u32,
        dispatch_y: u32,
        dispatch_z: u32,
    ) {
        unsafe {
            device.cmd_dispatch(
                *cmd_buffer,
                (dispatch_x / self.group_size_x).max(1),
                (dispatch_y / self.group_size_y).max(1),
                (dispatch_z / self.group_size_z).max(1),
            );
        }
    }
}

impl VkDestroy for ComputeShader{
    fn destroy(&self, vk_init: &crate::VkInit) -> Result<()> {
        unsafe {
            vk_init.device.destroy_pipeline_layout(self.layout, None);
            vk_init.device.destroy_pipeline(self.pipeline, None);
            vk_init.device.destroy_descriptor_set_layout(self.desc_set_layout, None);
            vk_init.device.destroy_descriptor_pool(self.desc_pool, None);
        }
        Ok(())
    }
}