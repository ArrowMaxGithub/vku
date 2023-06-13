use ash::util::read_spv;
use ash::vk::*;
use ash::Device;
use itertools::izip;
use std::ffi::CStr;
use std::mem::size_of;
use std::path::Path;
use std::result::Result;

use crate::Error;

pub struct DepthInfo {
    pub test: bool,
    pub write: bool,
    pub comp_op: CompareOp,
    pub min_depth: f32,
    pub max_depth: f32,
}

pub struct StencilInfo {
    pub test: bool,
    pub front: StencilOpState,
    pub back: StencilOpState,
}

/// Trait for client code to convert vertex struct to [VertexInputBindingDescription] and [VertexInputAttributeDescription].
pub trait VertexConvert {
    fn binding_desc() -> Vec<VertexInputBindingDescription>;
    fn attrib_desc() -> Vec<VertexInputAttributeDescription>;
}

/// Shortcut to generate [PipelineColorBlendAttachmentState] for common blend modes.
#[derive(Clone, Copy)]
pub enum BlendMode {
    Opaque,
    TraditionalTransparency,
    PremultipliedTransparency,
}

impl From<BlendMode> for PipelineColorBlendAttachmentState {
    fn from(val: BlendMode) -> Self {
        match val {
            BlendMode::Opaque => PipelineColorBlendAttachmentState::builder()
                .color_write_mask(ColorComponentFlags::RGBA)
                .blend_enable(false)
                .build(),

            BlendMode::TraditionalTransparency => PipelineColorBlendAttachmentState::builder()
                .color_write_mask(ColorComponentFlags::RGBA)
                .blend_enable(true)
                .color_blend_op(BlendOp::ADD)
                .src_color_blend_factor(BlendFactor::SRC_ALPHA)
                .src_alpha_blend_factor(BlendFactor::ONE)
                .alpha_blend_op(BlendOp::ADD)
                .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
                .dst_alpha_blend_factor(BlendFactor::ZERO)
                .build(),

            BlendMode::PremultipliedTransparency => PipelineColorBlendAttachmentState::builder()
                .color_write_mask(ColorComponentFlags::RGBA)
                .blend_enable(true)
                .color_blend_op(BlendOp::ADD)
                .src_color_blend_factor(BlendFactor::ONE)
                .src_alpha_blend_factor(BlendFactor::ONE_MINUS_DST_ALPHA)
                .alpha_blend_op(BlendOp::ADD)
                .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
                .dst_alpha_blend_factor(BlendFactor::ONE)
                .build(),
        }
    }
}

pub struct VKUPipeline {
    pub desc_set_layout: DescriptorSetLayout,
    pub layout: PipelineLayout,
    pub pipeline: Pipeline,
    pub renderpasses: Vec<RenderPass>,
}

impl VKUPipeline {
    pub fn builder<V: VertexConvert>() -> VKUPipelineBuilder {
        VKUPipelineBuilder::default()
    }

    pub fn destroy(&mut self, device: &Device) -> Result<(), Error> {
        unsafe {
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_pipeline(self.pipeline, None);
            for renderpass in &mut self.renderpasses {
                device.destroy_render_pass(*renderpass, None);
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct VKUPipelineBuilder {
    layout_push_constants_range: PushConstantRange,
    layout_desc_set_layout_create_info: DescriptorSetLayoutCreateInfo,

    renderpass_attachments: Vec<Vec<AttachmentDescription>>,
    renderpass_subpasses: Vec<Vec<SubpassDescription>>,
    renderpass_dependencies: Vec<Vec<SubpassDependency>>,

    pipeline_stages: Vec<PipelineShaderStageCreateInfo>,
    pipeline_vertex_input: PipelineVertexInputStateCreateInfo,
    pipeline_input_assembly: PipelineInputAssemblyStateCreateInfo,
    pipeline_rasterization: PipelineRasterizationStateCreateInfo,
    pipeline_multisample: PipelineMultisampleStateCreateInfo,
    pipeline_depthstencil: PipelineDepthStencilStateCreateInfo,
    pipeline_colorblend: PipelineColorBlendStateCreateInfo,
    pipeline_dynamic: PipelineDynamicStateCreateInfo,
}

impl VKUPipelineBuilder {
    pub fn build(self, device: &Device) -> Result<VKUPipeline, Error> {
        let desc_set_layout = unsafe {
            device.create_descriptor_set_layout(&self.layout_desc_set_layout_create_info, None)?
        };

        let layout_create_info = PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&[self.layout_push_constants_range])
            .set_layouts(&[desc_set_layout])
            .build();

        let layout = unsafe { device.create_pipeline_layout(&layout_create_info, None)? };

        assert!(self.renderpass_attachments.len() == self.renderpass_subpasses.len());
        assert!(self.renderpass_subpasses.len() == self.renderpass_dependencies.len());

        let renderpass_create_infos: Vec<RenderPassCreateInfo> = izip!(
            self.renderpass_attachments,
            self.renderpass_subpasses,
            self.renderpass_dependencies
        )
        .map(|(attachments, subpasses, dependencies)| {
            RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(&subpasses)
                .dependencies(&dependencies)
                .build()
        })
        .collect();

        let renderpasses: Vec<RenderPass> =
            unsafe { Self::create_render_passes(device, &renderpass_create_infos)? };

        let pipeline_create_info = GraphicsPipelineCreateInfo::builder()
            .stages(&self.pipeline_stages)
            .vertex_input_state(&self.pipeline_vertex_input)
            .input_assembly_state(&self.pipeline_input_assembly)
            .rasterization_state(&self.pipeline_rasterization)
            .multisample_state(&self.pipeline_multisample)
            .depth_stencil_state(&self.pipeline_depthstencil)
            .color_blend_state(&self.pipeline_colorblend)
            .dynamic_state(&self.pipeline_dynamic)
            .layout(layout)
            .build();

        let pipeline = unsafe { Self::create_pipeline(device, &[pipeline_create_info])? };

        Ok(VKUPipeline {
            desc_set_layout,
            layout,
            pipeline,
            renderpasses,
        })
    }

    pub fn push_shader_stage(
        mut self,
        device: &Device,
        stage: ShaderStageFlags,
        path: impl AsRef<Path>,
        spec_constants: &[u32],
    ) -> Result<Self, Error> {
        let shader_module = {
            let mut file = std::fs::File::open(path)?;
            let spirv = read_spv(&mut file)?;
            let create_info = ShaderModuleCreateInfo::builder()
                .flags(ShaderModuleCreateFlags::empty())
                .code(&spirv)
                .build();

            unsafe { device.create_shader_module(&create_info, None)? }
        };

        let map_entries: Vec<SpecializationMapEntry> = spec_constants
            .iter()
            .enumerate()
            .map(|(index, _)| SpecializationMapEntry {
                constant_id: index as u32,
                offset: (index * size_of::<u32>()) as u32,
                size: size_of::<u32>(),
            })
            .collect();

        let bytes: Vec<u8> = spec_constants
            .iter()
            .flat_map(|c| c.to_ne_bytes())
            .collect();

        let spec_info = SpecializationInfo::builder()
            .map_entries(&map_entries)
            .data(&bytes)
            .build();

        let shader_stage_create_info = PipelineShaderStageCreateInfo::builder()
            .stage(stage)
            .module(shader_module)
            .name(CStr::from_bytes_with_nul(b"main\0").unwrap())
            .specialization_info(&spec_info)
            .build();

        self.pipeline_stages.push(shader_stage_create_info);
        Ok(self)
    }

    pub fn push_render_pass(
        mut self,
        attachments: &[AttachmentDescription],
        subpasses: &[SubpassDescription],
        dependecies: &[SubpassDependency],
    ) -> Result<Self, Error> {
        self.renderpass_attachments.push(attachments.to_vec());
        self.renderpass_subpasses.push(subpasses.to_vec());
        self.renderpass_dependencies.push(dependecies.to_vec());

        Ok(self)
    }

    pub fn with_rasterization(
        mut self,
        polygon_mode: PolygonMode,
        cull_mode: CullModeFlags,
    ) -> Self {
        let rasterization = PipelineRasterizationStateCreateInfo::builder()
            .front_face(FrontFace::COUNTER_CLOCKWISE)
            .polygon_mode(polygon_mode)
            .cull_mode(cull_mode)
            .build();

        self.pipeline_rasterization = rasterization;
        self
    }

    pub fn with_multisample(mut self, samples: SampleCountFlags) -> Self {
        let multisample = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(true)
            .rasterization_samples(samples)
            .build();

        self.pipeline_multisample = multisample;
        self
    }

    pub fn with_depthstencil(mut self, depth: DepthInfo, stencil: StencilInfo) -> Self {
        let depthstencil = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(depth.test)
            .depth_write_enable(depth.write)
            .depth_compare_op(depth.comp_op)
            .min_depth_bounds(depth.min_depth)
            .max_depth_bounds(depth.max_depth)
            .stencil_test_enable(stencil.test)
            .front(stencil.front)
            .back(stencil.back)
            .build();

        self.pipeline_depthstencil = depthstencil;
        self
    }

    pub fn with_colorblends(mut self, blend_modes: &[BlendMode]) -> Self {
        let attachments: Vec<PipelineColorBlendAttachmentState> = blend_modes
            .iter()
            .map(|mode| PipelineColorBlendAttachmentState::from(*mode))
            .collect();

        let colorblend = PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(true)
            .attachments(&attachments)
            .build();

        self.pipeline_colorblend = colorblend;
        self
    }

    pub fn with_dynamic(mut self, dynamic_states: &[DynamicState]) -> Self {
        let dynamic = PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(dynamic_states)
            .build();

        self.pipeline_dynamic = dynamic;
        self
    }

    pub fn with_vertex<V: VertexConvert>(mut self, vertex_topology: PrimitiveTopology) -> Self {
        let pipeline_vertex_input = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&V::binding_desc())
            .vertex_attribute_descriptions(&V::attrib_desc())
            .build();

        let pipeline_input_assembly = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vertex_topology)
            .build();

        self.pipeline_vertex_input = pipeline_vertex_input;
        self.pipeline_input_assembly = pipeline_input_assembly;
        self
    }

    pub fn with_push_constants<P>(mut self) -> Self {
        let size_of = size_of::<P>();
        let push_constants_range = PushConstantRange::builder()
            .offset(0)
            .size(size_of as u32)
            .stage_flags(ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT)
            .build();

        self.layout_push_constants_range = push_constants_range;
        self
    }

    pub fn with_descriptors(mut self, descriptors: &[(DescriptorType, ShaderStageFlags)]) -> Self {
        let bindings: Vec<DescriptorSetLayoutBinding> = descriptors
            .iter()
            .enumerate()
            .map(|(index, (ty, stages))| {
                DescriptorSetLayoutBinding::builder()
                    .binding(index as u32)
                    .descriptor_type(*ty)
                    .stage_flags(*stages)
                    .build()
            })
            .collect();

        let desc_set_layout_create_info = DescriptorSetLayoutCreateInfo::builder()
            .flags(DescriptorSetLayoutCreateFlags::empty())
            .bindings(&bindings)
            .build();

        self.layout_desc_set_layout_create_info = desc_set_layout_create_info;
        self
    }

    unsafe fn create_pipeline(
        device: &Device,
        create_infos: &[GraphicsPipelineCreateInfo],
    ) -> Result<Pipeline, Error> {
        match device.create_graphics_pipelines(PipelineCache::null(), create_infos, None) {
            Ok(pipeline) => Ok(pipeline[0]),
            Err(e) => Err(Error::VkError(e.1)),
        }
    }

    unsafe fn create_render_passes(
        device: &Device,
        render_pass_create_infos: &[RenderPassCreateInfo],
    ) -> Result<Vec<RenderPass>, Error> {
        render_pass_create_infos
            .iter()
            .map(|info| match device.create_render_pass(info, None) {
                Ok(render_pass) => Ok(render_pass),
                Err(e) => Err(Error::VkError(e)),
            })
            .collect()
    }
}
