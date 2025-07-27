use crate::Error;

#[derive(Default)]
pub(crate) struct FnLoader {
    pub(crate) debug_utils_instance: Option<ash::ext::debug_utils::Instance>,
    pub(crate) debug_utils_device: Option<ash::ext::debug_utils::Device>,
    pub(crate) swapchain_instance: Option<ash::khr::swapchain::Instance>,
    pub(crate) swapchain_device: Option<ash::khr::swapchain::Device>,
    pub(crate) surface_instance: Option<ash::khr::surface::Instance>,
}

impl FnLoader {
    pub(crate) fn debug_utils_instance(&self) -> Result<&ash::ext::debug_utils::Instance, Error> {
        self.debug_utils_instance
            .as_ref()
            .ok_or(Error::FnLoaderNotInitialized(
                "debug utils instance".to_string(),
            ))
    }
    pub(crate) fn debug_utils_device(&self) -> Result<&ash::ext::debug_utils::Device, Error> {
        self.debug_utils_device
            .as_ref()
            .ok_or(Error::FnLoaderNotInitialized(
                "debug utils device".to_string(),
            ))
    }
    pub(crate) fn swapchain_device(&self) -> Result<&ash::khr::swapchain::Device, Error> {
        self.swapchain_device
            .as_ref()
            .ok_or(Error::FnLoaderNotInitialized(
                "swapchain device".to_string(),
            ))
    }
    pub(crate) fn surface_instance(&self) -> Result<&ash::khr::surface::Instance, Error> {
        self.surface_instance
            .as_ref()
            .ok_or(Error::FnLoaderNotInitialized(
                "surface instance".to_string(),
            ))
    }
}
