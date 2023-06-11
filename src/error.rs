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
    Preprocess(shaderc::Error),

    #[error("incorrect usage of the vulkan API: {0}")]
    VkError(ash::vk::Result),

    #[error("encountered an unknown error: {0}")]
    Catch(Box<dyn std::error::Error>),
}

#[cfg(feature = "shader")]
impl From<shaderc::Error> for Error {
    fn from(value: shaderc::Error) -> Self {
        Self::Preprocess(value)
    }
}

impl From<Utf8Error> for Error {
    fn from(value: Utf8Error) -> Self {
        Self::Catch(Box::new(value))
    }
}

impl From<NulError> for Error {
    fn from(value: NulError) -> Self {
        Self::Catch(Box::new(value))
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Catch(Box::new(value))
    }
}

impl From<ash::vk::Result> for Error {
    fn from(value: ash::vk::Result) -> Self {
        Self::VkError(value)
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Catch(value)
    }
}
