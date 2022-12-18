use crate::create_info::VkInitCreateInfo;
use crate::errors::VkInitError;
use crate::imports::*;

pub trait VkDestroy {
    fn destroy(&self, vk_init: &VkInit) -> Result<()>;
}

/// Wrapper around 'static' vulkan objects (instance, device etc.) and utility functions for ease of use.
///
/// Handles initialization and destruction of Vulkan objects and offers utility functions for:
/// - GLSL shader compilation with #include directive
/// - Swapchain recreation and resizing
/// - Optionally exposed dedicated compute and transfer queues
/// - Shortcuts for present and submit operations
pub struct VkInit {
    pub info: VkInitInfo,
    /// [VMA](vk_mem_alloc::Allocator) allocator
    pub allocator: Allocator,
    pub entry: Entry,
    pub instance: Instance,
    /// Only created with [VkInitCreateInfo](crate::VkInitCreateInfo).enable_validation
    pub debug_loader: Option<DebugUtils>,
    /// Only created with [VkInitCreateInfo](crate::VkInitCreateInfo).enable_validation    
    pub debug_messenger: Option<DebugUtilsMessengerEXT>,
    pub physical_device: PhysicalDevice,
    pub device: Device,
    /// Unfified queue is guarenteed to be present per vulkan spec and can handle any command
    pub unified_queue: Queue,
    /// Optionally exposed
    pub transfer_queue: Option<Queue>,
    /// Optionally exposed
    pub compute_queue: Option<Queue>,
    pub surface_loader: Surface,
    pub surface: SurfaceKHR,
    pub swapchain_loader: Swapchain,
    pub swapchain: SwapchainKHR,
    pub swapchain_images: Vec<Image>,
    pub swapchain_image_views: Vec<ImageView>,
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
/// # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
/// # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
/// # let create_info = VkInitCreateInfo::default();
/// let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
///
/// let (compute_queue, compute_family_index) = init.get_queue(CmdType::Compute);
pub enum CmdType {
    /// Graphics | Transfer | Compute
    Any,
    Graphics,
    Transfer,
    Compute,
}

/// Info wrapper around [PhysicalDeviceInfo](crate::init::PhysicalDeviceInfo) and [SurfaceInfo](crate::init::SurfaceInfo).
pub struct VkInitInfo {
    pub physical_device_info: PhysicalDeviceInfo,
    pub surface_info: SurfaceInfo,
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

    pub max_work_group_dispatch: [u32; 3],
    pub max_work_group_size: [u32; 3],
    pub max_work_group_invocations: u32,
    pub max_shared_memory_size: u32,
    pub max_bound_descriptor_sets: u32,
}

/// Return info about the created surface and its capabilities.
pub struct SurfaceInfo {
    pub min_extent: Extent2D,
    pub max_extent: Extent2D,
    pub current_extent: Extent2D,
    pub present_mode: PresentModeKHR,
    pub format: SurfaceFormatKHR,
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
    /// let display_handle = window.raw_display_handle();
    /// let window_handle = window.raw_window_handle();
    /// let create_info = VkInitCreateInfo::default();
    ///
    /// let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
    /// ```
    pub fn new(
        display_handle: &RawDisplayHandle,
        window_handle: &RawWindowHandle,
        window_size: [u32; 2],
        create_info: &VkInitCreateInfo,
    ) -> Result<Self> {
        unsafe {
            let window_extent = Extent2D {
                width: window_size[0],
                height: window_size[1],
            };
            let entry = ash::Entry::linked();
            let (instance, debug_loader, debug_messenger) =
                Self::create_instance_and_debug(&entry, display_handle, create_info)?;
            let (physical_device, physical_device_info) =
                Self::create_physical_device(&instance, create_info)?;
            let device = Self::create_device(
                &instance,
                &physical_device,
                &physical_device_info,
                create_info,
            )?;
            let allocator = Self::create_allocator(&instance, &physical_device, &device)?;
            let (unified_queue, transfer_queue, compute_queue) =
                Self::create_queues(&device, &physical_device_info)?;
            let (surface_loader, surface, surface_info) = Self::create_surface(
                &entry,
                &instance,
                display_handle,
                window_handle,
                window_size,
                &physical_device,
                create_info,
            )?;
            let (swapchain_loader, swapchain) = Self::create_swapchain(
                &instance,
                &device,
                &surface,
                &surface_info,
                &window_extent,
                create_info.frames_in_flight,
            )?;
            let (swapchain_images, swapchain_image_views) = Self::create_swapchain_images(
                &device,
                &swapchain_loader,
                &swapchain,
                &surface_info,
            )?;

            let info = VkInitInfo {
                physical_device_info,
                surface_info,
            };

            info!("VkInit created successfully");

            Ok(Self {
                info,
                allocator,
                entry,
                instance,
                debug_loader,
                debug_messenger,
                physical_device,
                device,
                unified_queue,
                compute_queue,
                transfer_queue,
                surface_loader,
                surface,
                swapchain_loader,
                swapchain,
                swapchain_images,
                swapchain_image_views,
            })
        }
    }

    pub fn destroy(&self) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()?;
            for image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(*image_view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
            vk_mem_alloc::destroy_allocator(self.allocator);
            self.device.destroy_device(None);
            if let Some(dbg_loader) = &self.debug_loader {
                dbg_loader.destroy_debug_utils_messenger(self.debug_messenger.unwrap(), None);
            }
            self.instance.destroy_instance(None);
        }

        info!("VkInit destroyed successfully");

        Ok(())
    }

    pub fn create_cmd_pool(&self, cmd_type: CmdType) -> Result<CommandPool> {
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
    ) -> Result<Vec<CommandBuffer>> {
        let create_info = CommandBufferAllocateInfo::builder()
            .command_pool(*pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        let alloc = unsafe { self.device.allocate_command_buffers(&create_info)? };
        Ok(alloc)
    }

    /// Creates a signaled fence.
    pub fn create_fence(&self) -> Result<Fence> {
        let create_info = FenceCreateInfo::builder().flags(FenceCreateFlags::SIGNALED);
        let fence = unsafe { self.device.create_fence(&create_info, None)? };

        Ok(fence)
    }

    /// Creates a Vec of signaled fence.
    pub fn create_fences(&self, count: usize) -> Result<Vec<Fence>> {
        let mut fences = Vec::new();
        for _ in 0..count {
            let create_info = FenceCreateInfo::builder().flags(FenceCreateFlags::SIGNALED);
            let fence = unsafe { self.device.create_fence(&create_info, None)? };
            fences.push(fence);
        }

        Ok(fences)
    }

    pub fn destroy_fence(&self, fence: &Fence) -> Result<()> {
        unsafe {
            self.device.destroy_fence(*fence, None);
        }

        Ok(())
    }

    pub fn create_semaphore(&self) -> Result<Semaphore> {
        let create_info = SemaphoreCreateInfo::default();
        let semaphore = unsafe { self.device.create_semaphore(&create_info, None)? };

        Ok(semaphore)
    }

    pub fn create_semaphores(&self, count: usize) -> Result<Vec<Semaphore>> {
        let mut semaphores = Vec::new();
        for _ in 0..count {
            let create_info = SemaphoreCreateInfo::default();
            let semaphore = unsafe { self.device.create_semaphore(&create_info, None)? };
            semaphores.push(semaphore);
        }

        Ok(semaphores)
    }

    pub fn destroy_semaphore(&self, semaphore: &Semaphore) -> Result<()> {
        unsafe {
            self.device.destroy_semaphore(*semaphore, None);
        }

        Ok(())
    }

    pub fn destroy_cmd_pool(&self, pool: &CommandPool) -> Result<()> {
        unsafe {
            self.device.destroy_command_pool(*pool, None);
        }

        Ok(())
    }

    /// Acquires image for frame ```frame``` and signals sempahore ```acquire_img_semaphore```.
    pub fn acquire_next_swapchain_image(
        &self,
        acquire_img_semaphore: Semaphore,
        frame: usize,
    ) -> Result<(Image, ImageView)> {
        unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                acquire_img_semaphore,
                Fence::null(),
            )?;
        }
        let swapchain_image = self.swapchain_images[frame];
        let swapchain_image_view = self.swapchain_image_views[frame];
        Ok((swapchain_image, swapchain_image_view))
    }

    pub fn begin_cmd_buffer(&self, cmd_buffer: &CommandBuffer) -> Result<()> {
        let cmd_buffer_begin_info =
            CommandBufferBeginInfo::builder().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(*cmd_buffer, &cmd_buffer_begin_info)?
        };

        Ok(())
    }

    pub fn begin_rendering(&self, swapchain_image_view: &ImageView, cmd_buffer: &CommandBuffer) {
        let clear_value = ClearValue {
            color: ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
        };
        let render_area = Rect2D::builder()
            .offset(Offset2D { x: 0, y: 0 })
            .extent(self.info.surface_info.current_extent);

        let color_attachment_info = [RenderingAttachmentInfo::builder()
            .image_view(*swapchain_image_view)
            .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .clear_value(clear_value)
            .build()];

        let rendering_begin_info = RenderingInfo::builder()
            .render_area(*render_area)
            .layer_count(1)
            .color_attachments(&color_attachment_info);

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
    ) -> Result<()> {
        unsafe { self.device.end_command_buffer(*cmd_buffer)? };

        let mut submit_info = SubmitInfo::builder()
            .command_buffers(&[*cmd_buffer])
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
        fence: &Fence,
        cmd_buffers: &[&CommandBuffer],
    ) -> Result<()> {
        unsafe {
            self.device.wait_for_fences(&[*fence], true, u64::MAX)?;
            self.device.reset_fences(&[*fence])?;
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
            .build();

        unsafe {
            self.device
                .cmd_pipeline_barrier2(*cmd_buffer, &dependency_info);
        }
    }

    pub fn present(&self, rendering_complete_semaphore: &Semaphore, frame: usize) -> Result<()> {
        let swapchains = [self.swapchain];
        let image_indices = [frame as u32];
        let present_info = ash::vk::PresentInfoKHR::builder()
            .wait_semaphores(&[*rendering_complete_semaphore])
            .swapchains(&swapchains)
            .image_indices(&image_indices)
            .build();

        unsafe {
            self.swapchain_loader
                .queue_present(self.unified_queue, &present_info)?;
        }

        Ok(())
    }

    pub fn wait_device_idle(&self) -> Result<()> {
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
                self.info.physical_device_info.unified_queue_family_index,
            ),
            CmdType::Graphics => (
                self.unified_queue,
                self.info.physical_device_info.unified_queue_family_index,
            ),
            CmdType::Transfer => {
                //TODO: Implement as if-let chains once stabilized
                if self
                    .info
                    .physical_device_info
                    .transfer_queue_family_index
                    .is_some()
                    && self.transfer_queue.is_some()
                {
                    (
                        self.transfer_queue.unwrap(),
                        self.info
                            .physical_device_info
                            .transfer_queue_family_index
                            .unwrap(),
                    )
                } else {
                    (
                        self.unified_queue,
                        self.info.physical_device_info.unified_queue_family_index,
                    )
                }
            }
            CmdType::Compute => {
                //TODO: Implement as if-let chains once stabilized
                if self
                    .info
                    .physical_device_info
                    .compute_queue_family_index
                    .is_some()
                    && self.compute_queue.is_some()
                {
                    (
                        self.compute_queue.unwrap(),
                        self.info
                            .physical_device_info
                            .compute_queue_family_index
                            .unwrap(),
                    )
                } else {
                    (
                        self.unified_queue,
                        self.info.physical_device_info.unified_queue_family_index,
                    )
                }
            }
        }
    }

    pub(crate) unsafe fn create_instance_and_debug(
        entry: &Entry,
        display_handle: &RawDisplayHandle,
        create_info: &VkInitCreateInfo,
    ) -> Result<(Instance, Option<DebugUtils>, Option<DebugUtilsMessengerEXT>)> {
        let app_info = ApplicationInfo::builder()
            .application_name(CStr::from_ptr(create_info.app_name.as_ptr() as *const i8))
            .engine_name(CStr::from_ptr(create_info.engine_name.as_ptr() as *const i8))
            .application_version(create_info.app_version)
            .api_version(create_info.vk_version);

        let mut extensions_names =
            ash_window::enumerate_required_extensions(*display_handle)?.to_vec();

        for ext in &create_info.additional_instance_extensions {
            extensions_names.push(CStr::from_ptr(ext.as_ptr() as *const i8).as_ptr());
        }

        if create_info.enable_validation {
            extensions_names.push(DebugUtils::name().as_ptr());

            let enabled_layers_names: Vec<*const c_char> = create_info
                .enabled_validation_layers
                .iter()
                .map(|s| CStr::from_ptr(s.as_ptr() as *const i8).as_ptr())
                .collect();

            let mut debug_messenger_info = DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(create_info.log_level)
                .message_type(create_info.log_msg)
                .pfn_user_callback(Some(vulkan_debug_callback));

            let mut val_features = ValidationFeaturesEXT::builder()
                .enabled_validation_features(&create_info.enabled_validation_features);

            let mut instance_create_info = InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&enabled_layers_names)
                .enabled_extension_names(&extensions_names)
                .push_next(&mut val_features);

            instance_create_info = instance_create_info.push_next(&mut debug_messenger_info);

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
                enabled_layers_names.len()
            );
            for layer in &enabled_layers_names {
                let cstr = CStr::from_ptr(*layer);
                trace!("{:#?}", String::from_utf8_lossy(cstr.to_bytes()));
            }

            trace!(
                "Enabled validation features count: {}",
                create_info.enabled_validation_features.len()
            );
            for feature in &create_info.enabled_validation_features {
                trace!("{:#?}", feature);
            }

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
    ) -> Result<(PhysicalDevice, PhysicalDeviceInfo)> {
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

                trace!("Queue family indices: unified queue: {:?}, dedicated transfer: {:?} | dedicated compute: {:?}",
                unified_queue_family_index, transfer_queue_family_index, compute_queue_family_index);

                trace!(
                    "Max_compute_work_group_count_xyz: {:?}",
                    pdevice_prop.limits.max_compute_work_group_count
                );
                trace!(
                    "Max_compute_work_group_size_xyz: {:?}",
                    pdevice_prop.limits.max_compute_work_group_size
                );
                trace!(
                    "Max_compute_work_group_invocations: {}",
                    pdevice_prop.limits.max_compute_work_group_invocations
                );
                trace!(
                    "Max_bound_descriptor_sets: {}",
                    pdevice_prop.limits.max_bound_descriptor_sets
                );

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

                    max_work_group_dispatch: pdevice_prop.limits.max_compute_work_group_count,
                    max_work_group_size: pdevice_prop.limits.max_compute_work_group_size,
                    max_work_group_invocations: pdevice_prop
                        .limits
                        .max_compute_work_group_invocations,
                    max_shared_memory_size: pdevice_prop.limits.max_compute_shared_memory_size,
                    max_bound_descriptor_sets: pdevice_prop.limits.max_bound_descriptor_sets,
                };

                return Ok((physical_device, physical_device_info));
            }
        }
        Err(anyhow!(VkInitError::NoSuitableGPUFound))
    }

    pub(crate) unsafe fn create_device(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        physical_device_info: &PhysicalDeviceInfo,
        create_info: &VkInitCreateInfo,
    ) -> Result<Device> {
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
                    return Err(anyhow!(VkInitError::RequiredDeviceExtensionNotSupported)
                        .context(ext_name.to_str().unwrap()))
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

        let mut pdevice_1_3_features = create_info.physical_device_1_3_features;

        device_create_info = device_create_info.push_next(&mut pdevice_1_3_features);

        let device = instance.create_device(*physical_device, &device_create_info, None)?;
        Ok(device)
    }

    pub(crate) unsafe fn create_allocator(
        instance: &Instance,
        physical_device: &PhysicalDevice,
        device: &Device,
    ) -> Result<Allocator> {
        let allocator = vk_mem_alloc::create_allocator(instance, *physical_device, device, None)?;
        Ok(allocator)
    }

    pub(crate) unsafe fn create_queues(
        device: &Device,
        physical_device_info: &PhysicalDeviceInfo,
    ) -> Result<(Queue, Option<Queue>, Option<Queue>)> {
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
        display_handle: &RawDisplayHandle,
        window_handle: &RawWindowHandle,
        window_size: [u32; 2],
        physical_device: &PhysicalDevice,
        create_info: &VkInitCreateInfo,
    ) -> Result<(Surface, SurfaceKHR, SurfaceInfo)> {
        let loader = Surface::new(entry, instance);
        let surface =
            ash_window::create_surface(entry, instance, *display_handle, *window_handle, None)?;
        let formats = loader.get_physical_device_surface_formats(*physical_device, surface)?;

        let format = *formats
            .iter()
            .find(|format| format.format == create_info.surface_format)
            .ok_or(VkInitError::RequestedSurfaceFormatNotSupported)?;

        let present_modes =
            loader.get_physical_device_surface_present_modes(*physical_device, surface)?;

        let present_mode = present_modes
            .iter()
            .copied()
            .find(|&mode| mode == create_info.present_mode)
            .ok_or(VkInitError::PresentModeNotSupported)?;

        let capabilities =
            loader.get_physical_device_surface_capabilities(*physical_device, surface)?;

        if capabilities.max_image_count != 0
            && create_info.frames_in_flight > capabilities.max_image_count
        {
            let max_frame = capabilities.max_image_count;
            trace!("max supported frames in flight: {max_frame}");
            return Err(anyhow!(VkInitError::InsufficientFramesInFlightSupported));
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
            format,
            pre_transform,
        };

        Ok((loader, surface, surface_info))
    }

    pub(crate) unsafe fn create_swapchain(
        instance: &Instance,
        device: &Device,
        surface: &SurfaceKHR,
        surface_info: &SurfaceInfo,
        window_extent: &Extent2D,
        frames_in_flight: u32,
    ) -> Result<(Swapchain, SwapchainKHR)> {
        let swapchain_create_info = SwapchainCreateInfoKHR::builder()
            .surface(*surface)
            .min_image_count(frames_in_flight)
            .image_color_space(surface_info.format.color_space)
            .image_format(surface_info.format.format)
            .image_extent(*window_extent)
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
    ) -> Result<(Vec<Image>, Vec<ImageView>)> {
        let images = swapchain_loader.get_swapchain_images(*swapchain)?;
        let mut image_views = Vec::new();
        for image in &images {
            let create_view_info = ImageViewCreateInfo::builder()
                .view_type(ImageViewType::TYPE_2D)
                .format(surface_info.format.format)
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
}

impl AsRef<Instance> for VkInit {
    fn as_ref(&self) -> &Instance {
        &self.instance
    }
}

impl AsRef<Device> for VkInit {
    fn as_ref(&self) -> &Device {
        &self.device
    }
}

impl AsRef<Allocator> for VkInit {
    fn as_ref(&self) -> &Allocator {
        &self.allocator
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
