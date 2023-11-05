use gpu_allocator::vulkan::AllocationScheme;

use crate::{imports::*, VkInit};

/// VMA-allocated buffer, allocation and allocation information.
pub struct VMABuffer {
    pub buffer: Buffer,
    pub allocation: Allocation,
}

impl VMABuffer {
    fn new(
        device: &Device,
        allocator: &mut Allocator,
        buffer_info: BufferCreateInfo,
        mut allocation_create_info: AllocationCreateDesc,
    ) -> Result<Self, Error> {
        let (buffer, allocation) = unsafe {
            let buffer = device.create_buffer(&buffer_info, None)?;
            let req = device.get_buffer_memory_requirements(buffer);
            allocation_create_info.requirements = req;
            let alloc = allocator.allocate(&allocation_create_info)?;
            device.bind_buffer_memory(buffer, alloc.memory(), alloc.offset())?;
            (buffer, alloc)
        };

        Ok(Self { buffer, allocation })
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) -> Result<(), Error> {
        unsafe {
            device.destroy_buffer(self.buffer, None);
            let alloc = std::mem::take(&mut self.allocation);
            allocator.free(alloc)?;
        }
        Ok(())
    }

    pub fn set_debug_object_name(&self, vk_init: &VkInit, base_name: String) -> Result<(), Error> {
        vk_init.set_debug_object_name(
            self.buffer.as_raw(),
            ObjectType::BUFFER,
            format!("{base_name}_Buffer"),
        )?;
        vk_init.set_debug_object_name(
            unsafe { self.allocation.memory().as_raw() },
            ObjectType::DEVICE_MEMORY,
            format!("{base_name}_Memory"),
        )?;
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
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info)?;
    /// let size = 1024_usize;
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    ///
    /// let buffer = VMABuffer::create_local_buffer(&init.device, &mut init.allocator, size, usage)?;
    /// let buffer_shortcut = init.create_local_buffer(size, usage)?;
    /// # Ok::<(), vku::Error>(())

    pub fn create_local_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer, Error> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage)
            .build();

        let allocation_info = AllocationCreateDesc {
            name: "Local_Buffer_Memory",
            requirements: MemoryRequirements::default(),
            location: MemoryLocation::GpuOnly,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };

        Self::new(device, allocator, buffer_info, allocation_info)
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
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info)?;
    /// let size = 1024_usize;
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    ///
    /// let buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, usage)?;
    /// let buffer_shortcut = init.create_cpu_to_gpu_buffer(size, usage)?;
    /// # Ok::<(), vku::Error>(())

    pub fn create_cpu_to_gpu_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer, Error> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage)
            .build();

        let allocation_info = AllocationCreateDesc {
            name: "Upload_Buffer_Memory",
            requirements: MemoryRequirements::default(),
            location: MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };

        Self::new(device, allocator, buffer_info, allocation_info)
    }

    pub fn create_readback_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer, Error> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage)
            .build();

        let allocation_info = AllocationCreateDesc {
            name: "Readback_Buffer_Memory",
            requirements: MemoryRequirements::default(),
            location: MemoryLocation::GpuToCpu,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };

        Self::new(device, allocator, buffer_info, allocation_info)
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
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info)?;
    /// let size = 1024 * size_of::<usize>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = init.create_cpu_to_gpu_buffer(size, usage)?;
    ///
    /// let data = [42_usize; 1024];
    /// let offset = 0;
    /// buffer.set_data(offset, &data)?;
    /// # Ok::<(), vku::Error>(())
    /// ```

    pub fn set_data<T>(&self, offset: usize, data: &[T]) -> Result<(), Error> {
        let Some(ptr) = self.allocation.mapped_ptr() else {
            return Err(Error::WriteAttemptToUnmappedBuffer);
        };

        let mut ptr = ptr.as_ptr() as *mut T;
        unsafe {
            ptr = ptr.add(offset);
            ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());
        };
        Ok(())
    }

    /// Sets data on a mapped buffer.
    ///
    /// Buffer needs to be created in host-visible memory and mapped.
    /// Use [create_cpu_to_gpu_buffer](VMABuffer::create_cpu_to_gpu_buffer) to allocate a compatible buffer.
    ///
    /// # Valid usage:
    /// - Validate input data type to avoid misalignment on GLSL side: usize vs uint
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
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info)?;
    /// let size = 2 * size_of::<u32>() + 1024 * size_of::<f32>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = init.create_cpu_to_gpu_buffer(size, usage)?;
    ///
    /// let start_data = [4_u32, 2_u32];
    /// let data = [42.0; 1024];
    /// buffer.set_data_with_start_data(&start_data, &data)?;
    /// # Ok::<(), vku::Error>(())
    /// ```

    pub fn set_data_with_start_data<T, U>(
        &self,
        start_data: &[U],
        data: &[T],
    ) -> Result<(), Error> {
        let Some(ptr) = self.allocation.mapped_ptr() else {
            return Err(Error::WriteAttemptToUnmappedBuffer);
        };

        let ptr = ptr.as_ptr() as *mut U;
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
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info)?;
    /// # let cmd_buffer_pool =
    /// #    init.create_cmd_pool(CmdType::Any)?;
    /// # let cmd_buffer =
    /// #    init.create_command_buffers(&cmd_buffer_pool, 1)?[0];
    /// # init.begin_cmd_buffer(&cmd_buffer)?;
    /// let size = 1024 * size_of::<u32>();
    /// let src_usage = BufferUsageFlags::TRANSFER_SRC;
    /// let dst_usage = BufferUsageFlags::TRANSFER_DST;
    /// let src_buffer = init.create_cpu_to_gpu_buffer(size, src_usage)?;
    /// let dst_buffer = init.create_cpu_to_gpu_buffer(size, dst_usage)?;
    ///
    /// let data = [42_u32; 1024];
    /// let offset = 0;
    /// src_buffer.set_data(offset, &data)?;
    ///
    /// src_buffer.enqueue_copy_to_buffer(
    ///     &init.device,
    ///     &cmd_buffer,
    ///     &dst_buffer,
    ///     None,
    ///     None,
    ///     None
    ///     )?;
    /// # Ok::<(), vku::Error>(())
    /// ```

    pub fn enqueue_copy_to_buffer(
        &self,
        device: &Device,
        cmd_buffer: &CommandBuffer,
        dst_buffer: &VMABuffer,
        src_offset: Option<u64>,
        dst_offset: Option<u64>,
        size: Option<u64>,
    ) -> Result<(), Error> {
        let src_offset = src_offset.unwrap_or(0);
        let dst_offset = dst_offset.unwrap_or(0);
        let size = size.unwrap_or(self.allocation.size() - src_offset);

        let buffer_copy_region = BufferCopy::builder()
            .src_offset(src_offset)
            .dst_offset(dst_offset)
            .size(size)
            .build();

        unsafe {
            device.cmd_copy_buffer(
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
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&window), Some(size), create_info)?;
    /// let size = 1024 * size_of::<u32>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = init.create_cpu_to_gpu_buffer(size, usage)?;
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
    /// # Ok::<(), vku::Error>(())
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
        let size = size.unwrap_or(self.allocation.size());

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
    pub fn create_local_buffer(
        &mut self,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer, Error> {
        VMABuffer::create_local_buffer(&self.device, &mut self.allocator, size, usage)
    }

    /// Shortcut - see [VMABuffer](VMABuffer::create_cpu_to_gpu_buffer) for example.
    pub fn create_cpu_to_gpu_buffer(
        &mut self,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer, Error> {
        VMABuffer::create_cpu_to_gpu_buffer(&self.device, &mut self.allocator, size, usage)
    }

    pub fn create_readback_buffer(
        &mut self,
        size: usize,
        usage: BufferUsageFlags,
    ) -> Result<VMABuffer, Error> {
        VMABuffer::create_readback_buffer(&self.device, &mut self.allocator, size, usage)
    }

    /// Shortcut - see [VMABuffer](VMABuffer::create_local_buffer) for example.
    pub fn create_local_buffers(
        &mut self,
        size: usize,
        usage: BufferUsageFlags,
        count: usize,
    ) -> Result<Vec<VMABuffer>, Error> {
        let mut buffers = Vec::new();
        for _ in 0..count {
            let buffer =
                VMABuffer::create_local_buffer(&self.device, &mut self.allocator, size, usage)?;
            buffers.push(buffer);
        }
        Ok(buffers)
    }

    /// Shortcut - see [VMABuffer](VMABuffer::create_cpu_to_gpu_buffer) for example.
    pub fn create_cpu_to_gpu_buffers(
        &mut self,
        size: usize,
        usage: BufferUsageFlags,
        count: usize,
    ) -> Result<Vec<VMABuffer>, Error> {
        let mut buffers = Vec::new();
        for _ in 0..count {
            let buffer = VMABuffer::create_cpu_to_gpu_buffer(
                &self.device,
                &mut self.allocator,
                size,
                usage,
            )?;
            buffers.push(buffer);
        }
        Ok(buffers)
    }
}
