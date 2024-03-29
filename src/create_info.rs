use crate::imports::*;

/// Creation parameters for [VkInit](crate::init::VkInit).
///
/// Windowing extensions are enabled automatically depending on the chosen platform.

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
    pub physical_device_1_1_features: PhysicalDeviceVulkan11Features,
    pub physical_device_1_2_features: PhysicalDeviceVulkan12Features,
    pub physical_device_1_3_features: PhysicalDeviceVulkan13Features,

    //Device
    pub additional_device_extensions: Vec<String>,

    //Surface
    pub surface_format: Format,
    pub depth_format: Format,
    pub depth_format_sizeof: usize,
    pub request_img_count: u32,
    pub present_mode: PresentModeKHR,
    pub clear_color_value: ClearColorValue,
    pub clear_depth_stencil_value: ClearDepthStencilValue,
}

impl VkInitCreateInfo {
    /// Suitable for debug builds against Vulkan 1.3 with all available information:
    /// - validation enabled
    /// - best practices and synchronization checks enabled
    /// - log level: all
    /// - log messages: all
    ///
    /// [DynamicRendering](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_dynamic_rendering.html),
    /// [DescriptorIndexing](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_EXT_descriptor_indexing.html),
    /// [ShaderDrawParameters](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_shader_draw_parameters.html),
    /// and [Synchronization2](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_synchronization2.html)
    /// are enabled by default.

    pub fn verbose_debug_vk_1_3() -> Self {
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
            log_level: DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | DebugUtilsMessageSeverityFlagsEXT::INFO
                | DebugUtilsMessageSeverityFlagsEXT::WARNING
                | DebugUtilsMessageSeverityFlagsEXT::ERROR,
            log_msg: DebugUtilsMessageTypeFlagsEXT::GENERAL
                | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            allow_igpu: false,
            physical_device_1_3_features: PhysicalDeviceVulkan13Features::builder()
                .synchronization2(true)
                .dynamic_rendering(true)
                .build(),
            physical_device_1_2_features: PhysicalDeviceVulkan12Features::builder()
                .descriptor_binding_sampled_image_update_after_bind(true)
                .descriptor_indexing(true)
                .build(),
            physical_device_1_1_features: PhysicalDeviceVulkan11Features::builder()
                .shader_draw_parameters(true)
                .build(),
            additional_device_extensions: vec![],
            surface_format: if cfg!(target_os = "linux") {
                Format::B8G8R8A8_UNORM
            } else {
                Format::R8G8B8A8_UNORM
            },
            depth_format: Format::D32_SFLOAT,
            depth_format_sizeof: 4,
            request_img_count: 3,
            present_mode: PresentModeKHR::FIFO,
            clear_color_value: ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
            clear_depth_stencil_value: ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        }
    }

    /// Suitable for debug builds against Vulkan 1.3:
    /// - validation enabled
    /// - best practices and synchronization checks enabled
    /// - log level: >= info
    /// - log messages: validation and performance
    pub fn debug_vk_1_3() -> Self {
        Self {
            log_level: DebugUtilsMessageSeverityFlagsEXT::INFO
                | DebugUtilsMessageSeverityFlagsEXT::WARNING
                | DebugUtilsMessageSeverityFlagsEXT::ERROR,
            ..Self::verbose_debug_vk_1_3()
        }
    }

    /// Suitable for test release builds against Vulkan 1.3:
    /// - validation enabled
    /// - synchronization checks enabled
    /// - log level: >= warn
    /// - log messages: validation and performance
    pub fn test_release_vk_1_3() -> Self {
        Self {
            log_level: DebugUtilsMessageSeverityFlagsEXT::WARNING,
            ..Self::verbose_debug_vk_1_3()
        }
    }

    /// Suitable for final release builds against Vulkan 1.3:
    /// - no validation
    /// - no logging
    pub fn dist_vk_1_3() -> Self {
        Self {
            enable_validation: false,
            enabled_validation_layers: vec![],
            enabled_validation_features: vec![],
            log_level: DebugUtilsMessageSeverityFlagsEXT::empty(),
            log_msg: DebugUtilsMessageTypeFlagsEXT::empty(),
            ..Self::verbose_debug_vk_1_3()
        }
    }
}

impl Default for VkInitCreateInfo {
    /// Default options are suitable for a debug build against Vulkan 1.3.
    fn default() -> Self {
        Self::debug_vk_1_3()
    }
}
