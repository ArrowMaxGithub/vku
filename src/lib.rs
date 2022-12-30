#![allow(dead_code)]
//! Utility crate for kickstarting vulkan development with [shaderc](https://docs.rs/shaderc/0.8.0/shaderc/index.html), [ash](https://docs.rs/ash/0.37.1+1.3.235/ash/index.html) and the [VMA](https://docs.rs/vk-mem-alloc/0.1.1/vk_mem_alloc/index.html) allocator.
//!
//! Center module is [Vkinit](crate::init::VkInit), created from [RawHandles](https://docs.rs/raw-window-handle/0.5.0/raw_window_handle/index.html) and [VkInitCreateInfo](crate::create_info::VkInitCreateInfo).
//!
//! ## Initialization
//!
//! [winit](https://docs.rs/winit/0.27.5/winit/index.html):
//! ```
//! extern crate winit;
//! use winit::window::WindowBuilder;
//! use winit::event_loop::{EventLoop, EventLoopBuilder};
//! use winit::dpi::LogicalSize;
//! use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
//! use vku::{VkInitCreateInfo, VkInit};
//!
//! let event_loop: EventLoop<()> = EventLoopBuilder::default().build();
//! let size = [800_u32, 600_u32];
//! let window = WindowBuilder::new()
//!     .with_inner_size(LogicalSize{width: size[0], height: size[1]})
//!     .build(&event_loop).unwrap();
//! let display_handle = window.raw_display_handle();
//! let window_handle = window.raw_window_handle();
//! let create_info = VkInitCreateInfo::default();
//!
//! let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
//! ```
//!
//! ## Swapchain recreation:
//! ```
//! # extern crate winit;
//! # use vku::*;
//! # use ash::vk::*;
//! # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
//! # let size = [800_u32, 600_u32];
//! # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
//! # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
//! # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
//! # let create_info = VkInitCreateInfo::default();
//! let mut init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
//!
//! let new_size = [1200_u32, 900_u32];
//! let in_flight = 3;
//!
//! init.recreate_swapchain(new_size, in_flight).unwrap();
//! ```
//!
//!  ## VMA Image allocation and layout transition:
//! ```
//!#
//! # extern crate winit;
//! # use vku::*;
//! # use ash::vk::*;
//! # let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
//! # let size = [800_u32, 600_u32];
//! # let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
//! # let display_handle = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
//! # let window_handle = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
//! # let create_info = VkInitCreateInfo::default();
//! let init = VkInit::new(&display_handle, &window_handle, size, &create_info).unwrap();
//! let setup_cmd_buffer_pool =
//!     init.create_cmd_pool(CmdType::Any).unwrap();
//! let setup_cmd_buffer =
//!     init.create_command_buffers(&setup_cmd_buffer_pool, 1).unwrap()[0];
//! let setup_fence = init.create_fence().unwrap();
//! init.begin_cmd_buffer(&setup_cmd_buffer).unwrap();
//!
//! let extent = Extent3D{width: 100, height: 100, depth: 1};
//! let format = Format::R8G8B8A8_UNORM;
//! let aspect_flags = ImageAspectFlags::COLOR;
//! let mut image = init.create_empty_image(extent, format, aspect_flags).unwrap();
//!
//! let image_barrier = image.get_image_layout_transition_barrier2(
//!     ImageLayout::TRANSFER_DST_OPTIMAL,
//!     None, None).unwrap(); // No queue family ownership transfer
//!
//! init.cmd_pipeline_barrier2(
//!     &setup_cmd_buffer,
//!     &[image_barrier], &[]); // Only this image barrier, no BufferMemoryBarriers
//!
//! init.end_and_submit_cmd_buffer(
//!     &setup_cmd_buffer,
//!     CmdType::Any,
//!     &setup_fence, &[], &[], &[]).unwrap(); // Wait on setup fence, wait/signal no semaphores
//! ```
//! ## Shader compilation with #include directives:
//! ```rust,no_run
//! # use std::path::Path;
//! # use vku::VkInit;
//! let src_dir_path = Path::new("./assets/shaders/src/");
//! let target_dir_path = Path::new("./assets/shaders/compiled_shaders/");
//! let src_glsl_path = src_dir_path.join(Path::new("example.glsl"));
//! let src_vert_path = src_dir_path.join(Path::new("example.vert"));
//! let debug_text_result = true;
//!
//! std::fs::write(&src_glsl_path, r#"
//! struct Example{
//!     float pos_x;
//!     float pos_y;
//!     float pos_z;
//!     float size;
//!     float color;
//! };"#).unwrap();
//!
//! std::fs::write(&src_vert_path, r#"
//! #version 450
//! #include "./assets/shaders/src/example.glsl" // relative path from .exe
//! layout(location = 0) in vec4 i_pos_size;
//! layout(location = 1) in vec4 i_col;
//! layout(location = 0) out vec4 o_col;
//! void main() {
//!     o_col = i_col;
//!     gl_Position = vec4(i_pos_size.xyz, 1.0);
//!     gl_PointSize  = i_pos_size.w;
//! }"#).unwrap();
//!
//! vku::compile_all_shaders(&src_dir_path, &target_dir_path, debug_text_result).unwrap();
//!```

#[macro_use]
extern crate derive_error;

mod create_info;
mod errors;
mod image_layout_transitions;
mod imports;
mod init;
mod renderer;
mod swapchain;
mod vertex;
mod vma_buffer;
mod vma_image;
mod compute_shader;

pub mod shader;

pub use ash;
pub use create_info::VkInitCreateInfo;
pub use init::{CmdType, PhysicalDeviceInfo, SurfaceInfo, VkDestroy, VkInit, VkInitInfo};
pub use renderer::{BaseRenderer, RendererBarriers, RendererCreateInfo};
pub use shader::{compile_all_shaders, reflect_spirv_shader, ReflectionResult};
pub use vertex::*;
pub use vma_buffer::VMABuffer;
pub use vma_image::VMAImage;
pub use compute_shader::ComputeShader;