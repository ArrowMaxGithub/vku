### [0.3.0] Unreleased
- Changed: HasRawWindowHandle and HasRawDisplayHandle are now trait bounds on VkInit functions.
- Changed: env_logger only as dev-dep.
- Changed: PhysicalDevice now reports all property limits.
- Added: Element offset for set_data on VMABuffer.
- Added: Update after bind descriptors for pipeline builder.
- Added: Crate features for linked or loaded Ash entry (default: loaded).

### [0.2.0] Restructure
- Changed: All shader compilation functionality is now behind the `shader` feature to make shaderc an optional dependency.
- Changed: Moved examples to separate crate: `vku-examples`.
- Changed: Switched to vma crate for VMA implementation.
- Added: More layout transitions, render texture creation helper.
- Added: PipelineBuilder - see [integration test](tests/pipeline_builder.rs) for an example. 

### [0.1.2] Build script fix

### [0.1.1] Egui example patch
- Example egui renderer now features the color test window from https://www.egui.rs/#colors and passes.

## [0.1.0] Initial release