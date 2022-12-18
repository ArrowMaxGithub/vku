use crate::imports::*;

/// Creation parameters for [VkInit](crate::init::VkInit).
///
/// Windowing extensions are enabled automatically depending on the chosen platform.
///
/// [Dynamic rendering](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_dynamic_rendering.html)
/// and [Synchronization2](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_synchronization2.html)
/// are enabled by default due to being awesome extensions.
pub struct VkInitCreateInfo {
    pub app_name: String,
    pub engine_name: String,
    pub app_version: u32,
    pub vk_version: u32,

    //Instance
    pub enable_validation: bool,
    pub enabled_validation_layers: Vec<String>,
    pub enabled_validation_features: Vec<ValidationFeatureEnableEXT>,
    pub additional_instance_extensions: Vec<String>,
    pub log_level: DebugUtilsMessageSeverityFlagsEXT,
    pub log_msg: DebugUtilsMessageTypeFlagsEXT,

    //PhysicalDevice
    pub allow_igpu: bool,
    pub physical_device_1_3_features: PhysicalDeviceVulkan13Features,

    //Device
    pub additional_device_extensions: Vec<String>,

    //Surface
    pub surface_format: Format,
    pub frames_in_flight: u32,
    pub present_mode: PresentModeKHR,
}

impl VkInitCreateInfo {
    /// Suitable for debug builds against Vulkan 1.3:
    /// - validation enalbed
    /// - best practices and synchronization checks enabled
    /// - log level: >= warning
    /// - log messages: validation and performance
    ///
    /// Synchronization2 and dynamic rendering extensions enabled by default.
    pub fn debug_vk_1_3() -> Self {
        Self {
            app_name: String::from("Default app name"),
            engine_name: String::from("Default engine name"),
            app_version: make_api_version(0, 0, 0, 1),
            vk_version: API_VERSION_1_3,
            enable_validation: true,
            enabled_validation_layers: vec![String::from("VK_LAYER_KHRONOS_validation")],
            enabled_validation_features: vec![
                ValidationFeatureEnableEXT::BEST_PRACTICES,
                ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
            ],
            additional_instance_extensions: vec![],
            log_level: DebugUtilsMessageSeverityFlagsEXT::WARNING
                | DebugUtilsMessageSeverityFlagsEXT::ERROR,
            log_msg: DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            allow_igpu: false,
            physical_device_1_3_features: PhysicalDeviceVulkan13Features::builder()
                .synchronization2(true)
                .dynamic_rendering(true)
                .build(),
            additional_device_extensions: vec![],
            surface_format: if cfg!(target_os = "linux") {
                Format::B8G8R8A8_UNORM
            } else {
                Format::R8G8B8A8_UNORM
            },
            frames_in_flight: 3,
            present_mode: PresentModeKHR::IMMEDIATE,
        }
    }

    /// Suitable for release builds against Vulkan 1.3:
    /// - no validation
    /// - no logging
    ///
    /// Synchronization2 and dynamic rendering extensions enabled by default.
    pub fn release_vk_1_3() -> Self {
        Self {
            app_name: String::from("Default app name"),
            engine_name: String::from("Default engine name"),
            app_version: make_api_version(0, 0, 0, 1),
            vk_version: API_VERSION_1_3,
            enable_validation: false,
            enabled_validation_layers: vec![],
            enabled_validation_features: vec![],
            additional_instance_extensions: vec![],
            log_level: DebugUtilsMessageSeverityFlagsEXT::empty(),
            log_msg: DebugUtilsMessageTypeFlagsEXT::empty(),
            allow_igpu: false,
            physical_device_1_3_features: PhysicalDeviceVulkan13Features::builder()
                .synchronization2(true)
                .dynamic_rendering(true)
                .build(),
            additional_device_extensions: vec![],
            surface_format: if cfg!(target_os = "linux") {
                Format::B8G8R8A8_UNORM
            } else {
                Format::R8G8B8A8_UNORM
            },
            frames_in_flight: 3,
            present_mode: PresentModeKHR::IMMEDIATE,
        }
    }
}

impl Default for VkInitCreateInfo {
    /// Default options are suitable for a debug build against Vulkan 1.3 with dynamic rendering and syncronization2 enabled.
    fn default() -> Self {
        Self::debug_vk_1_3()
    }
}
