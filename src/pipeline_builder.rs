use ash::util::read_spv;
use ash::vk::*;
use ash::Device;
use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;
use std::result::Result;

use crate::Error;
use crate::VkInit;

pub struct VKUPipeline {
    pub set_layout: DescriptorSetLayout,
    pub renderpass: RenderPass,
    pub layout: PipelineLayout,
    pub pipeline: Pipeline,
}

impl VKUPipeline {
    pub fn builder() -> VKUPipelineBuilder {
        VKUPipelineBuilder::default()
    }

    pub fn destroy(&mut self, device: &Device) -> Result<(), Error> {
        unsafe {
            device.destroy_descriptor_set_layout(self.set_layout, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_render_pass(self.renderpass, None);
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct VKUPipelineBuilder {
    pipeline_stages: Vec<(
        ShaderStageFlags,
        ShaderModule,
        Vec<u8>,
        Vec<SpecializationMapEntry>,
    )>,
    pipeline_vertex_input: (
        Vec<VertexInputBindingDescription>,
        Vec<VertexInputAttributeDescription>,
    ),
    pipeline_input_assembly: PrimitiveTopology,
    pipeline_tesselation: u32,
    pipeline_viewport: (Vec<Viewport>, Vec<Rect2D>),
    pipeline_rasterization: (PolygonMode, CullModeFlags),
    pipeline_multisample: SampleCountFlags,
    pipeline_depthstencil: (DepthInfo, StencilInfo),
    pipeline_colorblend: Vec<PipelineColorBlendAttachmentState>,
    pipeline_dynamic: Vec<DynamicState>,
    pipeline_layout: (
        Vec<DescriptorBindingFlags>,
        Vec<DescriptorSetLayoutBinding>,
        Vec<PushConstantRange>,
    ),
    pipeline_renderpass: (
        Vec<AttachmentDescription>,
        Vec<SubpassDescription>,
        Vec<SubpassDependency>,
    ),
}

impl VKUPipelineBuilder {
    pub fn build(self, vk_init: &VkInit, base_name: &str) -> Result<VKUPipeline, Error> {
        let (bindings, attribs) = self.pipeline_vertex_input;
        let pipeline_vertex_input = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attribs)
            .build();

        let topology = self.pipeline_input_assembly;
        let pipeline_input_assembly = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(topology)
            .build();

        let patch_control_points = self.pipeline_tesselation;
        let pipeline_tesselation = PipelineTessellationStateCreateInfo::builder()
            .patch_control_points(patch_control_points)
            .build();

        let (viewports, scissors) = self.pipeline_viewport;
        let pipeline_viewport = PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors)
            .build();

        let (polygon_mode, cull_mode) = self.pipeline_rasterization;
        let pipeline_rasterization = PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(polygon_mode)
            .cull_mode(cull_mode)
            .front_face(FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0)
            .build();

        let samples = self.pipeline_multisample;
        let pipeline_multisample = PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(samples)
            .build();

        let (depth_info, stencil_info) = self.pipeline_depthstencil;
        let pipeline_depthstencil = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(depth_info.test)
            .depth_write_enable(depth_info.write)
            .depth_compare_op(depth_info.comp_op)
            .min_depth_bounds(depth_info.min_depth)
            .max_depth_bounds(depth_info.max_depth)
            .stencil_test_enable(stencil_info.test)
            .front(stencil_info.front)
            .back(stencil_info.back)
            .build();

        let attachments = self.pipeline_colorblend;
        let pipeline_colorblend = PipelineColorBlendStateCreateInfo::builder()
            .attachments(&attachments)
            .build();

        let dynamic_states = self.pipeline_dynamic;
        let pipeline_dynamic = PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_states)
            .build();

        let spec_infos: Vec<SpecializationInfo> = self
            .pipeline_stages
            .iter()
            .map(|(_, _, data, map_entries)| {
                SpecializationInfo::builder()
                    .map_entries(map_entries)
                    .data(data)
                    .build()
            })
            .collect();

        let entry_name = CString::new("main")?;
        let pipeline_stages: Vec<PipelineShaderStageCreateInfo> = self
            .pipeline_stages
            .iter()
            .zip(spec_infos.iter())
            .map(|((stage, module, _, _), info)| {
                PipelineShaderStageCreateInfo::builder()
                    .stage(*stage)
                    .module(*module)
                    .specialization_info(info)
                    .name(&entry_name)
                    .build()
            })
            .collect();

        let (update_after_bind, bindings, push_constant_ranges) = self.pipeline_layout;

        let mut desc_set_binding_flags = DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&update_after_bind)
            .build();

        let flags = if update_after_bind
            .iter()
            .any(|flag| flag == &DescriptorBindingFlags::UPDATE_AFTER_BIND)
        {
            DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL
        } else {
            DescriptorSetLayoutCreateFlags::empty()
        };

        let set_layouts = {
            let create_info = DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings)
                .flags(flags)
                .push_next(&mut desc_set_binding_flags)
                .build();

            unsafe {
                vec![vk_init
                    .device
                    .create_descriptor_set_layout(&create_info, None)?]
            }
        };

        let layout = {
            let create_info = PipelineLayoutCreateInfo::builder()
                .set_layouts(&set_layouts)
                .push_constant_ranges(&push_constant_ranges)
                .build();

            unsafe { vk_init.device.create_pipeline_layout(&create_info, None)? }
        };

        let (attachments, subpasses, dependencies) = self.pipeline_renderpass;
        let renderpass = {
            let create_info = RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(&subpasses)
                .dependencies(&dependencies)
                .build();

            unsafe { vk_init.device.create_render_pass(&create_info, None)? }
        };

        let pipeline_create_info = GraphicsPipelineCreateInfo::builder()
            .vertex_input_state(&pipeline_vertex_input)
            .input_assembly_state(&pipeline_input_assembly)
            .tessellation_state(&pipeline_tesselation)
            .viewport_state(&pipeline_viewport)
            .rasterization_state(&pipeline_rasterization)
            .multisample_state(&pipeline_multisample)
            .depth_stencil_state(&pipeline_depthstencil)
            .color_blend_state(&pipeline_colorblend)
            .dynamic_state(&pipeline_dynamic)
            .stages(&pipeline_stages)
            .layout(layout)
            .render_pass(renderpass)
            .subpass(0)
            .build();

        let pipeline = unsafe { Self::create_pipeline(vk_init, &[pipeline_create_info])? };

        for (_, module, _, _) in self.pipeline_stages {
            unsafe { vk_init.device.destroy_shader_module(module, None) }
        }

        vk_init.set_debug_object_name(
            set_layouts[0].as_raw(),
            ObjectType::DESCRIPTOR_SET_LAYOUT,
            format!("{base_name}_Desc_Set_Layout"),
        )?;
        vk_init.set_debug_object_name(
            layout.as_raw(),
            ObjectType::PIPELINE_LAYOUT,
            format!("{base_name}_Pipeline_Layout"),
        )?;
        vk_init.set_debug_object_name(
            pipeline.as_raw(),
            ObjectType::PIPELINE,
            format!("{base_name}_Pipeline"),
        )?;
        vk_init.set_debug_object_name(
            renderpass.as_raw(),
            ObjectType::RENDER_PASS,
            format!("{base_name}_Renderpass"),
        )?;

        Ok(VKUPipeline {
            set_layout: set_layouts[0],
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
    ) -> Result<Self, Error> {
        let module = {
            let mut file = std::fs::File::open(path.as_ref())?;

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

        let data: Vec<u8> = spec_constants
            .iter()
            .flat_map(|c| c.to_ne_bytes())
            .collect();

        self.pipeline_stages
            .push((stage, module, data, map_entries));
        Ok(self)
    }

    pub fn push_shader_stage_spirv(
        mut self,
        device: &Device,
        stage: ShaderStageFlags,
        spirv: &[u32],
        spec_constants: &[u32],
    ) -> Result<Self, Error> {
        let module = {
            let create_info = ShaderModuleCreateInfo::builder()
                .flags(ShaderModuleCreateFlags::empty())
                .code(spirv)
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

        let data: Vec<u8> = spec_constants
            .iter()
            .flat_map(|c| c.to_ne_bytes())
            .collect();

        self.pipeline_stages
            .push((stage, module, data, map_entries));
        Ok(self)
    }

    #[cfg(feature = "shader")]
    pub fn push_shader_stage_glsl(
        mut self,
        device: &Device,
        stage: ShaderStageFlags,
        glsl: String,
        spec_constants: &[u32],
    ) -> Result<Self, Error> {
        let ext = match stage {
            ShaderStageFlags::VERTEX => "vert",
            ShaderStageFlags::FRAGMENT => "frag",
            _ => return Err(Error::UnknownShaderFileExtension),
        };

        let compiled = crate::shader::shader_ad_hoc(glsl, "", ext, false)?;

        let module = {
            let create_info = ShaderModuleCreateInfo::builder()
                .flags(ShaderModuleCreateFlags::empty())
                .code(compiled.as_binary())
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

        let data: Vec<u8> = spec_constants
            .iter()
            .flat_map(|c| c.to_ne_bytes())
            .collect();

        self.pipeline_stages
            .push((stage, module, data, map_entries));

        Ok(self)
    }

    pub fn with_render_pass(
        mut self,
        attachments: &[AttachmentDescription],
        subpasses: &[SubpassDescription],
        dependecies: &[SubpassDependency],
    ) -> Self {
        self.pipeline_renderpass = (
            attachments.to_vec(),
            subpasses.to_vec(),
            dependecies.to_vec(),
        );
        self
    }

    pub fn with_rasterization(
        mut self,
        polygon_mode: PolygonMode,
        cull_mode: CullModeFlags,
    ) -> Self {
        self.pipeline_rasterization = (polygon_mode, cull_mode);
        self
    }

    pub fn with_tesselation(mut self, patch_control_points: u32) -> Self {
        self.pipeline_tesselation = patch_control_points;
        self
    }

    pub fn with_viewports_scissors(mut self, viewports: &[Viewport], scissors: &[Rect2D]) -> Self {
        self.pipeline_viewport = (viewports.to_vec(), scissors.to_vec());
        self
    }

    pub fn with_multisample(mut self, samples: SampleCountFlags) -> Self {
        self.pipeline_multisample = samples;
        self
    }

    pub fn with_depthstencil(mut self, depth: DepthInfo, stencil: StencilInfo) -> Self {
        self.pipeline_depthstencil = (depth, stencil);
        self
    }

    pub fn with_colorblends(mut self, blend_modes: &[BlendMode]) -> Self {
        let attachments: Vec<PipelineColorBlendAttachmentState> = blend_modes
            .iter()
            .map(|mode| PipelineColorBlendAttachmentState::from(*mode))
            .collect();

        self.pipeline_colorblend = attachments;
        self
    }

    pub fn with_dynamic(mut self, dynamic_states: &[DynamicState]) -> Self {
        self.pipeline_dynamic = dynamic_states.to_vec();
        self
    }

    pub fn with_vertex<V: VertexConvert>(mut self, primitive_topology: PrimitiveTopology) -> Self {
        self.pipeline_vertex_input = (V::binding_desc(), V::attrib_desc());
        self.pipeline_input_assembly = primitive_topology;
        self
    }

    pub fn with_push_constants<P>(mut self) -> Self {
        let size_of = size_of::<P>();
        let push_constants_range = PushConstantRange::builder()
            .offset(0)
            .size(size_of as u32)
            .stage_flags(ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT)
            .build();

        self.pipeline_layout.2 = vec![push_constants_range];
        self
    }

    pub fn with_descriptors(
        mut self,
        descriptors: &[(bool, DescriptorType, ShaderStageFlags, u32)],
    ) -> Self {
        let desc_set_layout_bindings: Vec<DescriptorSetLayoutBinding> = descriptors
            .iter()
            .enumerate()
            .map(|(index, (_, ty, stages, count))| {
                DescriptorSetLayoutBinding::builder()
                    .descriptor_count(*count)
                    .binding(index as u32)
                    .descriptor_type(*ty)
                    .stage_flags(*stages)
                    .build()
            })
            .collect();

        let binding_flags: Vec<DescriptorBindingFlags> = descriptors
            .iter()
            .map(|(dynamic, _, _, _)| match dynamic {
                true => DescriptorBindingFlags::UPDATE_AFTER_BIND,
                false => DescriptorBindingFlags::empty(),
            })
            .collect();

        self.pipeline_layout.0 = binding_flags;
        self.pipeline_layout.1 = desc_set_layout_bindings;
        self
    }

    unsafe fn create_pipeline(
        vk_init: &VkInit,
        create_infos: &[GraphicsPipelineCreateInfo],
    ) -> Result<Pipeline, Error> {
        match vk_init
            .device
            .create_graphics_pipelines(PipelineCache::null(), create_infos, None)
        {
            Ok(pipeline) => Ok(pipeline[0]),
            Err(e) => Err(Error::VkError(e.1)),
        }
    }
}

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
            min_depth: -1.0,
            max_depth: 1.0,
        }
    }
}

#[derive(Default)]
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

impl VertexConvert for () {
    fn binding_desc() -> Vec<VertexInputBindingDescription> {
        vec![]
    }

    fn attrib_desc() -> Vec<VertexInputAttributeDescription> {
        vec![]
    }
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
