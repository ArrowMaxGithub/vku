# VKU
Work in progress utility crate for kickstarting vulkan development with [shaderc](https://docs.rs/shaderc/0.8.0/shaderc/index.html), [ash](https://docs.rs/ash/0.37.1+1.3.235/ash/index.html) and the [VMA](https://docs.rs/vma/0.3.0/vma/) allocator.

Center module is [Vkinit](crate::init::VkInit), created from [RawHandles](https://docs.rs/raw-window-handle/0.5.0/raw_window_handle/index.html) and [VkInitCreateInfo](crate::create_info::VkInitCreateInfo).

## Warning
This is mostly a personal utility crate and no guarentees are made in terms of stability until 1.0.0.

## Initialization
```rust,no_run
use winit::window::WindowBuilder;
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::dpi::LogicalSize;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vku::{VkInitCreateInfo, VkInit};

let event_loop: EventLoop<()> = EventLoopBuilder::default().build();
let size = [800_u32, 600_u32];
let window = WindowBuilder::new()
    .with_inner_size(LogicalSize{width: size[0], height: size[1]})
    .build(&event_loop).unwrap();

let display_h = window.raw_display_handle();
let window_h = window.raw_window_handle();

let create_info = VkInitCreateInfo::default();
let init = VkInit::new(Some(&display_h), Some(&window_h), Some(size), create_info).unwrap();
```
## Swapchain recreation:
```rust,no_run
# use vku::*;
# use ash::vk::*;
# let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
# let size = [800_u32, 600_u32];
# let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
# let display_h = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
# let window_h = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
# let create_info = VkInitCreateInfo::default();

let mut init = VkInit::new(Some(&display_h), Some(&window_h), Some(size), create_info).unwrap();
let new_size = [1200_u32, 900_u32];
init.on_resize(&display_h, &window_h, new_size).unwrap();
```
 ## VMA Image allocation and layout transition:
```rust,no_run
# use vku::*;
# use ash::vk::*;
# let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoopBuilder::default().build();
# let size = [800_u32, 600_u32];
# let window = winit::window::WindowBuilder::new().with_inner_size(winit::dpi::LogicalSize{width: size[0], height: size[1]}).build(&event_loop).unwrap();
# let display_h = raw_window_handle::HasRawDisplayHandle::raw_display_handle(&window);
# let window_h = raw_window_handle::HasRawWindowHandle::raw_window_handle(&window);
# let create_info = VkInitCreateInfo::default();

let init = VkInit::new(Some(&display_h), Some(&window_h), Some(size), create_info).unwrap();

let setup_cmd_buffer_pool =
    init.create_cmd_pool(CmdType::Any).unwrap();

let setup_cmd_buffer =
    init.create_command_buffers(&setup_cmd_buffer_pool, 1).unwrap()[0];
    
let setup_fence = init.create_fence().unwrap();

init.begin_cmd_buffer(&setup_cmd_buffer).unwrap();

let extent = Extent3D{width: 100, height: 100, depth: 1};
let format = Format::R8G8B8A8_UNORM;
let aspect_flags = ImageAspectFlags::COLOR;
let mut image = init.create_empty_image(extent, format, aspect_flags).unwrap();

let image_barrier = image.get_image_layout_transition_barrier2(
    ImageLayout::TRANSFER_DST_OPTIMAL,
    None, None).unwrap(); // No queue family ownership transfer

init.cmd_pipeline_barrier2(
    &setup_cmd_buffer,
    &[image_barrier], &[]); // Only this image barrier, no BufferMemoryBarriers

init.end_and_submit_cmd_buffer(
    &setup_cmd_buffer,
    CmdType::Any,
    &setup_fence, &[], &[], &[]).unwrap(); // Wait on setup fence, wait/signal no semaphores
```
## Shader compilation with #include directives:
```rust,no_run
# use std::path::Path;
# use vku::VkInit;

let src_dir_path = Path::new("./assets/shaders/src/");
let target_dir_path = Path::new("./assets/shaders/compiled_shaders/");
let debug_text_result = true;

vku::compile_all_shaders(&src_dir_path, &target_dir_path, debug_text_result).unwrap();
```

## More examples
For more examples check the examples folder.