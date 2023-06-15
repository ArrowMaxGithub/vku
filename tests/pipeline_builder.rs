#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use ash::vk::*;
    use vku::{BlendMode, DepthInfo, StencilInfo, VKUPipeline, VertexConvert, VkInit};
    use winit::platform::wayland::EventLoopBuilderExtWayland;

    #[repr(C)]
    struct Vertex2D {
        pub pos: [f32; 4],
        pub uv: [f32; 2],
        pub vol: [u8; 4],
    }

    impl VertexConvert for Vertex2D {
        fn binding_desc() -> Vec<VertexInputBindingDescription> {
            vec![VertexInputBindingDescription {
                stride: size_of::<Self>() as u32,
                input_rate: VertexInputRate::VERTEX,
                binding: 0,
            }]
        }

        fn attrib_desc() -> Vec<VertexInputAttributeDescription> {
            vec![
                VertexInputAttributeDescription {
                    binding: 0,
                    location: 0,
                    offset: 0,
                    format: Format::R32G32B32A32_SFLOAT,
                },
                VertexInputAttributeDescription {
                    binding: 0,
                    location: 1,
                    offset: 16,
                    format: Format::R32G32_SFLOAT,
                },
                VertexInputAttributeDescription {
                    binding: 0,
                    location: 2,
                    offset: 24,
                    format: Format::R8G8B8A8_UNORM,
                },
            ]
        }
    }

    #[repr(C)]
    struct Push {
        pub mat_0: [f32; 16],
        pub vec_0: [f32; 4],
        pub vec_1: [f32; 4],
        pub vec_2: [f32; 4],
        pub vec_3: [f32; 4],
    }

    fn default_vk_init() -> VkInit {
        use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
        use vku::VkInitCreateInfo;
        use winit::dpi::LogicalSize;
        use winit::event_loop::{EventLoop, EventLoopBuilder};
        use winit::window::WindowBuilder;

        env_logger::init();
        let event_loop: EventLoop<()> = EventLoopBuilder::default().with_any_thread(true).build();
        let size = [800_u32, 600_u32];
        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize {
                width: size[0],
                height: size[1],
            })
            .build(&event_loop)
            .unwrap();

        let display_h = window.raw_display_handle();
        let window_h = window.raw_window_handle();

        let create_info = VkInitCreateInfo::default();
        VkInit::new(Some(&display_h), Some(&window_h), Some(size), create_info).unwrap()
    }

    #[test]
    fn default_pipeline() {
        let vk_init = default_vk_init();

        let _pipeline = VKUPipeline::builder()
            .with_vertex::<Vertex2D>(PrimitiveTopology::TRIANGLE_LIST)
            .with_tesselation(1)
            .with_viewports_scissors(&[Viewport::default()], &[Rect2D::default()]) // using dynamic viewport/scissor later
            .with_rasterization(PolygonMode::FILL, CullModeFlags::BACK)
            .with_multisample(SampleCountFlags::TYPE_1)
            .with_depthstencil(DepthInfo::enabled_positive_depth(), StencilInfo::default())
            .with_colorblends(&[BlendMode::TraditionalTransparency])
            .with_dynamic(&[DynamicState::VIEWPORT, DynamicState::SCISSOR])
            .with_push_constants::<Push>()
            .with_descriptors(&[(
                DescriptorType::COMBINED_IMAGE_SAMPLER,
                ShaderStageFlags::FRAGMENT,
                1,
            )])
            .push_shader_stage(
                &vk_init.device,
                ShaderStageFlags::VERTEX,
                "./tests/default.vert.spv",
                &[],
            )
            .push_shader_stage(
                &vk_init.device,
                ShaderStageFlags::FRAGMENT,
                "./tests/default.frag.spv",
                &[],
            )
            .with_render_pass(
                &[
                    AttachmentDescription::builder()
                        .format(Format::R8G8B8A8_UNORM)
                        .samples(SampleCountFlags::TYPE_1)
                        .load_op(AttachmentLoadOp::CLEAR)
                        .store_op(AttachmentStoreOp::STORE)
                        .initial_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .final_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .build(),
                    AttachmentDescription::builder()
                        .format(Format::D32_SFLOAT)
                        .samples(SampleCountFlags::TYPE_1)
                        .load_op(AttachmentLoadOp::CLEAR)
                        .store_op(AttachmentStoreOp::STORE)
                        .initial_layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .final_layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                        .build(),
                ],
                &[SubpassDescription::builder()
                    .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[AttachmentReference {
                        attachment: 0,
                        layout: ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }])
                    .depth_stencil_attachment(&AttachmentReference {
                        attachment: 1,
                        layout: ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    })
                    .build()],
                &[],
            )
            .build(&vk_init.device)
            .unwrap();
    }
}
