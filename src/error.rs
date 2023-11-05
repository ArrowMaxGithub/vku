use std::{ffi::NulError, str::Utf8Error};
use thiserror::Error;

unsafe impl Send for Error {}
unsafe impl Sync for Error {}

#[derive(Error, Debug)]
pub enum Error {
    #[error("no suitable GPU was found to create the physical device")]
    NoSuitableGPUFound,
    #[error("device extension was requested but is not supported. Extension: {0}")]
    RequiredDeviceExtensionNotSupported(String),
    #[error("requested surface format is not supported by the surface")]
    RequestedSurfaceFormatNotSupported,
    #[error("more frames in flight were requested than the surface supports")]
    InsufficientFramesInFlightSupported,
    #[error("requested present mode is not supported by the surface")]
    PresentModeNotSupported,

    #[error("the requested image layout transition is not supported")]
    UnsupportedImageLayoutTransition,
    #[error("tried to set data on an unmapped buffer")]
    WriteAttemptToUnmappedBuffer,

    #[error("the file extension of the shader could not be handled")]
    UnknownShaderFileExtension,

    #[cfg(feature = "shader")]
    #[error("shader compilation failed, see preprocess trace above. Source error: {0}")]
    Preprocess(#[from] shaderc::Error),

    #[error("incorrect usage of the vulkan API: {0}")]
    VkError(#[from] ash::vk::Result),

    #[error("vulkan entry could not be loaded: {0}")]
    AshLoadError(#[from] ash::LoadingError),

    #[error("encountered an unknown error: {0}")]
    Catch(Box<dyn std::error::Error>),

    #[error("utf8 error: {0}")]
    Utf8Error(#[from] Utf8Error),

    #[error("cstring convert error: {0}")]
    CStringConvertError(#[from] NulError),

    #[error("io error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("gpu allocation error: {0}")]
    GpuAllocError(#[from] gpu_allocator::AllocationError),
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Catch(value)
    }
}
