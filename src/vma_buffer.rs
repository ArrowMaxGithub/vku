use crate::{imports::*, VkInit};
use gpu_allocator::MemoryLocation;

/// VMA-allocated buffer, allocation and allocation information.
pub struct VMABuffer {
    pub buffer: Buffer,
    pub allocation: Allocation,
    pub is_mapped: bool,
}

impl VMABuffer {
    fn new(
        device: &Device,
        allocator: &mut Allocator,
        buffer_info: &BufferCreateInfo,
        location: MemoryLocation,
        name: &str,
    ) -> Result<Self, Error> {
        let buffer = unsafe { device.create_buffer(buffer_info, None)? };
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation_create_info = AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        };
        let allocation = allocator.allocate(&allocation_create_info)?;

        let is_mapped = match location {
            MemoryLocation::CpuToGpu => {
                unsafe { device.bind_buffer_memory(buffer, allocation.memory(), 0)? };
                true
            }
            MemoryLocation::GpuToCpu => {
                unsafe { device.bind_buffer_memory(buffer, allocation.memory(), 0)? };
                true
            }
            _ => false,
        };

        Ok(Self {
            buffer,
            allocation,
            is_mapped,
        })
    }

    pub fn destroy(self, device: &Device, allocator: &mut Allocator) -> Result<(), Error> {
        unsafe {
            allocator.free(self.allocation)?;
            device.destroy_buffer(self.buffer, None);
        }
        Ok(())
    }

    pub fn set_debug_object_name(&self, vk_init: &VkInit, base_name: String) -> Result<(), Error> {
        vk_init.set_debug_object_name(
            self.buffer.as_raw(),
            ObjectType::BUFFER,
            format!("{base_name}_Buffer"),
        )?;
        unsafe {
            vk_init.set_debug_object_name(
                self.allocation.memory().as_raw(),
                ObjectType::DEVICE_MEMORY,
                format!("{base_name}_Memory"),
            )?;
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
    /// let mut init = VkInit::new(Some(&display_handle), Some(&window_handle), Some(size), create_info).unwrap();
    /// let size = 1024_usize;
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    ///
    /// let buffer = VMABuffer::create_local_buffer(&init.device, &mut init.allocator, size, usage, "buffer").unwrap();

    pub fn create_local_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: usize,
        usage: BufferUsageFlags,
        name: &str,
    ) -> Result<VMABuffer, Error> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage);

        let location = MemoryLocation::GpuOnly;

        Self::new(device, allocator, &buffer_info, location, name)
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
    /// let mut init = VkInit::new(Some(&display_handle), Some(&window_handle), Some(size), create_info).unwrap();
    /// let size = 1024_usize;
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    ///
    /// let buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, usage, "name").unwrap();

    pub fn create_cpu_to_gpu_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: usize,
        usage: BufferUsageFlags,
        name: &str,
    ) -> Result<VMABuffer, Error> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage);

        let location = MemoryLocation::CpuToGpu;

        Self::new(device, allocator, &buffer_info, location, name)
    }

    pub fn create_readback_buffer(
        device: &Device,
        allocator: &mut Allocator,
        size: usize,
        usage: BufferUsageFlags,
        name: &str,
    ) -> Result<VMABuffer, Error> {
        let buffer_info = BufferCreateInfo::builder()
            .size(size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .usage(usage);

        let location = MemoryLocation::GpuToCpu;

        Self::new(device, allocator, &buffer_info, location, name)
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
    /// let mut init = VkInit::new(Some(&display_handle), Some(&window_handle), Some(size), create_info).unwrap();
    /// let size = 1024 * size_of::<usize>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, usage, "buffer").unwrap();
    ///
    /// let data = [42_usize; 1024];
    /// buffer.set_data(&data).unwrap();
    /// ```

    pub fn set_data<T>(&self, data: &[T]) -> Result<(), Error> {
        if !self.is_mapped {
            return Err(Error::WriteAttemptToUnmappedBuffer);
        }

        let Some(ptr) = self.allocation.mapped_ptr() else {
            return Err(Error::WriteAttemptToUnmappedBuffer);
        };

        unsafe {
            let ptr = ptr.as_ptr() as *mut T;
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
    /// # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
    /// # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
    /// # let create_info = VkInitCreateInfo::default();
    /// let mut init = VkInit::new(Some(&display_handle), Some(&window_handle), Some(size), create_info).unwrap();
    /// let size = 2 * size_of::<u32>() + 1024 * size_of::<f32>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, usage, "buffer").unwrap();
    ///
    /// let start_data = [4_u32, 2_u32];
    /// let data = [42.0; 1024];
    /// buffer.set_data_with_start_data(&start_data, &data).unwrap();
    /// ```

    pub fn set_data_with_start_data<T, U>(
        &self,
        start_data: &[U],
        data: &[T],
    ) -> Result<(), Error> {
        if !self.is_mapped {
            return Err(Error::WriteAttemptToUnmappedBuffer);
        }

        let Some(ptr) = self.allocation.mapped_ptr() else {
            return Err(Error::WriteAttemptToUnmappedBuffer);
        };

        unsafe {
            let ptr = ptr.as_ptr() as *mut U;
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
    /// let mut init = VkInit::new(Some(&display_handle), Some(&window_handle), Some(size), create_info).unwrap();
    /// # let cmd_buffer_pool =
    /// #    init.create_cmd_pool(CmdType::Any).unwrap();
    /// # let cmd_buffer =
    /// #    init.create_command_buffers(&cmd_buffer_pool, 1).unwrap()[0];
    /// # init.begin_cmd_buffer(&cmd_buffer).unwrap();
    /// let size = 1024 * size_of::<u32>();
    /// let src_usage = BufferUsageFlags::TRANSFER_SRC;
    /// let dst_usage = BufferUsageFlags::TRANSFER_DST;
    /// let src_buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, src_usage, "src_buffer").unwrap();
    /// let dst_buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, dst_usage, "dst_buffer").unwrap();
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
    /// let mut init = VkInit::new(Some(&display_handle), Some(&window_handle), Some(size), create_info).unwrap();
    /// let size = 1024 * size_of::<u32>();
    /// let usage = BufferUsageFlags::STORAGE_BUFFER;
    /// let buffer = VMABuffer::create_cpu_to_gpu_buffer(&init.device, &mut init.allocator, size, usage, "buffer").unwrap();
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
