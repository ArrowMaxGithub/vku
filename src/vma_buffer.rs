use crate::{imports::*, VkInit};

/// VMA-allocated buffer, allocation and allocation information.
pub struct VMABuffer {
    pub buffer: Buffer,
    pub allocation: Allocation,
    pub allocation_info: AllocationInfo,
    pub is_mapped: bool,
}

impl VMABuffer {
    fn new(
        allocator: &Allocator,
        buffer_info: &BufferCreateInfo,
        allocation_create_info: &AllocationCreateInfo,
    ) -> Result<Self> {
        let (buffer, allocation, allocation_info) = unsafe {
            vk_mem_alloc::create_buffer(*allocator, buffer_info, allocation_create_info)?
        };

        let is_mapped = allocation_create_info
            .flags
            .contains(AllocationCreateFlags::MAPPED);

        Ok(Self {
            buffer,
            allocation,
            allocation_info,
            is_mapped,
        })
    }

    pub fn destroy(&self, allocator: &Allocator) -> Result<()> {
        unsafe {
            vk_mem_alloc::destroy_buffer(*allocator, self.buffer, self.allocation);
        }
        Ok(())
    }

    /// Creates and allocates a buffer of the requested size.
    ///
    /// Preferred to be device-local.
    /// ```
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
    /// let size = 1024_usize;
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    ///
    /// let buffer = VMABuffer::create_local_buffer(&init.allocator, size, usage).unwrap();
    /// let buffer_shortcut = init.create_local_buffer(size, usage).unwrap();
    pub fn create_local_buffer(
        allocator: &Allocator,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage);

        let allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AUTO_PREFER_DEVICE,
            ..Default::default()
        };

        Self::new(allocator, &buffer_info, &allocation_info)
    }

    /// Creates, allocates and maps a buffer of the requested size.
    ///
    /// Guarenteed to be host-visible.
    /// Preferred to be device-local
    /// ```
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
    /// let size = 1024_usize;
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    ///
    /// let buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.allocator, size, usage).unwrap();
    /// let buffer_shortcut = init.create_cpu_to_gpu_buffer(size, usage).unwrap();
    pub fn create_cpu_to_gpu_buffer(
        allocator: &Allocator,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage);

        let allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::AUTO_PREFER_DEVICE,
            flags: AllocationCreateFlags::MAPPED
                | AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            ..Default::default()
        };

        Self::new(allocator, &buffer_info, &allocation_info)
    }

    /// Sets data on a mapped buffer.
    ///
    /// Buffer needs to be created in host-visible memory and mapped.
    /// Use [create_cpu_to_gpu_buffer](VMABuffer::create_cpu_to_gpu_buffer) to allocate a compatible buffer.
    /// ```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # use std::mem::size_of;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
    /// # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
    /// # let create_info = VkInitCreateInfo::default();
    /// let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
    /// let size = 1024 * size_of::<usize>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = init.create_cpu_to_gpu_buffer(size, usage).unwrap();
    ///
    /// let data = [42_usize; 1024];
    /// buffer.set_data(&data).unwrap();
    /// ```
    pub fn set_data<T>(&self, data: &[T]) -> Result<()> {
        if !self.is_mapped {
            return Err(anyhow!("Tried to set_data on an unmapped buffer"));
        }

        let ptr = self.allocation_info.mapped_data as *mut T;
        unsafe {
            ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        };
        Ok(())
    }

    /// Sets data on a mapped buffer.
    ///
    /// Buffer needs to be created in host-visible memory and mapped.
    /// Use [create_cpu_to_gpu_buffer](VMABuffer::create_cpu_to_gpu_buffer) to allocate a compatible buffer.
    ///
    /// ### Examples
    /// Useful for GLSL buffers with some starting data and any unbound data thereafter:
    ///
    /// ```glsl
    /// layout (set = 0, binding = 0) buffer MyBuffer{
    ///     uint foo;
    ///     uint bar;     
    ///     float data[];     
    /// } my_buffer;
    ///
    /// ```
    /// ```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # use std::mem::size_of;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
    /// # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
    /// # let create_info = VkInitCreateInfo::default();
    /// let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
    /// let size = 2 * size_of::<u32>() + 1024 * size_of::<f32>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = init.create_cpu_to_gpu_buffer(size, usage).unwrap();
    ///
    /// let start_data = [4_u32, 2_u32];
    /// let data = [42.0; 1024];
    /// buffer.set_data_with_start_data(&start_data, &data).unwrap();
    /// ```
    pub fn set_data_with_start_data<T, U>(&self, start_data: &[U], data: &[T]) -> Result<()> {
        let ptr = self.allocation_info.mapped_data as *mut U;
        unsafe {
            ptr.copy_from_nonoverlapping(start_data.as_ptr(), start_data.len());

            let offset_ptr = ptr.add(start_data.len()) as *mut T;
            offset_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        };
        Ok(())
    }

    /// Enqueues a cmd_copy_buffer from this buffer to dst_buffer.
    ///
    /// No barriers are issued.
    /// Bounds are unchecked.
    ///
    ///  **Defaults**:
    /// - src_offset: 0.
    /// - dst_offset: 0.     
    /// - size: full size of this buffer.
    ///
    ///```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # use std::mem::size_of;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
    /// # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
    /// # let create_info = VkInitCreateInfo::default();
    /// let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
    /// # let cmd_buffer_pool =
    /// #    init.create_cmd_pool(CmdType::Any).unwrap();
    /// # let cmd_buffer =
    /// #    init.create_command_buffers(&cmd_buffer_pool, 1).unwrap()[0];
    /// # init.begin_cmd_buffer(&cmd_buffer).unwrap();
    /// let size = 1024 * size_of::<u32>();
    /// let src_usage = BufferUsageFlags::TRANSFER_SRC;
    /// let dst_usage = BufferUsageFlags::TRANSFER_DST;
    /// let src_buffer = init.create_cpu_to_gpu_buffer(size, src_usage).unwrap();
    /// let dst_buffer = init.create_cpu_to_gpu_buffer(size, dst_usage).unwrap();
    ///
    /// let data = [42_u32; 1024];
    /// src_buffer.set_data(&data).unwrap();
    ///
    /// src_buffer.enqueue_copy_to_buffer(
    ///     &init,
    ///     &cmd_buffer,
    ///     &dst_buffer,
    ///     None,
    ///     None,
    ///     None
    ///     ).unwrap();
    /// ```
    pub fn enqueue_copy_to_buffer<D: AsRef<Device>>(
        &self,
        device: D,
        cmd_buffer: &CommandBuffer,
        dst_buffer: &VMABuffer,
        src_offset: Option<u64>,
        dst_offset: Option<u64>,
        size: Option<u64>,
    ) -> Result<()> {
        let src_offset = src_offset.unwrap_or(0);
        let dst_offset = dst_offset.unwrap_or(0);
        let size = size.unwrap_or(self.allocation_info.size - src_offset);

        let buffer_copy_region = BufferCopy::builder()
            .src_offset(src_offset)
            .dst_offset(dst_offset)
            .size(size)
            .build();

        unsafe {
            device.as_ref().cmd_copy_buffer(
                *cmd_buffer,
                self.buffer,
                dst_buffer.buffer,
                &[buffer_copy_region],
            );
        }

        Ok(())
    }

    /// Returns a barrier2 for this buffer.
    ///
    ///  **Defaults:**
    /// - src_queue: 0.
    /// - dst_queue: 0.     
    /// - size: full size of this buffer.
    ///```
    /// # extern crate winit;
    /// # use vku::*;
    /// # use ash::vk::*;
    /// # use std::mem::size_of;
    /// # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
    /// # let size = [800_u32, 600_u32];
    /// # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
    /// # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
    /// # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
    /// # let create_info = VkInitCreateInfo::default();
    /// let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
    /// let size = 1024 * size_of::<u32>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = init.create_cpu_to_gpu_buffer(size, usage).unwrap();
    ///
    /// let barrier2 = buffer.get_barrier2(
    ///     PipelineStageFlags2::HOST,
    ///     PipelineStageFlags2::FRAGMENT_SHADER,
    ///     AccessFlags2::HOST_WRITE,
    ///     AccessFlags2::SHADER_READ,
    ///     None,
    ///     None,
    ///     None
    ///     );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn get_barrier2(
        &self,
        src_stage: PipelineStageFlags2,
        dst_stage: PipelineStageFlags2,
        src_access: AccessFlags2,
        dst_access: AccessFlags2,
        src_queue: Option<u32>,
        dst_queue: Option<u32>,
        size: Option<u64>,
    ) -> BufferMemoryBarrier2 {
        let src_queue = src_queue.unwrap_or(0);
        let dst_queue = dst_queue.unwrap_or(0);
        let size = size.unwrap_or(self.allocation_info.size);

        BufferMemoryBarrier2::builder()
            .buffer(self.buffer)
            .src_stage_mask(src_stage)
            .dst_stage_mask(dst_stage)
            .src_access_mask(src_access)
            .dst_access_mask(dst_access)
            .src_queue_family_index(src_queue)
            .dst_queue_family_index(dst_queue)
            .size(size)
            .build()
    }
}

impl VkInit {
    /// Shortcut - see [VMABuffer](VMABuffer::create_local_buffer) for example.
    pub fn create_local_buffer(&self, size: usize, usage: BufferUsageFlags) -> Result<VMABuffer> {
        VMABuffer::create_local_buffer(&self.allocator, size, usage)
    }
    /// Shortcut - see [VMABuffer](VMABuffer::create_cpu_to_gpu_buffer) for example.
    pub fn create_cpu_to_gpu_buffer(
        &self,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer> {
        VMABuffer::create_cpu_to_gpu_buffer(&self.allocator, size, usage)
    }

    /// Shortcut - see [VMABuffer](VMABuffer::create_local_buffer) for example.
    pub fn create_local_buffers(
        &self,
        size: usize,
        usage: BufferUsageFlags,
        count: usize,
    ) -> Result<Vec<VMABuffer>> {
        let mut buffers = Vec::new();
        for _ in 0..count {
            let buffer = VMABuffer::create_local_buffer(&self.allocator, size, usage)?;
            buffers.push(buffer);
        }
        Ok(buffers)
    }
    /// Shortcut - see [VMABuffer](VMABuffer::create_cpu_to_gpu_buffer) for example.
    pub fn create_cpu_to_gpu_buffers(
        &self,
        size: usize,
        usage: BufferUsageFlags,
        count: usize,
    ) -> Result<Vec<VMABuffer>> {
        let mut buffers = Vec::new();
        for _ in 0..count {
            let buffer = VMABuffer::create_cpu_to_gpu_buffer(&self.allocator, size, usage)?;
            buffers.push(buffer);
        }
        Ok(buffers)
    }
}
