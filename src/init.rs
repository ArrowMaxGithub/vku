use std::mem::ManuallyDrop;

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vma::AllocatorCreateInfo;

use crate::create_info::VkInitCreateInfo;
use crate::{imports::*, VMAImage};

/// Wrapper around 'static' vulkan objects (instance, device etc.), optional head (surface, swapchain etc.), and utility functions for ease of use.
///
/// Handles initialization and destruction of Vulkan objects and offers utility functions for:
/// - GLSL shader compilation with #include directive
/// - Swapchain recreation and resizing
/// - Optionally exposed dedicated compute and transfer queues
/// - Shortcuts for present and submit operations
pub struct VkInit {
    /// [VMA](vma::Allocator) allocator
    pub allocator: ManuallyDrop<Allocator>,
    pub entry: Entry,
    pub instance: Instance,
    /// Only created with enabled validation
    pub debug_loader: Option<DebugUtils>,
    /// Only created with enabled validation   
    pub debug_messenger: Option<DebugUtilsMessengerEXT>,
    pub physical_device: PhysicalDevice,
    pub device: Device,
    /// Unfified queue is guarenteed to be present per vulkan spec and can handle any command
    pub unified_queue: Queue,
    /// Optionally exposed
    pub transfer_queue: Option<Queue>,
    /// Optionally exposed
    pub compute_queue: Option<Queue>,
    pub physical_device_info: PhysicalDeviceInfo,
    pub head: Option<Head>,
    pub create_info: VkInitCreateInfo,
}

/// Wrapper around presentation resources.
/// - Depth image
pub struct Head {
    pub surface_loader: Surface,
    pub surface: SurfaceKHR,
    pub swapchain_loader: Swapchain,
    pub swapchain: SwapchainKHR,
    pub swapchain_images: Vec<Image>,
    pub swapchain_image_views: Vec<ImageView>,
    pub clear_color_value: ClearColorValue,
    pub clear_depth_stencil_value: ClearDepthStencilValue,
    pub surface_info: SurfaceInfo,
    pub depth_format: Format,
    pub depth_format_sizeof: usize,
    pub depth_image: VMAImage,
}

/// Abstraction over queue capability and command types since dedicated queues may not be available.
///
/// [get_queue](VkInit::get_queue) will fallback to the guarenteed unified queue if necessary.
///
///  ```
/// # extern crate winit;
/// # use vku::*;
/// # use ash::vk::*;
/// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
/// # let size = [800_u32, 600_u32];
/// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
/// # let create_info = VkInitCreateInfo::default();
/// let init = VkInit::new(Some(&window), Some(size), create_info).unwrap();
///
/// let (compute_queue, compute_queue_family_index) = init.get_queue(CmdType::Compute);
pub enum CmdType {
    /// Graphics | Transfer | Compute
    Any,
    Graphics,
    Transfer,
    Compute,
}

/// Return info about the selected physical device and its capabilities.
///
/// The unified queue is guarenteed to be present and can process any command.
///
/// Dedicated transfer and compute queues are optional.
pub struct PhysicalDeviceInfo {
    pub name: String,
    pub unified_queue_family_index: u32,
    pub transfer_queue_family_index: Option<u32>,
    pub compute_queue_family_index: Option<u32>,
    pub features: PhysicalDeviceFeatures,
    pub memory_props: PhysicalDeviceMemoryProperties,
    pub limits: PhysicalDeviceLimits,
}

/// Return info about the created surface and its capabilities.
pub struct SurfaceInfo {
    pub min_extent: Extent2D,
    pub max_extent: Extent2D,
    pub current_extent: Extent2D,
    pub image_count: u32,
    pub present_mode: PresentModeKHR,
    pub color_format: SurfaceFormatKHR,
    pub pre_transform: SurfaceTransformFlagsKHR,
}

impl VkInit {
    /// Creates a new VkInit Vulkan wrapper from raw display and window handles.
    ///
    /// All creation parameters are provided via [VkInitCreateInfo].
    /// Required platform-specific extensions for windowing are included.
    ///
    /// Returns VkInit which holds all 'static' vulkan objects and information about the initialization e.g. physical device capabilities and queue family indices.
    /// Will get 1 unified queue guarenteed and 1 dedicated queue each for compute and transfer operations, if available.
    ///
    /// Example initialization for winit:
    ///```
    /// extern crate winit;
    /// use winit::window::WindowBuilder;
    /// use winit::event_loop::{EventLoop, EventLoopBuilder};
    /// use winit::dpi::LogicalSize;
    /// use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
    /// use vku::{VkInitCreateInfo, VkInit};
    ///
    /// let event_loop: EventLoop<()> = EventLoopBuilder::default().build();
    /// let size = [800_u32, 600_u32];
    /// let window = WindowBuilder::new()
    ///     .with_inner_size(LogicalSize{width: size[0], height: size[1]})
    ///     .build(&event_loop).unwrap();
    /// let create_info = VkInitCreateInfo::default();
    ///
    /// let init = VkInit::new(Some(&window), Some(size), create_info).unwrap();
    /// ```

    pub fn new<T: HasRawDisplayHandle + HasRawWindowHandle>(
        raw_window_handles: Option<&T>,
        window_size: Option<[u32; 2]>,
        create_info: VkInitCreateInfo,
    ) -> Result<Self, Error> {
        unsafe {
            let (display_h, window_h) = match raw_window_handles {
                Some(handles) => (
                    Some(handles.raw_display_handle()),
                    Some(handles.raw_window_handle()),
                ),
                None => (None, None),
            };
            #[cfg(feature = "linked")]
            let entry = ash::Entry::linked();

            #[cfg(not(feature = "linked"))]
            let entry = ash::Entry::load()?;

            let (instance, debug_loader, debug_messenger) =
                Self::create_instance_and_debug(&entry, display_h, &create_info)?;
            let (physical_device, physical_device_info) =
                Self::create_physical_device(&instance, &create_info)?;
            let device = Self::create_device(
                &instance,
                &physical_device,
                &physical_device_info,
                &create_info,
            )?;
            let allocator = Self::create_allocator(&instance, &physical_device, &device)?;
            let (unified_queue, transfer_queue, compute_queue) =
                Self::create_queues(&device, &physical_device_info)?;

            let head = if let (Some(display_handle), Some(window_handle), Some(window_size)) =
                (display_h, window_h, window_size)
            {
                Some(Self::create_head(
                    &device,
                    &allocator,
                    &entry,
                    &instance,
                    display_handle,
                    window_handle,
                    window_size,
                    &physical_device,
                    &create_info,
                )?)
            } else {
                None
            };

            //TODO: Why is RenderDoc crashing when Instance debug name is set?
            //TODO: Why are swapchain, swapchain_images, and swapchain_image_views names not set in RenderDoc?
            if let Some(dbg) = &debug_loader {
                Self::set_debug_object_name_static(
                    dbg,
                    &device,
                    physical_device.as_raw(),
                    ObjectType::PHYSICAL_DEVICE,
                    "VKU_Physical_Device".to_string(),
                )?;
                Self::set_debug_object_name_static(
                    dbg,
                    &device,
                    device.handle().as_raw(),
                    ObjectType::DEVICE,
                    "VKU_Device".to_string(),
                )?;
                Self::set_debug_object_name_static(
                    dbg,
                    &device,
                    unified_queue.as_raw(),
                    ObjectType::QUEUE,
                    "VKU_Unified_Queue".to_string(),
                )?;
                if let Some(transfer_queue) = transfer_queue {
                    Self::set_debug_object_name_static(
                        dbg,
                        &device,
                        transfer_queue.as_raw(),
                        ObjectType::QUEUE,
                        "VKU_Transfer_Queue".to_string(),
                    )?;
                }

                if let Some(compute_queue) = compute_queue {
                    Self::set_debug_object_name_static(
                        dbg,
                        &device,
                        compute_queue.as_raw(),
                        ObjectType::QUEUE,
                        "VKU_Compute_Queue".to_string(),
                    )?;
                }

                if let Some(head) = &head {
                    Self::set_debug_object_name_static(
                        dbg,
                        &device,
                        head.swapchain.as_raw(),
                        ObjectType::SWAPCHAIN_KHR,
                        "VKU_SwapchainKHR".to_string(),
                    )?;

                    Self::set_debug_object_name_static(
                        dbg,
                        &device,
                        head.depth_image.image.as_raw(),
                        ObjectType::IMAGE,
                        "VKU_DepthImage".to_string(),
                    )?;
                    Self::set_debug_object_name_static(
                        dbg,
                        &device,
                        head.depth_image.image_view.as_raw(),
                        ObjectType::IMAGE_VIEW,
                        "VKU_DepthImage_View".to_string(),
                    )?;
                    Self::set_debug_object_name_static(
                        dbg,
                        &device,
                        head.depth_image.allocation_info.device_memory.as_raw(),
                        ObjectType::DEVICE_MEMORY,
                        "VKU_DepthImage_Memory".to_string(),
                    )?;

                    for (i, image) in head.swapchain_images.iter().enumerate() {
                        Self::set_debug_object_name_static(
                            dbg,
                            &device,
                            image.as_raw(),
                            ObjectType::IMAGE,
                            format!("VKU_Swapchain_Image_{i}"),
                        )?;
                    }

                    for (i, image_view) in head.swapchain_image_views.iter().enumerate() {
                        Self::set_debug_object_name_static(
                            dbg,
                            &device,
                            image_view.as_raw(),
                            ObjectType::IMAGE_VIEW,
                            format!("VKU_Swapchain_Image_View_{i}"),
                        )?;
                    }
                }
            }

            Ok(Self {
                allocator: ManuallyDrop::new(allocator),
                entry,
                instance,
                debug_loader,
                debug_messenger,
                physical_device,
                device,
                unified_queue,
                compute_queue,
                transfer_queue,
                physical_device_info,
                head,
                create_info,
            })
        }
    }

    pub fn destroy(&mut self) -> Result<(), Error> {
        unsafe {
            self.device.device_wait_idle()?;
            if let Some(head) = &mut self.head {
                for image_view in &head.swapchain_image_views {
                    self.device.destroy_image_view(*image_view, None);
                }
                head.swapchain_loader
                    .destroy_swapchain(head.swapchain, None);
                head.surface_loader.destroy_surface(head.surface, None);
                head.depth_image.destroy(&self.device, &self.allocator)?;
            }
            if let Some(dbg_loader) = &self.debug_loader {
                dbg_loader.destroy_debug_utils_messenger(self.debug_messenger.unwrap(), None);
            }

            ManuallyDrop::drop(&mut self.allocator);

            self.device.destroy_device(None);
            // self.instance.destroy_instance(None); seg faults for no apparant reason
        }

        Ok(())
    }

    pub fn head(&self) -> &Head {
        self.head.as_ref().expect("called head() on headless vku")
    }

    pub fn head_mut(&mut self) -> &mut Head {
        self.head
            .as_mut()
            .expect("called head_mut() on headless vku")
    }

    pub fn set_debug_object_name(
        &self,
        obj_handle: u64,
        obj_type: ObjectType,
        name: String,
    ) -> Result<(), Error> {
        if let Some(dbg) = &self.debug_loader {
            let c_name = CString::new(name)?;

            let name_info = DebugUtilsObjectNameInfoEXT::builder()
                .object_name(&c_name)
                .object_handle(obj_handle)
                .object_type(obj_type)
                .build();

            unsafe { dbg.set_debug_utils_object_name(self.device.handle(), &name_info)? };
        }
        Ok(())
    }

    pub fn insert_debug_label(&self, cmd_buffer: &CommandBuffer, name: &str) -> Result<(), Error> {
        if let Some(dbg) = &self.debug_loader {
            let label_info = DebugUtilsLabelEXT::builder()
                .label_name(unsafe { CStr::from_ptr(name.as_ptr() as *const i8) })
                .build();

            unsafe { dbg.cmd_insert_debug_utils_label(*cmd_buffer, &label_info) };
        }
        Ok(())
    }

    pub fn begin_debug_label(&self, cmd_buffer: &CommandBuffer, name: &str) -> Result<(), Error> {
        if let Some(dbg) = &self.debug_loader {
            let label_info = DebugUtilsLabelEXT::builder()
                .label_name(unsafe { CStr::from_ptr(name.as_ptr() as *const i8) })
                .build();

            unsafe { dbg.cmd_begin_debug_utils_label(*cmd_buffer, &label_info) };
        }
        Ok(())
    }

    pub fn end_debug_label(&self, cmd_buffer: &CommandBuffer) -> Result<(), Error> {
        if let Some(dbg) = &self.debug_loader {
            unsafe { dbg.cmd_end_debug_utils_label(*cmd_buffer) };
        }
        Ok(())
    }

    pub fn create_cmd_pool(&self, cmd_type: CmdType) -> Result<CommandPool, Error> {
        let (_, queue_family_index) = self.get_queue(cmd_type);
        let create_info = CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe { self.device.create_command_pool(&create_info, None)? };
        Ok(command_pool)
    }

    pub fn create_command_buffers(
        &self,
        pool: &CommandPool,
        count: u32,
    ) -> Result<Vec<CommandBuffer>, Error> {
        let create_info = CommandBufferAllocateInfo::builder()
            .command_pool(*pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        let alloc = unsafe { self.device.allocate_command_buffers(&create_info)? };
        Ok(alloc)
    }

    /// Creates a signaled fence.
    pub fn create_fence(&self) -> Result<Fence, Error> {
        let create_info = FenceCreateInfo::builder().flags(FenceCreateFlags::SIGNALED);
        let fence = unsafe { self.device.create_fence(&create_info, None)? };

        Ok(fence)
    }

    /// Creates a Vec of signaled fence.
    pub fn create_fences(&self, count: usize) -> Result<Vec<Fence>, Error> {
        let mut fences = Vec::new();
        for _ in 0..count {
            let create_info = FenceCreateInfo::builder().flags(FenceCreateFlags::SIGNALED);
            let fence = unsafe { self.device.create_fence(&create_info, None)? };
            fences.push(fence);
        }

        Ok(fences)
    }

    pub fn destroy_fence(&self, fence: &Fence) -> Result<(), Error> {
        unsafe {
            self.device.destroy_fence(*fence, None);
        }

        Ok(())
    }

    pub fn create_semaphore(&self) -> Result<Semaphore, Error> {
        let create_info = SemaphoreCreateInfo::default();
        let semaphore = unsafe { self.device.create_semaphore(&create_info, None)? };

        Ok(semaphore)
    }

    pub fn create_semaphores(&self, count: usize) -> Result<Vec<Semaphore>, Error> {
        let mut semaphores = Vec::new();
        for _ in 0..count {
            let create_info = SemaphoreCreateInfo::default();
            let semaphore = unsafe { self.device.create_semaphore(&create_info, None)? };
            semaphores.push(semaphore);
        }

        Ok(semaphores)
    }

    pub fn destroy_semaphore(&self, semaphore: &Semaphore) -> Result<(), Error> {
        unsafe {
            self.device.destroy_semaphore(*semaphore, None);
        }

        Ok(())
    }

    pub fn destroy_cmd_pool(&self, pool: &CommandPool) -> Result<(), Error> {
        unsafe {
            self.device.destroy_command_pool(*pool, None);
        }

        Ok(())
    }

    /// Acquires next image and signals sempahore ```acquire_img_semaphore```.
    pub fn acquire_next_swapchain_image(
        &self,
        acquire_img_semaphore: Semaphore,
    ) -> Result<(usize, Image, ImageView, bool), Error> {
        let head = self.head.as_ref().unwrap();
        let (index, sub_optimal) = unsafe {
            head.swapchain_loader.acquire_next_image(
                head.swapchain,
                1000 * 1000 * 1000, //One second
                acquire_img_semaphore,
                Fence::null(),
            )?
        };
        let swapchain_image = head.swapchain_images[index as usize];
        let swapchain_image_view = head.swapchain_image_views[index as usize];
        Ok((
            index as usize,
            swapchain_image,
            swapchain_image_view,
            sub_optimal,
        ))
    }

    pub fn begin_cmd_buffer(&self, cmd_buffer: &CommandBuffer) -> Result<(), Error> {
        let cmd_buffer_begin_info =
            CommandBufferBeginInfo::builder().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(*cmd_buffer, &cmd_buffer_begin_info)?
        };

        Ok(())
    }

    pub fn begin_rendering(&self, swapchain_image_view: &ImageView, cmd_buffer: &CommandBuffer) {
        let head = self.head.as_ref().unwrap();

        let clear_color_value = ClearValue {
            color: head.clear_color_value,
        };
        let clear_depth_stencil_value = ClearValue {
            depth_stencil: head.clear_depth_stencil_value,
        };

        let render_area = Rect2D::builder()
            .offset(Offset2D { x: 0, y: 0 })
            .extent(head.surface_info.current_extent);

        let color_attachment_info = [RenderingAttachmentInfo::builder()
            .image_view(*swapchain_image_view)
            .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .clear_value(clear_color_value)
            .build()];

        let depth_attachment_info = RenderingAttachmentInfo::builder()
            .image_view(head.depth_image.image_view)
            .image_layout(ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .clear_value(clear_depth_stencil_value)
            .build();

        let rendering_begin_info = RenderingInfo::builder()
            .render_area(*render_area)
            .layer_count(1)
            .color_attachments(&color_attachment_info)
            .depth_attachment(&depth_attachment_info);

        unsafe {
            self.device
                .cmd_begin_rendering(*cmd_buffer, &rendering_begin_info);
        }
    }

    pub fn end_rendering(&self, cmd_buffer: &CommandBuffer) {
        unsafe {
            self.device.cmd_end_rendering(*cmd_buffer);
        }
    }

    pub fn end_and_submit_cmd_buffer(
        &self,
        cmd_buffer: &CommandBuffer,
        cmd_type: CmdType,
        fence: &Fence,
        wait_sem: &[Semaphore],
        signal_sem: &[Semaphore],
        wait_dst_flags: &[PipelineStageFlags],
    ) -> Result<(), Error> {
        unsafe { self.device.end_command_buffer(*cmd_buffer)? };

        let cmd_buffers = [*cmd_buffer];
        let mut submit_info = SubmitInfo::builder()
            .command_buffers(&cmd_buffers)
            .wait_dst_stage_mask(wait_dst_flags)
            .signal_semaphores(signal_sem)
            .wait_semaphores(wait_sem)
            .build();

        if wait_sem.is_empty() {
            submit_info.wait_semaphore_count = 0;
            submit_info.p_wait_semaphores = std::ptr::null();
        }

        if signal_sem.is_empty() {
            submit_info.signal_semaphore_count = 0;
            submit_info.p_signal_semaphores = std::ptr::null();
        }

        let (queue, _) = self.get_queue(cmd_type);
        unsafe { self.device.queue_submit(queue, &[submit_info], *fence)? };

        Ok(())
    }

    pub fn wait_on_fence_and_reset(
        &self,
        fence: Option<&Fence>,
        cmd_buffers: &[&CommandBuffer],
    ) -> Result<(), Error> {
        unsafe {
            if let Some(fence) = fence {
                self.device.wait_for_fences(&[*fence], true, u64::MAX)?;
                self.device.reset_fences(&[*fence])?;
            }
            for cmd_buffer in cmd_buffers {
                self.device.reset_command_buffer(
                    **cmd_buffer,
                    CommandBufferResetFlags::RELEASE_RESOURCES,
                )?;
            }
        }
        Ok(())
    }

    pub fn cmd_pipeline_barrier2(
        &self,
        cmd_buffer: &CommandBuffer,
        image_memory_barriers: &[ImageMemoryBarrier2],
        buffer_memory_barriers: &[BufferMemoryBarrier2],
    ) {
        let dependency_info = DependencyInfo::builder()
            .image_memory_barriers(image_memory_barriers)
            .buffer_memory_barriers(buffer_memory_barriers)
            .dependency_flags(DependencyFlags::empty())
            .build();

        unsafe {
            self.device
                .cmd_pipeline_barrier2(*cmd_buffer, &dependency_info);
        }
    }

    pub fn present(
        &self,
        rendering_complete_semaphore: &Semaphore,
        frame: usize,
    ) -> Result<(), Error> {
        let head = self.head.as_ref().unwrap();
        let swapchains = [head.swapchain];
        let image_indices = [frame as u32];
        let wait_sems = [*rendering_complete_semaphore];
        let present_info = ash::vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_sems)
            .swapchains(&swapchains)
            .image_indices(&image_indices)
            .build();

        unsafe {
            head.swapchain_loader
                .queue_present(self.unified_queue, &present_info)?;
        }

        Ok(())
    }

    pub fn wait_device_idle(&self) -> Result<(), Error> {
        unsafe {
            self.device.device_wait_idle()?;
        }

        Ok(())
    }

    /// Gets the queue and queue family index for the given [CmdType].
    ///
    /// If there is e.g. no dedicated compute queue, this will fallback to the guarenteed unified queue.

    pub fn get_queue(&self, cmd_type: CmdType) -> (Queue, u32) {
        match cmd_type {
            CmdType::Any => (
                self.unified_queue,
                self.physical_device_info.unified_queue_family_index,
            ),
            CmdType::Graphics => (
                self.unified_queue,
                self.physical_device_info.unified_queue_family_index,
            ),
            CmdType::Transfer => {
                //TODO: Implement as if-let chains once stabilized
                if self
                    .physical_device_info
                    .transfer_queue_family_index
                    .is_some()
                    && self.transfer_queue.is_some()
                {
                    (
                        self.transfer_queue.unwrap(),
                        self.physical_device_info
                            .transfer_queue_family_index
                            .unwrap(),
                    )
                } else {
                    (
                        self.unified_queue,
                        self.physical_device_info.unified_queue_family_index,
                    )
                }
            }
            CmdType::Compute => {
                //TODO: Implement as if-let chains once stabilized
                if self
                    .physical_device_info
                    .compute_queue_family_index
                    .is_some()
                    && self.compute_queue.is_some()
                {
                    (
                        self.compute_queue.unwrap(),
                        self.physical_device_info
                            .compute_queue_family_index
                            .unwrap(),
                    )
                } else {
                    (
                        self.unified_queue,
                        self.physical_device_info.unified_queue_family_index,
                    )
                }
            }
        }
    }

    fn set_debug_object_name_static(
        dbg: &DebugUtils,
        device: &Device,
        obj_handle: u64,
        obj_type: ObjectType,
        name: String,
    ) -> Result<(), Error> {
        let c_name = CString::new(name)?;
        let name_info = DebugUtilsObjectNameInfoEXT::builder()
            .object_name(&c_name)
            .object_handle(obj_handle)
            .object_type(obj_type)
            .build();

        unsafe { dbg.set_debug_utils_object_name(device.handle(), &name_info)? };
        Ok(())
    }

    pub(crate) unsafe fn create_instance_and_debug(
        entry: &Entry,
        display_handle: Option<RawDisplayHandle>,
        create_info: &VkInitCreateInfo,
    ) -> Result<(Instance, Option<DebugUtils>, Option<DebugUtilsMessengerEXT>), Error> {
        let app_info = ApplicationInfo::builder()
            .application_name(CStr::from_ptr(create_info.app_name.as_ptr() as *const i8))
            .engine_name(CStr::from_ptr(create_info.engine_name.as_ptr() as *const i8))
            .application_version(create_info.app_version)
            .api_version(create_info.vk_version);

        let mut extensions_names = match display_handle {
            Some(handle) => ash_window::enumerate_required_extensions(handle)?.to_vec(),
            None => vec![],
        };

        for ext in &create_info.additional_instance_extensions {
            extensions_names.push(CStr::from_ptr(ext.as_ptr() as *const i8).as_ptr());
        }

        if create_info.enable_validation {
            extensions_names.push(DebugUtils::name().as_ptr());

            let enabled_layers_names_c_strings: Vec<CString> = create_info
                .enabled_validation_layers
                .iter()
                .map(|s| CString::new(s.clone()).unwrap())
                .collect();

            let enabled_layers_names_ptr: Vec<*const i8> = enabled_layers_names_c_strings
                .iter()
                .map(|c_string| c_string.as_ptr())
                .collect();

            let debug_messenger_info = DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(create_info.log_level)
                .message_type(create_info.log_msg)
                .pfn_user_callback(Some(vulkan_debug_callback));

            let mut val_features = ValidationFeaturesEXT::builder()
                .enabled_validation_features(&create_info.enabled_validation_features);

            let instance_create_info = InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&enabled_layers_names_ptr)
                .enabled_extension_names(&extensions_names)
                .push_next(&mut val_features);

            let instance = entry.create_instance(&instance_create_info, None)?;
            let debug_utils_loader = DebugUtils::new(entry, &instance);
            let debug_messenger =
                debug_utils_loader.create_debug_utils_messenger(&debug_messenger_info, None)?;

            trace!("Created instance with validation enabled");

            trace!(
                "Enabled instance extensions count: {}",
                extensions_names.len()
            );
            for ext in extensions_names {
                let cstr = CStr::from_ptr(ext);
                trace!("{:#?}", String::from_utf8_lossy(cstr.to_bytes()));
            }

            trace!(
                "Enabled validation layer count: {}",
                enabled_layers_names_c_strings.len()
            );
            for layer in &enabled_layers_names_c_strings {
                trace!("{:#?}", layer);
            }

            trace!(
                "Enabled validation features count: {}",
                create_info.enabled_validation_features.len()
            );

            Ok((instance, Some(debug_utils_loader), Some(debug_messenger)))
        } else {
            let instance_create_info = InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(&extensions_names);
            let instance = entry.create_instance(&instance_create_info, None)?;

            trace!("Created instance with no validation enabled");
            trace!(
                "Enabled instance extensions count: {}",
                extensions_names.len()
            );
            for ext in extensions_names {
                let cstr = CStr::from_ptr(ext);
                trace!("{:#?}", String::from_utf8_lossy(cstr.to_bytes()));
            }

            Ok((instance, None, None))
        }
    }

    pub(crate) unsafe fn create_physical_device(
        instance: &Instance,
        create_info: &VkInitCreateInfo,
    ) -> Result<(PhysicalDevice, PhysicalDeviceInfo), Error> {
        let all_pdevices = instance.enumerate_physical_devices()?;
        for physical_device in all_pdevices {
            let pdevice_queue_props =
                instance.get_physical_device_queue_family_properties(physical_device);
            let pdevice_prop = instance.get_physical_device_properties(physical_device);

            if !create_info.allow_igpu
                && pdevice_prop.device_type != PhysicalDeviceType::DISCRETE_GPU
            {
                continue;
            }

            let mut unified_queue_family_index: Option<u32> = None;
            let mut transfer_queue_family_index: Option<u32> = None;
            let mut compute_queue_family_index: Option<u32> = None;

            for (index, queue_family_prop) in pdevice_queue_props.iter().enumerate() {
                let supports_transfer =
                    queue_family_prop.queue_flags.contains(QueueFlags::TRANSFER);
                let supports_compute = queue_family_prop.queue_flags.contains(QueueFlags::COMPUTE);
                let supports_graphics =
                    queue_family_prop.queue_flags.contains(QueueFlags::GRAPHICS);

                //Unified queue
                if unified_queue_family_index.is_none()
                    && supports_transfer
                    && supports_compute
                    && supports_graphics
                {
                    unified_queue_family_index = Some(index as u32);
                    continue;
                }

                //Get dedicated transfer queue
                if transfer_queue_family_index.is_none()
                    && supports_transfer
                    && !supports_compute
                    && !supports_graphics
                {
                    transfer_queue_family_index = Some(index as u32);
                    continue;
                }

                //Get dedicated compute queue
                //Any compute queue implicitly supports transfer ops
                if compute_queue_family_index.is_none() && supports_compute && !supports_graphics {
                    compute_queue_family_index = Some(index as u32);
                    continue;
                }
            }

            if let Some(unified_queue_family_index) = unified_queue_family_index {
                trace!(
                    "Picked suitable device: {:#?}",
                    char_array_to_string(&pdevice_prop.device_name)?
                );

                trace!("Physical device type: {:?}", pdevice_prop.device_type);
                trace!("Physical device limits: {:?}", pdevice_prop.limits);

                let features = instance.get_physical_device_features(physical_device);
                let memory_props = instance.get_physical_device_memory_properties(physical_device);
                let name = char_array_to_string(&pdevice_prop.device_name)?;
                let physical_device_info = PhysicalDeviceInfo {
                    name,
                    unified_queue_family_index,
                    transfer_queue_family_index,
                    compute_queue_family_index,
                    features,
                    memory_props,
                    limits: pdevice_prop.limits,
                };

                return Ok((physical_device, physical_device_info));
            }
        }
        Err(Error::NoSuitableGPUFound)
    }

    pub(crate) unsafe fn create_device(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        physical_device_info: &PhysicalDeviceInfo,
        create_info: &VkInitCreateInfo,
    ) -> Result<Device, Error> {
        let supported_extensions = instance
            .enumerate_device_extension_properties(*physical_device)
            .unwrap();

        let mut enabled_extensions_raw: Vec<*const i8> = create_info
            .additional_device_extensions
            .iter()
            .map(|ext| ext.as_ptr() as *const i8)
            .collect();

        enabled_extensions_raw.insert(0, Swapchain::name().as_ptr());

        for ext in &enabled_extensions_raw {
            let ext_name = CStr::from_ptr(*ext);
            let found = supported_extensions
                .iter()
                .find(|&&name| CStr::from_ptr(name.extension_name.as_ptr()) == ext_name);
            match found {
                Some(_) => continue,
                None => {
                    return Err(Error::RequiredDeviceExtensionNotSupported(
                        ext_name.to_str().unwrap().to_string(),
                    ))
                }
            }
        }

        let queue_priorities = [1.0];

        let mut queue_create_infos = Vec::new();

        queue_create_infos.push(
            DeviceQueueCreateInfo::builder()
                .queue_family_index(physical_device_info.unified_queue_family_index)
                .queue_priorities(&queue_priorities)
                .build(),
        );

        if let Some(transfer_index) = physical_device_info.transfer_queue_family_index {
            queue_create_infos.push(
                DeviceQueueCreateInfo::builder()
                    .queue_family_index(transfer_index)
                    .queue_priorities(&queue_priorities)
                    .build(),
            );
        }
        if let Some(compute_index) = physical_device_info.compute_queue_family_index {
            queue_create_infos.push(
                DeviceQueueCreateInfo::builder()
                    .queue_family_index(compute_index)
                    .queue_priorities(&queue_priorities)
                    .build(),
            );
        }

        let mut device_create_info = DeviceCreateInfo::builder()
            .enabled_extension_names(&enabled_extensions_raw)
            .enabled_features(&physical_device_info.features)
            .queue_create_infos(&queue_create_infos);

        let mut pdevice_1_1_features = create_info.physical_device_1_1_features;
        let mut pdevice_1_2_features = create_info.physical_device_1_2_features;
        let mut pdevice_1_3_features = create_info.physical_device_1_3_features;

        device_create_info = device_create_info.push_next(&mut pdevice_1_1_features);
        device_create_info = device_create_info.push_next(&mut pdevice_1_2_features);
        device_create_info = device_create_info.push_next(&mut pdevice_1_3_features);

        let device = instance.create_device(*physical_device, &device_create_info, None)?;
        Ok(device)
    }

    pub(crate) unsafe fn create_allocator(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        device: &Device,
    ) -> Result<Allocator, Error> {
        let create_info = AllocatorCreateInfo::new(instance, device, *physical_device);
        let allocator = vma::Allocator::new(create_info)?;
        Ok(allocator)
    }

    pub(crate) unsafe fn create_queues(
        device: &Device,
        physical_device_info: &PhysicalDeviceInfo,
    ) -> Result<(Queue, Option<Queue>, Option<Queue>), Error> {
        let unified_queue =
            device.get_device_queue(physical_device_info.unified_queue_family_index, 0);
        let transfer_queue = physical_device_info
            .transfer_queue_family_index
            .map(|transfer_index| device.get_device_queue(transfer_index, 0));
        let compute_queue = physical_device_info
            .compute_queue_family_index
            .map(|compute_index| device.get_device_queue(compute_index, 0));

        Ok((unified_queue, transfer_queue, compute_queue))
    }

    pub(crate) unsafe fn create_surface(
        entry: &Entry,
        instance: &Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        window_size: [u32; 2],
        physical_device: &PhysicalDevice,
        create_info: &VkInitCreateInfo,
    ) -> Result<(Surface, SurfaceKHR, SurfaceInfo), Error> {
        let loader = Surface::new(entry, instance);
        let surface =
            ash_window::create_surface(entry, instance, display_handle, window_handle, None)?;
        let formats = loader.get_physical_device_surface_formats(*physical_device, surface)?;

        let color_format = *formats
            .iter()
            .find(|format| format.format == create_info.surface_format)
            .ok_or(Error::RequestedSurfaceFormatNotSupported)?;

        let present_modes =
            loader.get_physical_device_surface_present_modes(*physical_device, surface)?;

        let present_mode = present_modes
            .iter()
            .copied()
            .find(|&mode| mode == create_info.present_mode)
            .ok_or(Error::PresentModeNotSupported)?;

        let capabilities =
            loader.get_physical_device_surface_capabilities(*physical_device, surface)?;

        let mut requested_img_count = create_info.request_img_count;
        if capabilities.max_image_count != 0 {
            requested_img_count = requested_img_count.min(capabilities.max_image_count);
        }
        if capabilities.min_image_count != 0 {
            requested_img_count = requested_img_count.max(capabilities.min_image_count);
        }

        let pre_transform = if capabilities
            .supported_transforms
            .contains(SurfaceTransformFlagsKHR::IDENTITY)
        {
            SurfaceTransformFlagsKHR::IDENTITY
        } else {
            capabilities.current_transform
        };

        let surface_info = SurfaceInfo {
            min_extent: capabilities.min_image_extent,
            max_extent: capabilities.max_image_extent,
            current_extent: Extent2D {
                width: window_size[0],
                height: window_size[1],
            },
            present_mode,
            image_count: requested_img_count,
            color_format,
            pre_transform,
        };

        Ok((loader, surface, surface_info))
    }

    pub(crate) unsafe fn create_swapchain(
        instance: &Instance,
        device: &Device,
        surface: &SurfaceKHR,
        surface_info: &SurfaceInfo,
        window_size: [u32; 2],
    ) -> Result<(Swapchain, SwapchainKHR), Error> {
        let window_extent = Extent2D {
            width: window_size[0],
            height: window_size[1],
        };
        let swapchain_create_info = SwapchainCreateInfoKHR::builder()
            .surface(*surface)
            .min_image_count(surface_info.image_count)
            .image_color_space(surface_info.color_format.color_space)
            .image_format(surface_info.color_format.format)
            .image_extent(window_extent)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(SharingMode::EXCLUSIVE)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .pre_transform(surface_info.pre_transform)
            .present_mode(surface_info.present_mode)
            .clipped(true)
            .image_array_layers(1);

        let loader = Swapchain::new(instance, device);
        let swapchain = loader.create_swapchain(&swapchain_create_info, None)?;

        Ok((loader, swapchain))
    }

    pub(crate) unsafe fn create_swapchain_images(
        device: &Device,
        swapchain_loader: &Swapchain,
        swapchain: &SwapchainKHR,
        surface_info: &SurfaceInfo,
    ) -> Result<(Vec<Image>, Vec<ImageView>), Error> {
        let images = swapchain_loader.get_swapchain_images(*swapchain)?;
        let mut image_views = Vec::new();
        for image in &images {
            let create_view_info = ImageViewCreateInfo::builder()
                .view_type(ImageViewType::TYPE_2D)
                .format(surface_info.color_format.format)
                .components(ComponentMapping {
                    r: ComponentSwizzle::R,
                    g: ComponentSwizzle::G,
                    b: ComponentSwizzle::B,
                    a: ComponentSwizzle::A,
                })
                .subresource_range(ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(*image);

            let image_view = device.create_image_view(&create_view_info, None)?;
            image_views.push(image_view);
        }

        Ok((images, image_views))
    }

    pub(crate) unsafe fn create_depth_image(
        device: &Device,
        allocator: &Allocator,
        window_size: [u32; 2],
        format: Format,
        sizeof: usize,
    ) -> Result<VMAImage, Error> {
        let depth_extent = Extent3D {
            width: window_size[0],
            height: window_size[1],
            depth: 1,
        };
        let depth_image =
            VMAImage::create_depth_image(device, allocator, depth_extent, format, sizeof)?;

        Ok(depth_image)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn create_head(
        device: &Device,
        allocator: &Allocator,
        entry: &Entry,
        instance: &Instance,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        window_size: [u32; 2],
        physical_device: &PhysicalDevice,
        create_info: &VkInitCreateInfo,
    ) -> Result<Head, Error> {
        let (surface_loader, surface, surface_info) = Self::create_surface(
            entry,
            instance,
            display_handle,
            window_handle,
            window_size,
            physical_device,
            create_info,
        )?;
        let (swapchain_loader, swapchain) =
            Self::create_swapchain(instance, device, &surface, &surface_info, window_size)?;
        let (swapchain_images, swapchain_image_views) =
            Self::create_swapchain_images(device, &swapchain_loader, &swapchain, &surface_info)?;
        let depth_image = Self::create_depth_image(
            device,
            allocator,
            window_size,
            create_info.depth_format,
            create_info.depth_format_sizeof,
        )?;

        Ok(Head {
            surface_loader,
            surface,
            swapchain_loader,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            clear_color_value: create_info.clear_color_value,
            clear_depth_stencil_value: create_info.clear_depth_stencil_value,
            surface_info,
            depth_format: create_info.depth_format,
            depth_format_sizeof: create_info.depth_format_sizeof,
            depth_image,
        })
    }

    pub fn change_present_mode<T: HasRawDisplayHandle + HasRawWindowHandle>(
        &mut self,
        raw_window_handles: T,
        window_size: [u32; 2],
        mode: PresentModeKHR,
    ) -> Result<(), Error> {
        unsafe {
            let display_h = raw_window_handles.raw_display_handle();
            let window_h = raw_window_handles.raw_window_handle();

            if let Some(head) = &mut self.head {
                self.device.device_wait_idle()?;
                for image_view in &head.swapchain_image_views {
                    self.device.destroy_image_view(*image_view, None);
                }
                head.swapchain_loader
                    .destroy_swapchain(head.swapchain, None);
                head.surface_loader.destroy_surface(head.surface, None);

                self.create_info.present_mode = mode;

                self.head = Some(Self::create_head(
                    &self.device,
                    &self.allocator,
                    &self.entry,
                    &self.instance,
                    display_h,
                    window_h,
                    window_size,
                    &self.physical_device,
                    &self.create_info,
                )?);
            }
        }

        Ok(())
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: DebugUtilsMessageSeverityFlagsEXT,
    _message_type: DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> Bool32 {
    let callback_data = *p_callback_data;

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    match message_severity {
        DebugUtilsMessageSeverityFlagsEXT::VERBOSE => trace!("{message}"),
        DebugUtilsMessageSeverityFlagsEXT::INFO => info!("{message}"),
        DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("{message}"),
        DebugUtilsMessageSeverityFlagsEXT::ERROR => error!("{message}"),
        _ => (),
    };

    FALSE
}
