use std::mem::ManuallyDrop;

use gpu_allocator::vulkan::AllocatorCreateDesc;
use gpu_allocator::{AllocationSizes, AllocatorDebugSettings};
use raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle};

use crate::create_info::{VkInitCreateInfo, WindowOptions};
use crate::fn_loader::FnLoader;
use crate::{imports::*, VMAImage};

/// Wrapper around 'static' vulkan objects (instance, device etc.), optional head (surface, swapchain etc.), and utility functions for ease of use.
///
/// Handles initialization and destruction of Vulkan objects and offers utility functions for:
/// - GLSL shader compilation with #include directive
/// - Swapchain recreation and resizing
/// - Optionally exposed dedicated compute and transfer queues
/// - Shortcuts for present and submit operations
pub struct VkInit<'a> {
    /// [GPU-Allocator](gpu-allocator::vulkan::Allocator)
    pub allocator: ManuallyDrop<Allocator>,
    pub entry: Entry,
    pub(crate) fn_loader: FnLoader,
    pub instance: Instance,
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
    pub create_info: VkInitCreateInfo<'a>,
}

/// Wrapper around presentation resources.
pub struct Head {
    pub surface: SurfaceKHR,
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
/// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build().unwrap();
/// # let size = [800_u32, 600_u32];
/// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
/// # let create_info = VkInitCreateInfo::default();
/// # let window_options = WindowOptions::new(window, size);
/// let init = VkInit::new(Some(window_options), create_info)?;
///
/// let (compute_queue, compute_queue_family_index) = init.get_queue(CmdType::Compute);
/// # Ok::<(), vku::Error>(())
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

impl<'a> VkInit<'a> {
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
    /// use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    /// use vku::{VkInitCreateInfo, WindowOptions, VkInit};
    ///
    /// let event_loop: EventLoop<()> = EventLoopBuilder::default().build().unwrap();
    /// let size = [800_u32, 600_u32];
    /// let window = WindowBuilder::new()
    ///     .with_inner_size(LogicalSize{width: size[0], height: size[1]})
    ///     .build(&event_loop).unwrap();
    /// let create_info = VkInitCreateInfo::default();
    /// let window_options = WindowOptions::new(window, size);
    /// let init = VkInit::new(Some(window_options), create_info)?;
    /// # Ok::<(), vku::Error>(())
    /// ```

    pub fn new<T: HasDisplayHandle + HasWindowHandle>(
        window_options: Option<WindowOptions<T>>,
        create_info: VkInitCreateInfo<'a>,
    ) -> Result<Self, Error> {
        unsafe {
            let display_h = match &window_options {
                Some(w) => w.handle.display_handle().map(Some),
                None => Ok(None),
            }?;

            let mut fn_loader = FnLoader::default();

            #[cfg(feature = "linked")]
            let entry = ash::Entry::linked();

            #[cfg(not(feature = "linked"))]
            let entry = ash::Entry::load()?;

            let (instance, debug_messenger) =
                Self::create_instance_and_debug(&mut fn_loader, &entry, display_h, &create_info)?;
            let (physical_device, physical_device_info) =
                Self::create_physical_device(&instance, &create_info)?;
            let device = Self::create_device(
                &instance,
                &physical_device,
                &physical_device_info,
                &create_info,
            )?;
            let mut allocator = Self::create_allocator(&instance, &physical_device, &device)?;
            let (unified_queue, transfer_queue, compute_queue) =
                Self::create_queues(&device, &physical_device_info)?;

            let head = if let Some(w) = window_options {
                Some(Self::create_head(
                    &mut fn_loader,
                    &device,
                    &mut allocator,
                    &entry,
                    &instance,
                    &physical_device,
                    &create_info,
                    w,
                )?)
            } else {
                None
            };

            //TODO: Why is RenderDoc crashing when Instance debug name is set?
            //TODO: Why are swapchain, swapchain_images, and swapchain_image_views names not set in RenderDoc?
            if let Ok(dbg) = fn_loader.debug_utils_device() {
                Self::set_debug_object_name_static(
                    dbg,
                    physical_device,
                    "VKU_Physical_Device".to_string(),
                )?;
                Self::set_debug_object_name_static(dbg, device.handle(), "VKU_Device".to_string())?;
                Self::set_debug_object_name_static(
                    dbg,
                    unified_queue,
                    "VKU_Unified_Queue".to_string(),
                )?;
                if let Some(transfer_queue) = transfer_queue {
                    Self::set_debug_object_name_static(
                        dbg,
                        transfer_queue,
                        "VKU_Transfer_Queue".to_string(),
                    )?;
                }

                if let Some(compute_queue) = compute_queue {
                    Self::set_debug_object_name_static(
                        dbg,
                        compute_queue,
                        "VKU_Compute_Queue".to_string(),
                    )?;
                }

                if let Some(head) = &head {
                    Self::set_debug_object_name_static(
                        dbg,
                        head.swapchain,
                        "VKU_Swapchain".to_string(),
                    )?;

                    Self::set_debug_object_name_static(
                        dbg,
                        head.depth_image.image,
                        "VKU_DepthImage".to_string(),
                    )?;

                    Self::set_debug_object_name_static(
                        dbg,
                        head.depth_image.image_view,
                        "VKU_DepthImage_View".to_string(),
                    )?;

                    Self::set_debug_object_name_static(
                        dbg,
                        head.depth_image.allocation.memory(),
                        "VKU_DepthImage_Memory".to_string(),
                    )?;

                    for (i, image) in head.swapchain_images.iter().enumerate() {
                        Self::set_debug_object_name_static(
                            dbg,
                            *image,
                            format!("VKU_Swapchain_Image_{i}"),
                        )?;
                    }

                    for (i, image_view) in head.swapchain_image_views.iter().enumerate() {
                        Self::set_debug_object_name_static(
                            dbg,
                            *image_view,
                            format!("VKU_Swapchain_Image_View_{i}"),
                        )?;
                    }
                }
            }

            trace!("Created VkInit");

            Ok(Self {
                fn_loader,
                allocator: ManuallyDrop::new(allocator),
                entry,
                instance,
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
                self.fn_loader
                    .swapchain_device()?
                    .destroy_swapchain(head.swapchain, None);
                self.fn_loader
                    .surface_instance()?
                    .destroy_surface(head.surface, None);
                head.depth_image
                    .destroy(&self.device, &mut self.allocator)?;
            }

            if let Ok(dbg_instance) = self.fn_loader.debug_utils_instance() {
                if let Some(dbg_msg) = self.debug_messenger {
                    dbg_instance.destroy_debug_utils_messenger(dbg_msg, None);
                }
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

    pub fn set_debug_object_name<T: Handle>(
        &self,
        obj_handle: T,
        name: String,
    ) -> Result<(), Error> {
        let Ok(loader) = self.fn_loader.debug_utils_device() else {
            return Ok(());
        };

        let c_name = CString::new(name)?;

        let name_info = DebugUtilsObjectNameInfoEXT::default()
            .object_name(&c_name)
            .object_handle(obj_handle);

        unsafe {
            loader.set_debug_utils_object_name(&name_info)?;
        }

        Ok(())
    }

    pub fn insert_debug_label(&self, cmd_buffer: &CommandBuffer, name: &str) -> Result<(), Error> {
        let Ok(loader) = &self.fn_loader.debug_utils_device() else {
            return Ok(());
        };

        let label_info = DebugUtilsLabelEXT::default()
            .label_name(unsafe { CStr::from_ptr(name.as_ptr() as *const i8) });

        unsafe { loader.cmd_insert_debug_utils_label(*cmd_buffer, &label_info) };

        Ok(())
    }

    pub fn begin_debug_label(&self, cmd_buffer: &CommandBuffer, name: &str) -> Result<(), Error> {
        let Ok(loader) = &self.fn_loader.debug_utils_device() else {
            return Ok(());
        };

        let label_info = DebugUtilsLabelEXT::default()
            .label_name(unsafe { CStr::from_ptr(name.as_ptr() as *const i8) });

        unsafe { loader.cmd_begin_debug_utils_label(*cmd_buffer, &label_info) };

        Ok(())
    }

    pub fn end_debug_label(&self, cmd_buffer: &CommandBuffer) -> Result<(), Error> {
        let Ok(loader) = &self.fn_loader.debug_utils_device() else {
            return Ok(());
        };

        unsafe { loader.cmd_end_debug_utils_label(*cmd_buffer) };

        Ok(())
    }

    pub fn create_cmd_pool(&self, cmd_type: CmdType) -> Result<CommandPool, Error> {
        let (_, queue_family_index) = self.get_queue(cmd_type);
        let create_info = CommandPoolCreateInfo::default()
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
        let create_info = CommandBufferAllocateInfo::default()
            .command_pool(*pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        let alloc = unsafe { self.device.allocate_command_buffers(&create_info)? };
        Ok(alloc)
    }

    /// Creates a signaled fence.
    pub fn create_fence(&self) -> Result<Fence, Error> {
        let create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);
        let fence = unsafe { self.device.create_fence(&create_info, None)? };

        Ok(fence)
    }

    /// Creates a Vec of signaled fence.
    pub fn create_fences(&self, count: usize) -> Result<Vec<Fence>, Error> {
        let mut fences = Vec::new();
        for _ in 0..count {
            let create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);
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
        let Some(head) = self.head.as_ref() else {
            return Err(Error::HeadCallOnHeadlessInstance);
        };
        let (index, sub_optimal) = unsafe {
            self.fn_loader.swapchain_device()?.acquire_next_image(
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
            CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(*cmd_buffer, &cmd_buffer_begin_info)?
        };

        Ok(())
    }

    pub fn begin_rendering(
        &self,
        swapchain_image_view: &ImageView,
        cmd_buffer: &CommandBuffer,
    ) -> Result<(), Error> {
        let Some(head) = self.head.as_ref() else {
            return Err(Error::HeadCallOnHeadlessInstance);
        };

        let clear_color_value = ClearValue {
            color: head.clear_color_value,
        };
        let clear_depth_stencil_value = ClearValue {
            depth_stencil: head.clear_depth_stencil_value,
        };

        let render_area = Rect2D::default()
            .offset(Offset2D { x: 0, y: 0 })
            .extent(head.surface_info.current_extent);

        let color_attachment_info = [RenderingAttachmentInfo::default()
            .image_view(*swapchain_image_view)
            .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .clear_value(clear_color_value)];

        let depth_attachment_info = RenderingAttachmentInfo::default()
            .image_view(head.depth_image.image_view)
            .image_layout(ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .clear_value(clear_depth_stencil_value);

        let rendering_begin_info = RenderingInfo::default()
            .render_area(render_area)
            .layer_count(1)
            .color_attachments(&color_attachment_info)
            .depth_attachment(&depth_attachment_info);

        unsafe {
            self.device
                .cmd_begin_rendering(*cmd_buffer, &rendering_begin_info);
        }

        Ok(())
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
        let mut submit_info = SubmitInfo::default()
            .command_buffers(&cmd_buffers)
            .wait_dst_stage_mask(wait_dst_flags)
            .signal_semaphores(signal_sem)
            .wait_semaphores(wait_sem);

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
        let dependency_info = DependencyInfo::default()
            .image_memory_barriers(image_memory_barriers)
            .buffer_memory_barriers(buffer_memory_barriers)
            .dependency_flags(DependencyFlags::empty());

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
        let Some(head) = self.head.as_ref() else {
            return Err(Error::HeadCallOnHeadlessInstance);
        };

        let Ok(loader) = &self.fn_loader.swapchain_device() else {
            return Err(Error::FnLoaderNotInitialized(
                "swapchain device".to_string(),
            ));
        };

        let swapchains = [head.swapchain];
        let image_indices = [frame as u32];
        let wait_sems = [*rendering_complete_semaphore];
        let present_info = ash::vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_sems)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            loader.queue_present(self.unified_queue, &present_info)?;
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
                if let (Some(queue), Some(index)) = (
                    self.transfer_queue,
                    self.physical_device_info.transfer_queue_family_index,
                ) {
                    (queue, index)
                } else {
                    (
                        self.unified_queue,
                        self.physical_device_info.unified_queue_family_index,
                    )
                }
            }
            CmdType::Compute => {
                if let (Some(queue), Some(index)) = (
                    self.compute_queue,
                    self.physical_device_info.compute_queue_family_index,
                ) {
                    (queue, index)
                } else {
                    (
                        self.unified_queue,
                        self.physical_device_info.unified_queue_family_index,
                    )
                }
            }
        }
    }

    fn set_debug_object_name_static<T: Handle>(
        dbg: &ash::ext::debug_utils::Device,
        obj_handle: T,
        name: String,
    ) -> Result<(), Error> {
        let c_name = CString::new(name)?;
        let name_info = DebugUtilsObjectNameInfoEXT::default()
            .object_name(&c_name)
            .object_handle(obj_handle);

        unsafe { dbg.set_debug_utils_object_name(&name_info)? };
        Ok(())
    }

    pub(crate) unsafe fn create_instance_and_debug(
        fn_load: &mut FnLoader,
        entry: &Entry,
        display_handle: Option<DisplayHandle>,
        create_info: &VkInitCreateInfo,
    ) -> Result<(Instance, Option<DebugUtilsMessengerEXT>), Error> {
        let app_info = ApplicationInfo::default()
            .application_name(CStr::from_ptr(create_info.app_name.as_ptr() as *const i8))
            .engine_name(CStr::from_ptr(create_info.engine_name.as_ptr() as *const i8))
            .application_version(create_info.app_version)
            .api_version(create_info.vk_version);

        let mut extensions_names = match display_handle {
            Some(handle) => ash_window::enumerate_required_extensions(handle.as_raw())?.to_vec(),
            None => vec![],
        };

        for ext in &create_info.additional_instance_extensions {
            extensions_names.push(CStr::from_ptr(ext.as_ptr() as *const i8).as_ptr());
        }

        if create_info.enable_validation {
            extensions_names.push(ash::ext::debug_utils::NAME.as_ptr());

            let supported_layers: Vec<String> = entry
                .enumerate_instance_layer_properties()?
                .iter()
                .filter_map(|prop| char_array_to_string(&prop.layer_name).ok())
                .collect();

            let enabled_layers_names_c_strings: Vec<CString> = create_info
                .enabled_validation_layers
                .iter()
                .filter(|layer| supported_layers.contains(*layer))
                .filter_map(|s| CString::new(s.clone()).ok())
                .collect();

            let enabled_layers_names_ptr: Vec<*const i8> = enabled_layers_names_c_strings
                .iter()
                .map(|c_string| c_string.as_ptr())
                .collect();

            let debug_messenger_info = DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(create_info.log_level)
                .message_type(create_info.log_msg)
                .pfn_user_callback(Some(vulkan_debug_callback));

            let mut val_features = ValidationFeaturesEXT::default()
                .enabled_validation_features(&create_info.enabled_validation_features);

            let instance_create_info = InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(&enabled_layers_names_ptr)
                .enabled_extension_names(&extensions_names)
                .push_next(&mut val_features);

            let instance = entry.create_instance(&instance_create_info, None)?;

            fn_load
                .debug_utils_instance
                .get_or_insert(ash::ext::debug_utils::Instance::new(entry, &instance));

            let debug_messenger = fn_load
                .debug_utils_instance()?
                .create_debug_utils_messenger(&debug_messenger_info, None)?;

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

            Ok((instance, Some(debug_messenger)))
        } else {
            let instance_create_info = InstanceCreateInfo::default()
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

            Ok((instance, None))
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
        let supported_extensions =
            instance.enumerate_device_extension_properties(*physical_device)?;

        let mut enabled_extensions_raw: Vec<*const i8> = create_info
            .additional_device_extensions
            .iter()
            .map(|ext| ext.as_ptr() as *const i8)
            .collect();

        enabled_extensions_raw.insert(0, ash::khr::swapchain::NAME.as_ptr());

        for ext in &enabled_extensions_raw {
            let ext_name = CStr::from_ptr(*ext);
            let found = supported_extensions
                .iter()
                .find(|&&name| CStr::from_ptr(name.extension_name.as_ptr()) == ext_name);
            match found {
                Some(_) => continue,
                None => {
                    return Err(Error::RequiredDeviceExtensionNotSupported(
                        ext_name.to_str()?.to_string(),
                    ))
                }
            }
        }

        let queue_priorities = [1.0];

        let mut queue_create_infos = Vec::new();

        queue_create_infos.push(
            DeviceQueueCreateInfo::default()
                .queue_family_index(physical_device_info.unified_queue_family_index)
                .queue_priorities(&queue_priorities),
        );

        if let Some(transfer_index) = physical_device_info.transfer_queue_family_index {
            queue_create_infos.push(
                DeviceQueueCreateInfo::default()
                    .queue_family_index(transfer_index)
                    .queue_priorities(&queue_priorities),
            );
        }
        if let Some(compute_index) = physical_device_info.compute_queue_family_index {
            queue_create_infos.push(
                DeviceQueueCreateInfo::default()
                    .queue_family_index(compute_index)
                    .queue_priorities(&queue_priorities),
            );
        }

        let mut device_create_info = DeviceCreateInfo::default()
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
        trace!("Created device");
        Ok(device)
    }

    pub(crate) unsafe fn create_allocator(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        device: &Device,
    ) -> Result<Allocator, Error> {
        let create_info = AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: *physical_device,
            debug_settings: AllocatorDebugSettings {
                log_memory_information: false,
                log_leaks_on_shutdown: true,
                store_stack_traces: false,
                log_allocations: false,
                log_frees: false,
                log_stack_traces: false,
            },
            buffer_device_address: false,
            allocation_sizes: AllocationSizes::default(),
        };
        let allocator = Allocator::new(&create_info)?;
        trace!("Created allocator");
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

        trace!("Created queues");
        Ok((unified_queue, transfer_queue, compute_queue))
    }

    pub(crate) unsafe fn create_surface<T: HasDisplayHandle + HasWindowHandle>(
        fn_loader: &FnLoader,
        entry: &Entry,
        instance: &Instance,
        physical_device: &PhysicalDevice,
        create_info: &VkInitCreateInfo,
        window_options: &WindowOptions<T>,
    ) -> Result<(SurfaceKHR, SurfaceInfo), Error> {
        let loader = fn_loader.surface_instance()?;
        let raw_display_h = window_options.handle.display_handle()?.as_raw();
        let raw_window_h = window_options.handle.window_handle()?.as_raw();

        let surface =
            ash_window::create_surface(entry, instance, raw_display_h, raw_window_h, None)?;
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
                width: window_options.size[0].clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: window_options.size[1].clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            },
            present_mode,
            image_count: requested_img_count,
            color_format,
            pre_transform,
        };

        trace!("Created surface");
        Ok((surface, surface_info))
    }

    pub(crate) unsafe fn create_swapchain(
        fn_loader: &FnLoader,
        surface: &SurfaceKHR,
        surface_info: &SurfaceInfo,
    ) -> Result<SwapchainKHR, Error> {
        let loader = fn_loader.swapchain_device()?;

        let swapchain_create_info = SwapchainCreateInfoKHR::default()
            .surface(*surface)
            .min_image_count(surface_info.image_count)
            .image_color_space(surface_info.color_format.color_space)
            .image_format(surface_info.color_format.format)
            .image_extent(surface_info.current_extent)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(SharingMode::EXCLUSIVE)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .pre_transform(surface_info.pre_transform)
            .present_mode(surface_info.present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = loader.create_swapchain(&swapchain_create_info, None)?;

        trace!("Created swapchain");
        Ok(swapchain)
    }

    pub(crate) unsafe fn create_swapchain_images(
        fn_loader: &FnLoader,
        device: &Device,
        swapchain: &SwapchainKHR,
        surface_info: &SurfaceInfo,
    ) -> Result<(Vec<Image>, Vec<ImageView>), Error> {
        let loader = fn_loader.swapchain_device()?;

        let images = loader.get_swapchain_images(*swapchain)?;

        let mut image_views = Vec::new();
        for image in &images {
            let create_view_info = ImageViewCreateInfo::default()
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

        trace!("Created swapchain images");
        Ok((images, image_views))
    }

    pub(crate) unsafe fn create_depth_image(
        device: &Device,
        allocator: &mut Allocator,
        surface_info: &SurfaceInfo,
        format: Format,
        sizeof: usize,
    ) -> Result<VMAImage, Error> {
        let depth_image =
            VMAImage::create_depth_image(device, allocator, surface_info, format, sizeof)?;

        trace!("Created depth images");
        Ok(depth_image)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn create_head<T: HasDisplayHandle + HasWindowHandle>(
        fn_loader: &mut FnLoader,
        device: &Device,
        allocator: &mut Allocator,
        entry: &Entry,
        instance: &Instance,
        physical_device: &PhysicalDevice,
        create_info: &VkInitCreateInfo,
        window_options: WindowOptions<T>,
    ) -> Result<Head, Error> {
        fn_loader
            .surface_instance
            .get_or_insert(ash::khr::surface::Instance::new(entry, instance));
        fn_loader
            .swapchain_instance
            .get_or_insert(ash::khr::swapchain::Instance::new(entry, instance));
        fn_loader
            .swapchain_device
            .get_or_insert(ash::khr::swapchain::Device::new(instance, device));

        let (surface, surface_info) = Self::create_surface(
            fn_loader,
            entry,
            instance,
            physical_device,
            create_info,
            &window_options,
        )?;
        let swapchain = Self::create_swapchain(fn_loader, &surface, &surface_info)?;
        let (swapchain_images, swapchain_image_views) =
            Self::create_swapchain_images(fn_loader, device, &swapchain, &surface_info)?;
        let depth_image = Self::create_depth_image(
            device,
            allocator,
            &surface_info,
            create_info.depth_format,
            create_info.depth_format_sizeof,
        )?;

        Ok(Head {
            surface,
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

    pub fn change_present_mode<T: HasDisplayHandle + HasWindowHandle>(
        &mut self,
        window_options: WindowOptions<T>,
        mode: PresentModeKHR,
    ) -> Result<(), Error> {
        unsafe {
            if let Some(head) = &mut self.head {
                self.device.device_wait_idle()?;
                for image_view in &head.swapchain_image_views {
                    self.device.destroy_image_view(*image_view, None);
                }
                self.fn_loader
                    .swapchain_device()?
                    .destroy_swapchain(head.swapchain, None);

                self.fn_loader
                    .surface_instance()?
                    .destroy_surface(head.surface, None);

                self.create_info.present_mode = mode;

                self.head = Some(Self::create_head(
                    &mut self.fn_loader,
                    &self.device,
                    &mut self.allocator,
                    &self.entry,
                    &self.instance,
                    &self.physical_device,
                    &self.create_info,
                    window_options,
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
