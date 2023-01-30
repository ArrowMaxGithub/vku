use crate::{errors::ImageLayoutTransitionError, imports::*};

pub fn get_image_layout_transition_barrier2(
    image: &Image,
    src_layout: ImageLayout,
    dst_layout: ImageLayout,
    aspect_flags: ImageAspectFlags,
    src_queue: Option<u32>,
    dst_queue: Option<u32>,
) -> Result<ImageMemoryBarrier2> {
    let (src_access, dst_access, src_stage, dst_stage) = match (src_layout, dst_layout) {
        (ImageLayout::UNDEFINED, ImageLayout::TRANSFER_DST_OPTIMAL) => (
            AccessFlags2::empty(),
            AccessFlags2::TRANSFER_WRITE,
            PipelineStageFlags2::TOP_OF_PIPE,
            PipelineStageFlags2::TRANSFER,
        ),

        (ImageLayout::UNDEFINED, ImageLayout::PRESENT_SRC_KHR) => (
            AccessFlags2::empty(),
            AccessFlags2::COLOR_ATTACHMENT_READ,
            PipelineStageFlags2::TOP_OF_PIPE,
            PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        ),

        (ImageLayout::SHADER_READ_ONLY_OPTIMAL, ImageLayout::TRANSFER_DST_OPTIMAL) => (
            AccessFlags2::SHADER_READ,
            AccessFlags2::TRANSFER_WRITE,
            PipelineStageFlags2::FRAGMENT_SHADER,
            PipelineStageFlags2::TRANSFER,
        ),

        (ImageLayout::SHADER_READ_ONLY_OPTIMAL, ImageLayout::PRESENT_SRC_KHR) => (
            AccessFlags2::SHADER_READ,
            AccessFlags2::COLOR_ATTACHMENT_READ,
            PipelineStageFlags2::FRAGMENT_SHADER,
            PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        ),

        (ImageLayout::TRANSFER_DST_OPTIMAL, ImageLayout::TRANSFER_SRC_OPTIMAL) => (
            AccessFlags2::TRANSFER_WRITE,
            AccessFlags2::TRANSFER_READ,
            PipelineStageFlags2::TRANSFER,
            PipelineStageFlags2::TRANSFER,
        ),

        (ImageLayout::TRANSFER_DST_OPTIMAL, ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            AccessFlags2::TRANSFER_WRITE,
            AccessFlags2::SHADER_READ,
            PipelineStageFlags2::TRANSFER,
            PipelineStageFlags2::FRAGMENT_SHADER,
        ),

        (ImageLayout::TRANSFER_DST_OPTIMAL, ImageLayout::PRESENT_SRC_KHR) => (
            AccessFlags2::TRANSFER_WRITE,
            AccessFlags2::COLOR_ATTACHMENT_READ,
            PipelineStageFlags2::TRANSFER,
            PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        ),

        (ImageLayout::PRESENT_SRC_KHR, ImageLayout::TRANSFER_DST_OPTIMAL) => (
            AccessFlags2::COLOR_ATTACHMENT_READ,
            AccessFlags2::TRANSFER_WRITE,
            PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags2::TRANSFER,
        ),

        (ImageLayout::PRESENT_SRC_KHR, ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            AccessFlags2::COLOR_ATTACHMENT_READ,
            AccessFlags2::SHADER_READ,
            PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags2::FRAGMENT_SHADER,
        ),

        (_, _) => {
            trace!("Tried to transition from {src_layout:?} to {dst_layout:?}");
            return Err(anyhow!(
                ImageLayoutTransitionError::UnsupportedImageLayoutTransition
            ));
        }
    };

    let barrier = ImageMemoryBarrier2::builder()
        .image(*image)
        .src_stage_mask(src_stage)
        .dst_stage_mask(dst_stage)
        .src_access_mask(src_access)
        .dst_access_mask(dst_access)
        .src_queue_family_index(src_queue.unwrap_or(0))
        .src_queue_family_index(dst_queue.unwrap_or(0))
        .old_layout(src_layout)
        .new_layout(dst_layout)
        .subresource_range(ImageSubresourceRange {
            aspect_mask: aspect_flags,
            level_count: 1,
            layer_count: 1,
            ..Default::default()
        })
        .build();

    Ok(barrier)
}
