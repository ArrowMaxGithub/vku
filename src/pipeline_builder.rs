use ash::util::read_spv;
use ash::vk::*;
use ash::Device;
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

impl Default for DepthInfo {
    fn default() -> Self {
        Self {
            test: false,
            write: false,
            comp_op: CompareOp::LESS_OR_EQUAL,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

impl DepthInfo {
    pub fn enabled_positive_depth() -> Self {
        DepthInfo {
            test: true,
            write: true,
            comp_op: CompareOp::LESS_OR_EQUAL,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

pub struct StencilInfo {
    pub test: bool,
    pub front: StencilOpState,
    pub back: StencilOpState,
}

impl Default for StencilInfo {
    fn default() -> Self {
        Self {
            test: false,
            front: Default::default(),
            back: Default::default(),
        }
    }
}

/// Trait for client code to convert vertex struct to [VertexInputBindingDescription] and [VertexInputAttributeDescription].
pub trait VertexConvert {
    fn binding_desc() -> &'static [VertexInputBindingDescription];
    fn attrib_desc() -> &'static [VertexInputAttributeDescription];
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
    pub renderpass: RenderPass,
}

impl VKUPipeline {
    pub fn builder() -> VKUPipelineBuilder {
        VKUPipelineBuilder::default()
    }

    pub fn destroy(&mut self, device: &Device) -> Result<(), Error> {
        unsafe {
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_render_pass(self.renderpass, None);
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct VKUPipelineBuilder {
    layout_push_constants_range: PushConstantRange,
    layout_desc_set_layout_bindings: Vec<DescriptorSetLayoutBinding>,

    renderpass_attachments: Vec<Vec<AttachmentDescription>>,
    renderpass_subpasses: Vec<Vec<SubpassDescription>>,
    renderpass_dependencies: Vec<Vec<SubpassDependency>>,

    pipeline_stages: Vec<PipelineShaderStageCreateInfo>,
    pipeline_vertex_input: PipelineVertexInputStateCreateInfo,
    pipeline_input_assembly: PipelineInputAssemblyStateCreateInfo,
    pipeline_tesselation: PipelineTessellationStateCreateInfo,
    pipeline_viewport: PipelineViewportStateCreateInfo,
    pipeline_rasterization: PipelineRasterizationStateCreateInfo,
    pipeline_multisample: PipelineMultisampleStateCreateInfo,
    pipeline_depthstencil: PipelineDepthStencilStateCreateInfo,
    pipeline_colorblend: PipelineColorBlendStateCreateInfo,
    pipeline_dynamic: PipelineDynamicStateCreateInfo,
}

impl VKUPipelineBuilder {
    pub fn build(self, device: &Device) -> Result<VKUPipeline, Error> {
        let desc_set_layout_create_info = DescriptorSetLayoutCreateInfo::builder()
            .flags(DescriptorSetLayoutCreateFlags::empty())
            .bindings(&self.layout_desc_set_layout_bindings)
            .build();

        let desc_set_layout =
            unsafe { device.create_descriptor_set_layout(&desc_set_layout_create_info, None)? };

        let layout_create_info = PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(&[self.layout_push_constants_range])
            .set_layouts(&[desc_set_layout])
            .build();

        let layout = unsafe { device.create_pipeline_layout(&layout_create_info, None)? };

        assert!(self.renderpass_attachments.len() == self.renderpass_subpasses.len());
        assert!(self.renderpass_subpasses.len() == self.renderpass_dependencies.len());

        let renderpass_create_infos: Vec<RenderPassCreateInfo> = self
            .renderpass_attachments
            .iter()
            .zip(self.renderpass_subpasses.iter())
            .zip(self.renderpass_dependencies.iter())
            .map(|((attachments, subpasses), dependencies)| {
                RenderPassCreateInfo::builder()
                    .attachments(attachments)
                    .subpasses(subpasses)
                    .dependencies(dependencies)
                    .build()
            })
            .collect();

        let renderpass: RenderPass =
            unsafe { Self::create_render_passes(device, &renderpass_create_infos)?[0] };

        let pipeline_create_info = GraphicsPipelineCreateInfo::builder()
            .stages(&self.pipeline_stages)
            .vertex_input_state(&self.pipeline_vertex_input)
            .input_assembly_state(&self.pipeline_input_assembly)
            .tessellation_state(&self.pipeline_tesselation)
            .viewport_state(&self.pipeline_viewport)
            .rasterization_state(&self.pipeline_rasterization)
            .multisample_state(&self.pipeline_multisample)
            .depth_stencil_state(&self.pipeline_depthstencil)
            .color_blend_state(&self.pipeline_colorblend)
            .dynamic_state(&self.pipeline_dynamic)
            .layout(layout)
            .render_pass(renderpass)
            .subpass(0)
            .build();

        let pipeline = unsafe { Self::create_pipeline(device, &[pipeline_create_info])? };

        Ok(VKUPipeline {
            desc_set_layout,
            layout,
            pipeline,
            renderpass,
        })
    }

    pub fn push_shader_stage(
        mut self,
        device: &Device,
        stage: ShaderStageFlags,
        path: impl AsRef<Path>,
        spec_constants: &[u32],
    ) -> Self {
        let shader_module = {
            let mut file = match std::fs::File::open(path.as_ref()) {
                Ok(file) => file,
                Err(e) => panic!(
                    "failed to open file at {:?}. Reason: {:?}",
                    path.as_ref(),
                    e
                ),
            };

            let spirv = match read_spv(&mut file) {
                Ok(spirv) => spirv,
                Err(e) => panic!("failed to read spirv from opened file. Reason: {e}"),
            };

            let create_info = ShaderModuleCreateInfo::builder()
                .flags(ShaderModuleCreateFlags::empty())
                .code(&spirv)
                .build();

            unsafe {
                match device.create_shader_module(&create_info, None) {
                    Ok(shader_module) => shader_module,
                    Err(e) => panic!("failed to create shader module from spirv. Reason: {e}"),
                }
            }
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
        self
    }

    pub fn push_render_pass(
        mut self,
        attachments: &[AttachmentDescription],
        subpasses: &[SubpassDescription],
        dependecies: &[SubpassDependency],
    ) -> Self {
        self.renderpass_attachments.push(attachments.to_vec());
        self.renderpass_subpasses.push(subpasses.to_vec());
        self.renderpass_dependencies.push(dependecies.to_vec());
        self
    }

    pub fn with_rasterization(
        mut self,
        polygon_mode: PolygonMode,
        cull_mode: CullModeFlags,
    ) -> Self {
        let rasterization = PipelineRasterizationStateCreateInfo::builder()
            .rasterizer_discard_enable(true)
            .front_face(FrontFace::COUNTER_CLOCKWISE)
            .polygon_mode(polygon_mode)
            .cull_mode(cull_mode)
            .build();

        self.pipeline_rasterization = rasterization;
        self
    }

    pub fn with_tesselation(mut self, patch_control_points: u32) -> Self {
        let tesselation = PipelineTessellationStateCreateInfo::builder()
            .patch_control_points(patch_control_points)
            .build();

        self.pipeline_tesselation = tesselation;
        self
    }

    pub fn with_viewport(mut self, viewports: &[Viewport]) -> Self {
        let viewport = PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .build();

        self.pipeline_viewport = viewport;
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

    pub fn with_vertex<V: VertexConvert>(mut self, primitive_topology: PrimitiveTopology) -> Self {
        let pipeline_vertex_input = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(V::binding_desc())
            .vertex_attribute_descriptions(V::attrib_desc())
            .build();

        let pipeline_input_assembly = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(primitive_topology)
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

    pub fn with_descriptors(
        mut self,
        descriptors: &[(DescriptorType, ShaderStageFlags, u32)],
    ) -> Self {
        let desc_set_layout_bindings: Vec<DescriptorSetLayoutBinding> = descriptors
            .iter()
            .enumerate()
            .map(|(index, (ty, stages, count))| {
                DescriptorSetLayoutBinding::builder()
                    .descriptor_count(*count)
                    .binding(index as u32)
                    .descriptor_type(*ty)
                    .stage_flags(*stages)
                    .build()
            })
            .collect();

        self.layout_desc_set_layout_bindings = desc_set_layout_bindings;
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
