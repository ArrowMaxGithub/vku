use anyhow::Result;
use log::error;
use nalgebra_glm::Mat4;
use std::path::Path;
use vku::ash::vk::*;
use vku::*;

mod egui_renderer;
use egui::{ClippedPrimitive, TexturesDelta};
use egui_renderer::EguiRenderer;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

mod push_constants;
mod vertex;

pub(crate) struct Graphics {
    vk_init: VkInit,
    setup_fence: Fence,
    setup_cmd_pool: CommandPool,
    setup_cmd_buffer: CommandBuffer,
    graphics_cmd_pool: CommandPool,
    graphics_cmd_buffer: CommandBuffer,
    in_flight_fence: Fence,
    image_acquired_semaphore: Semaphore,
    render_complete_semaphore: Semaphore,
    egui_renderer: EguiRenderer,
}

impl Graphics {
    pub(crate) fn new(window: &Window) -> Result<Self> {
        vku::compile_all_shaders(
            Path::new("../../../assets/egui/shaders/original"),
            Path::new("../../../assets/egui/shaders/compiled"),
            cfg!(debug_assertions),
        )?;

        let vk_init_create_info = VkInitCreateInfo::debug_vk_1_3();

        let vk_init = VkInit::new(
            Some(&window.raw_display_handle()),
            Some(&window.raw_window_handle()),
            Some(window.inner_size().into()),
            vk_init_create_info,
        )?;

        let setup_fence = vk_init.create_fence()?;
        vk_init.set_debug_object_name(
            setup_fence.as_raw(),
            ObjectType::FENCE,
            String::from("VKU_Setup_Fence"),
        )?;

        let setup_cmd_pool = vk_init.create_cmd_pool(CmdType::Any)?;
        vk_init.set_debug_object_name(
            setup_cmd_pool.as_raw(),
            ObjectType::COMMAND_POOL,
            format!("VKU_Setup_Cmd_Pool"),
        )?;

        let setup_cmd_buffer = vk_init.create_command_buffers(&setup_cmd_pool, 1)?[0];
        vk_init.set_debug_object_name(
            setup_cmd_buffer.as_raw(),
            ObjectType::COMMAND_BUFFER,
            format!("VKU_Setup_Cmd_Buffer"),
        )?;

        let graphics_cmd_pool = vk_init.create_cmd_pool(CmdType::Any)?;
        vk_init.set_debug_object_name(
            graphics_cmd_pool.as_raw(),
            ObjectType::COMMAND_POOL,
            format!("VKU_Graphics_Cmd_Pool"),
        )?;

        let graphics_cmd_buffer = vk_init.create_command_buffers(&graphics_cmd_pool, 1)?[0];
        vk_init.set_debug_object_name(
            graphics_cmd_buffer.as_raw(),
            ObjectType::COMMAND_BUFFER,
            format!("VKU_Graphics_Cmd_Buffer"),
        )?;

        let in_flight_fence = vk_init.create_fence()?;
        vk_init.set_debug_object_name(
            in_flight_fence.as_raw(),
            ObjectType::FENCE,
            String::from("VKU_In_Flight_Fence"),
        )?;

        let image_acquired_semaphore = vk_init.create_semaphore()?;
        vk_init.set_debug_object_name(
            image_acquired_semaphore.as_raw(),
            ObjectType::SEMAPHORE,
            String::from("VKU_Image_Acquired_Semaphore"),
        )?;

        let render_complete_semaphore = vk_init.create_semaphore()?;
        vk_init.set_debug_object_name(
            render_complete_semaphore.as_raw(),
            ObjectType::SEMAPHORE,
            String::from("VKU_Render_Complete_Semaphore"),
        )?;

        let egui_renderer = EguiRenderer::new(&vk_init, 1)?;

        vk_init.wait_on_fence_and_reset(Some(&setup_fence), &[&setup_cmd_buffer])?;
        vk_init.begin_cmd_buffer(&setup_cmd_buffer)?;

        let mut image_memory_barriers: Vec<ImageMemoryBarrier2> = vec![];

        //transition all swapchain images into PRESENT_SRC_KHR before first usage
        for swapchain_image in &vk_init.head().swapchain_images {
            let swapchain_layout_attachment_barrier = ImageMemoryBarrier2::builder()
                .image(*swapchain_image)
                .src_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
                .dst_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(AccessFlags2::empty())
                .dst_access_mask(AccessFlags2::MEMORY_WRITE)
                .old_layout(ImageLayout::UNDEFINED)
                .new_layout(ImageLayout::PRESENT_SRC_KHR)
                .subresource_range(ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    level_count: 1,
                    layer_count: 1,
                    ..Default::default()
                })
                .build();

            image_memory_barriers.push(swapchain_layout_attachment_barrier);
        }

        vk_init.cmd_pipeline_barrier2(&setup_cmd_buffer, &image_memory_barriers, &[]);

        vk_init.end_and_submit_cmd_buffer(
            &setup_cmd_buffer,
            CmdType::Any,
            &setup_fence,
            &[],
            &[],
            &[],
        )?;

        vk_init.wait_on_fence_and_reset(Some(&setup_fence), &[&setup_cmd_buffer])?;

        Ok(Self {
            vk_init,
            setup_fence,
            setup_cmd_pool,
            setup_cmd_buffer,
            graphics_cmd_pool,
            graphics_cmd_buffer,
            in_flight_fence,
            image_acquired_semaphore,
            render_complete_semaphore,
            egui_renderer,
        })
    }

    pub(crate) fn destroy(&self) -> Result<()> {
        self.vk_init.wait_device_idle()?;
        self.egui_renderer.destroy(&self.vk_init)?;
        self.vk_init.destroy_fence(&self.setup_fence)?;
        self.vk_init.destroy_fence(&self.in_flight_fence)?;
        self.vk_init
            .destroy_semaphore(&self.image_acquired_semaphore)?;
        self.vk_init
            .destroy_semaphore(&self.render_complete_semaphore)?;
        self.vk_init.destroy_cmd_pool(&self.setup_cmd_pool)?;
        self.vk_init.destroy_cmd_pool(&self.graphics_cmd_pool)?;
        self.vk_init.destroy()?;

        Ok(())
    }

    pub(crate) fn update(
        &mut self,
        images_delta: TexturesDelta,
        clipped_primitives: Vec<ClippedPrimitive>,
        ui_to_ndc: Mat4,
    ) -> Result<()> {
        let (swapchain_image_index, swapchain_image, swapchain_image_view, sub_optimal) = self
            .vk_init
            .acquire_next_swapchain_image(self.image_acquired_semaphore)?;

        if sub_optimal {
            error!("sub optimal swapchain");
        }

        self.vk_init
            .wait_on_fence_and_reset(Some(&self.in_flight_fence), &[&self.graphics_cmd_buffer])?;

        self.vk_init.begin_cmd_buffer(&self.graphics_cmd_buffer)?;

        let swapchain_attachment_barrier = ImageMemoryBarrier2::builder()
            .image(swapchain_image)
            .src_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
            .dst_stage_mask(PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags2::empty())
            .dst_access_mask(AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .old_layout(ImageLayout::PRESENT_SRC_KHR)
            .new_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .subresource_range(ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            })
            .build();

        self.vk_init.cmd_pipeline_barrier2(
            &self.graphics_cmd_buffer,
            &[swapchain_attachment_barrier],
            &[],
        );

        self.egui_renderer.input(
            &self.vk_init,
            &self.graphics_cmd_buffer,
            clipped_primitives,
            images_delta,
        )?;

        self.vk_init
            .begin_rendering(&swapchain_image_view, &self.graphics_cmd_buffer);
        self.egui_renderer
            .draw(&self.vk_init, &self.graphics_cmd_buffer, ui_to_ndc)?;
        self.vk_init.end_rendering(&self.graphics_cmd_buffer);

        let swapchain_present_barrier = ImageMemoryBarrier2::builder()
            .image(swapchain_image)
            .src_stage_mask(PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(PipelineStageFlags2::BOTTOM_OF_PIPE)
            .src_access_mask(AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .dst_access_mask(AccessFlags2::empty())
            .old_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .new_layout(ImageLayout::PRESENT_SRC_KHR)
            .subresource_range(ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            })
            .build();

        self.vk_init.cmd_pipeline_barrier2(
            &self.graphics_cmd_buffer,
            &[swapchain_present_barrier],
            &[],
        );

        self.vk_init.end_and_submit_cmd_buffer(
            &self.graphics_cmd_buffer,
            CmdType::Any,
            &self.in_flight_fence,
            &[self.image_acquired_semaphore],
            &[self.render_complete_semaphore],
            &[PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
        )?;

        self.vk_init
            .present(&self.render_complete_semaphore, swapchain_image_index)?;

        Ok(())
    }

    pub(crate) fn on_resize(&mut self, new_size: [u32; 2]) -> Result<()> {
        self.vk_init.recreate_swapchain(new_size, 1)?;

        self.vk_init.begin_cmd_buffer(&self.setup_cmd_buffer)?;

        //transition all swapchain images into PRESENT_SRC_KHR before first usage
        let buffer_barriers2: Vec<BufferMemoryBarrier2> = vec![];
        let mut image_barriers2: Vec<ImageMemoryBarrier2> = vec![];

        for swapchain_image in &self.vk_init.head().swapchain_images {
            let swapchain_layout_attachment_barrier = ImageMemoryBarrier2::builder()
                .image(*swapchain_image)
                .src_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
                .dst_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(AccessFlags2::empty())
                .dst_access_mask(AccessFlags2::MEMORY_WRITE)
                .old_layout(ImageLayout::UNDEFINED)
                .new_layout(ImageLayout::PRESENT_SRC_KHR)
                .subresource_range(ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    level_count: 1,
                    layer_count: 1,
                    ..Default::default()
                })
                .build();

            image_barriers2.push(swapchain_layout_attachment_barrier);
        }

        self.vk_init.cmd_pipeline_barrier2(
            &self.setup_cmd_buffer,
            &image_barriers2,
            &buffer_barriers2,
        );

        self.vk_init.end_and_submit_cmd_buffer(
            &self.setup_cmd_buffer,
            CmdType::Any,
            &self.setup_fence,
            &[],
            &[],
            &[],
        )?;

        self.vk_init
            .wait_on_fence_and_reset(Some(&self.setup_fence), &[&self.setup_cmd_buffer])?;
        Ok(())
    }
}
