#[derive(Debug, Error)]
pub enum VkInitError {
    ///No suitable GPU was found to create the physical device
    NoSuitableGPUFound,
    ///A unified device queue was requested but the physical device does not support it
    RequestedUnifiedQueueNotSupported,
    ///Device extension was requested but is not supported    
    RequiredDeviceExtensionNotSupported,
    ///Requested surface format is not supported by the surface
    RequestedSurfaceFormatNotSupported,
    ///More frames in flight were requested than the surface supports
    InsufficientFramesInFlightSupported,
    ///Requested present mode is not supported by the surface
    PresentModeNotSupported,
}

#[derive(Debug, Error)]
pub enum ImageLayoutTransitionError {
    ///The requested image layout transition is not supported
    UnsupportedImageLayoutTransition,
}

#[derive(Debug, Error)]
pub enum ShaderCompilationError {
    ///The file extension of the shader could not be handled
    UnknownShaderFileExtension,
}
