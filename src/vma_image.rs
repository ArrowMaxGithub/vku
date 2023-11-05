use gpu_allocator::vulkan::AllocationScheme;

use crate::{image_layout_transitions, imports::*, vma_buffer::VMABuffer, VkInit};

/// VMA-allocated image, image information, image view, allocation and allocation information.
///
/// Includes a host-visible staging buffer
pub struct VMAImage {
    pub staging_buffer: VMABuffer,
    pub image: Image,
    pub extent: Extent3D,
    pub aspect_flags: ImageAspectFlags,
    pub image_view: ImageView,
    pub allocation: Allocation,
    pub current_layout: ImageLayout,
}

impl VMAImage {
    fn new(
        device: &Device,
        allocator: &mut Allocator,
        image_info: ImageCreateInfo,
        aspect_flags: ImageAspectFlags,
        mut allocation_create_info: AllocationCreateDesc,
        staging_buffer: VMABuffer,
    ) -> Result<Self, Error> {
        let (image, allocation) = unsafe { 
            let image = device.create_image(&image_info, None)?;
            let req = device.get_image_memory_requirements(image);
            allocation_create_info.requirements = req;
            let alloc = allocator.allocate(&allocation_create_info)?;
            device.bind_image_memory(image, alloc.memory(), 0)?;
            (image, alloc)
        };

        let image_view_create_info = ImageViewCreateInfo {
            view_type: ImageViewType::TYPE_2D,
            format: image_info.format,
            components: ComponentMapping {
                r: ComponentSwizzle::R,
                g: ComponentSwizzle::G,
                b: ComponentSwizzle::B,
                a: ComponentSwizzle::A,
            },
            subresource_range: ImageSubresourceRange {
                aspect_mask: aspect_flags,
                level_count: 1,
                layer_count: 1,
                base_array_layer: 0,
                base_mip_level: 0,
            },
            image,
            ..Default::default()
        };

        let image_view = unsafe { device.create_image_view(&image_view_create_info, None) }?;
        let extent = image_info.extent;

        Ok(Self {
            image,
            extent,
            aspect_flags,
            image_view,
            allocation,
            staging_buffer,
            current_layout: ImageLayout::UNDEFINED,
        })
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) -> Result<(), Error> {
        unsafe {
            self.staging_buffer.destroy(device, allocator)?;
            device.destroy_image(self.image, None);
            device.destroy_image_view(self.image_view, None);
            let alloc = std::mem::replace(&mut self.allocation, Allocation::default());
            allocator.free(alloc)?;
        }
        Ok(())
    }

    pub fn set_debug_object_name(&self, vk_init: &VkInit, base_name: String) -> Result<(), Error> {
        vk_init.set_debug_object_name(
            self.image.as_raw(),
            ObjectType::IMAGE,
            format!("{base_name}_Image"),
        )?;
        vk_init.set_debug_object_name(
            unsafe { self.allocation.memory().as_raw() },
            ObjectType::DEVICE_MEMORY,
            format!("{base_name}_Memory"),
        )?;
        vk_init.set_debug_object_name(
            self.image_view.as_raw(),
            ObjectType::IMAGE_VIEW,
            format!("{base_name}_Image_View"),
        )?;
        self.staging_buffer
            .set_debug_object_name(vk_init, format!("{base_name}_Staging_Buffer"))?;
        Ok(())
    }

    /// Creates an empty image with specified format for transfer and sample operations.
    /// ```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info).unwrap();
    ///
    /// let extent = Extent3D{width: 100, height: 100, depth: 1};
    /// let format = Format::R8G8B8A8_UNORM;
    /// let format_bytes = 4;
    /// let aspect_flags = ImageAspectFlags::COLOR;
    ///
    /// let image = init.create_empty_image(extent, format, format_bytes, aspect_flags).unwrap();

    pub fn create_empty_image(
        device: &Device,
        allocator: &mut Allocator,
        extent: Extent3D,
        format: Format,
        sizeof: usize,
        aspect_mask: ImageAspectFlags,
    ) -> Result<VMAImage, Error> {
        let image_info = ImageCreateInfo {
            image_type: ImageType::TYPE_2D,
            format,
            extent,
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCountFlags::TYPE_1,
            tiling: ImageTiling::OPTIMAL,
            usage: ImageUsageFlags::SAMPLED
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::TRANSFER_SRC,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let allocation_info = AllocationCreateDesc{
            name: "Local_Image_Memory",
            requirements: MemoryRequirements::default(),
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };

        let staging_buffer = VMABuffer::create_cpu_to_gpu_buffer(
            device,
            allocator,
            (extent.width * extent.height * extent.depth) as usize * sizeof,
            BufferUsageFlags::TRANSFER_SRC,
        )?;

        Self::new(
            device,
            allocator,
            image_info,
            aspect_mask,
            allocation_info,
            staging_buffer,
        )
    }

    pub fn create_depth_image(
        device: &Device,
        allocator: &mut Allocator,
        extent: Extent3D,
        format: Format,
        sizeof: usize,
    ) -> Result<VMAImage, Error> {
        let image_info = ImageCreateInfo {
            image_type: ImageType::TYPE_2D,
            format,
            extent,
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCountFlags::TYPE_1,
            tiling: ImageTiling::OPTIMAL,
            usage: ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let allocation_info = AllocationCreateDesc{
            name: "Local_Image_Memory",
            requirements: MemoryRequirements::default(),
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };

        let staging_buffer = VMABuffer::create_cpu_to_gpu_buffer(
            device,
            allocator,
            (extent.width * extent.height * extent.depth) as usize * sizeof,
            BufferUsageFlags::TRANSFER_SRC,
        )?;

        Self::new(
            device,
            allocator,
            image_info,
            ImageAspectFlags::DEPTH,
            allocation_info,
            staging_buffer,
        )
    }

    pub fn create_render_image(
        device: &Device,
        allocator: &mut Allocator,
        extent: Extent3D,
        format: Format,
        sizeof: usize,
    ) -> Result<VMAImage, Error> {
        let image_info = ImageCreateInfo {
            image_type: ImageType::TYPE_2D,
            format,
            extent,
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCountFlags::TYPE_1,
            tiling: ImageTiling::OPTIMAL,
            usage: ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::SAMPLED,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let allocation_info = AllocationCreateDesc{
            name: "Local_Image_Memory",
            requirements: MemoryRequirements::default(),
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };

        let staging_buffer = VMABuffer::create_cpu_to_gpu_buffer(
            device,
            allocator,
            (extent.width * extent.height * extent.depth) as usize * sizeof,
            BufferUsageFlags::TRANSFER_SRC,
        )?;

        Self::new(
            device,
            allocator,
            image_info,
            ImageAspectFlags::COLOR,
            allocation_info,
            staging_buffer,
        )
    }

    /// Sets data for the staging buffer.
    ///
    /// ```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let create_info = VkInitCreateInfo::default();
    /// # let mut init = VkInit::new(Some(&window), Some(size), create_info).unwrap();
    /// let extent = Extent3D{width: 100, height: 100, depth: 1};
    /// let format = Format::R8G8B8A8_UNORM;
    /// let format_bytes = 4;
    /// let aspect_flags = ImageAspectFlags::COLOR;
    /// let image = init.create_empty_image(extent, format, format_bytes, aspect_flags).unwrap();
    /// let data = [42_u32; 100*100];
    ///
    /// image.set_staging_data(&data).unwrap();

    pub fn set_staging_data<T>(&self, data: &[T]) -> Result<(), Error>
    where
        T: Sized + Copy + Clone,
    {
        self.staging_buffer.set_data(0, data)
    }

    /// Enqueues cmd_copy_buffer_to_image from staging buffer to image.
    ///
    /// No barriers are issued. Image needs to be in ```ImageLayout::TRANSFER_DST_OPTIMAL```.
    /// ```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let create_info = VkInitCreateInfo::default();
    /// # let mut init = VkInit::new(Some(&window), Some(size), create_info).unwrap();
    /// # let setup_cmd_buffer_pool =
    /// #     init.create_cmd_pool(CmdType::Any).unwrap();
    /// # let setup_cmd_buffer =
    /// #     init.create_command_buffers(&setup_cmd_buffer_pool, 1).unwrap()[0];
    /// # let setup_fence = init.create_fence().unwrap();
    /// # init.begin_cmd_buffer(&setup_cmd_buffer).unwrap();
    /// # let extent = Extent3D{width: 100, height: 100, depth: 1};
    /// # let format = Format::R8G8B8A8_UNORM;
    /// # let format_bytes = 4;
    /// # let aspect_flags = ImageAspectFlags::COLOR;
    /// let mut image = init.create_empty_image(extent, format, format_bytes, aspect_flags).unwrap();
    ///
    /// let image_barrier = image.get_image_layout_transition_barrier2(
    ///     ImageLayout::TRANSFER_DST_OPTIMAL,
    ///     None,
    ///     None,
    ///     ).unwrap();
    ///
    /// init.cmd_pipeline_barrier2(
    ///     &setup_cmd_buffer,
    ///     &[image_barrier],
    ///     &[]
    ///     );
    ///
    /// let data = [42_u32; 100*100];
    /// image.set_staging_data(&data).unwrap();
    /// image.enque_copy_from_staging_buffer_to_image(&init.device, &setup_cmd_buffer);
    ///
    /// init.end_and_submit_cmd_buffer(
    ///     &setup_cmd_buffer,
    ///     CmdType::Any,
    ///     &setup_fence,
    ///     &[],
    ///     &[],
    ///     &[],    
    /// ).unwrap();
    /// ```

    pub fn enque_copy_from_staging_buffer_to_image(
        &self,
        device: &Device,
        cmd_buffer: &CommandBuffer,
    ) {
        unsafe {
            let buffer_copy_regions = BufferImageCopy::builder()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    ImageSubresourceLayers::builder()
                        .aspect_mask(ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .image_extent(Extent3D {
                    width: self.extent.width,
                    height: self.extent.height,
                    depth: 1,
                })
                .build();

            device.cmd_copy_buffer_to_image(
                *cmd_buffer,
                self.staging_buffer.buffer,
                self.image,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_copy_regions],
            );
        }
    }

    /// Gets appropriate ```ImageMemoryBarrier2``` from current layout to ```dst_layout``` for this image.
    ///
    /// Current layout is set to ```dst_layout``` after returning this barrier.
    ///
    /// **Defaults**:
    /// - src_queue: 0
    /// - dst_queue: 0

    pub fn get_image_layout_transition_barrier2(
        &mut self,
        dst_layout: ImageLayout,
        src_queue: Option<u32>,
        dst_queue: Option<u32>,
    ) -> Result<ImageMemoryBarrier2, Error> {
        let barrier = image_layout_transitions::get_image_layout_transition_barrier2(
            &self.image,
            self.current_layout,
            dst_layout,
            self.aspect_flags,
            src_queue,
            dst_queue,
        );
        self.current_layout = dst_layout;

        barrier
    }
}

impl VkInit {
    /// Shortcut - see [VMAImage](VMAImage::create_empty_image) for example.

    pub fn create_empty_image(
        &mut self,
        extent: Extent3D,
        format: Format,
        format_sizeof: usize,
        aspect_mask: ImageAspectFlags,
    ) -> Result<VMAImage, Error> {
        VMAImage::create_empty_image(&self.device, &mut self.allocator, extent, format, format_sizeof, aspect_mask)
    }
}
