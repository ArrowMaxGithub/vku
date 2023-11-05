[<img alt="Crates.io" src="https://img.shields.io/crates/v/vku">](https://crates.io/crates/vku)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/vku">](https://docs.rs/vku/latest/vku/)
<img alt="Crates.io" src="https://img.shields.io/crates/l/vku">

# VKU
Work in progress utility crate for kickstarting vulkan development with [shaderc](https://docs.rs/shaderc/), [ash](https://docs.rs/ash/) and the [gpu-allocator](https://docs.rs/gpu-allocator/).

Center module is [Vkinit](crate::init::VkInit), created from [RawHandles](https://docs.rs/raw-window-handle/) and [VkInitCreateInfo](crate::create_info::VkInitCreateInfo).

## Warning
This is mostly a personal utility crate and no guarentees are made in terms of stability until 1.0.0.

## Initialization
```rust,no_run
use winit::window::WindowBuilder;
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::dpi::LogicalSize;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vku::{VkInitCreateInfo, VkInit};

fn main() -> Result<(), vku::Error>{
    let event_loop: EventLoop<()> = EventLoopBuilder::default().build();
    let size = [800_u32, 600_u32];
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize{width: size[0], height: size[1]})
        .build(&event_loop)
        .unwrap();

    let create_info = VkInitCreateInfo::default();
    let vk_init = VkInit::new(Some(&window), Some(size), create_info)?;
    Ok(())
}
```

## Swapchain recreation:
```rust,no_run
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use vku::VkInit;
use winit::window::Window;
fn main() -> Result<(), vku::Error>{
    let window: Window = todo!();
    let mut vk_init: VkInit = todo!();
    
    let new_size = [1200_u32, 900_u32];
    
    vk_init.on_resize(&window, new_size)?;
    Ok(())
}
```

 ## Managed Image allocation and tracked layout transitions:
```rust,no_run
use vku::{VkInit, CmdType};
use ash::vk::{Extent3D, Format, ImageAspectFlags, ImageLayout};
fn main() -> Result<(), vku::Error>{
    let vk_init: VkInit = todo!();
    
    let setup_cmd_buffer_pool =
        vk_init.create_cmd_pool(CmdType::Any)?;
    let setup_cmd_buffer =
        vk_init.create_command_buffers(&setup_cmd_buffer_pool, 1)?[0];
    let setup_fence = vk_init.create_fence()?;
    
    vk_init.begin_cmd_buffer(&setup_cmd_buffer)?;
    
    let extent = Extent3D{width: 100, height: 100, depth: 1};
    let format = Format::R8G8B8A8_UNORM;
    let format_bytes = 4;
    let aspect_flags = ImageAspectFlags::COLOR;
    let mut image = vk_init.create_empty_image(extent, format, format_bytes, aspect_flags)?;
    
    let image_barrier = image.get_image_layout_transition_barrier2(
        ImageLayout::TRANSFER_DST_OPTIMAL,
        None, None)?; // No queue family ownership transfer
    
    vk_init.cmd_pipeline_barrier2(
        &setup_cmd_buffer,
        &[image_barrier], &[]); // Only this image barrier, no BufferMemoryBarriers
    
    vk_init.end_and_submit_cmd_buffer(
        &setup_cmd_buffer,
        CmdType::Any,
        &setup_fence, &[], &[], &[])?; // Wait on setup fence, wait/signal no semaphores
    Ok(())
}
```

## Shader compilation with #include directives:
```rust,no_run
fn main() -> Result<(), vku::Error>{
    let src_dir_path = std::path::Path::new("./assets/shaders/src/");
    let target_dir_path = std::path::Path::new("./assets/shaders/compiled_shaders/");
    let debug_text_result = true;
    
    vku::compile_all_shaders(&src_dir_path, &target_dir_path, debug_text_result)?;
    Ok(())
}
```

## More examples
For more examples check the [examples repo](https://github.com/ArrowMaxGithub/vku-examples).