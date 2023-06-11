#![doc = include_str!("../README.md")]

mod compute_shader;
mod create_info;
mod error;
mod image_layout_transitions;
mod imports;
mod init;
mod renderer;
mod shader;
mod swapchain;
mod vma_buffer;
mod vma_image;

pub use ash;
pub use compute_shader::ComputeShader;
pub use create_info::VkInitCreateInfo;
pub use error::Error;
pub use init::{CmdType, PhysicalDeviceInfo, SurfaceInfo, VkInit};
pub use renderer::{
    BaseRenderer, BlendMode, DepthTest, RendererCreateInfo, SampleMode, VertexConvert,
};
#[cfg(feature = "shader")]
pub use shader::{compile_all_shaders, shader_ad_hoc};
pub use vma_buffer::VMABuffer;
pub use vma_image::VMAImage;
