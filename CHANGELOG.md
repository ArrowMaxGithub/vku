### [0.2.0] Restructure
- Changed: All shader compilation functionality is now behind the `shader` feature to make shaderc an optional dependency.
- Changed: Switched unmaintained vk-mem-rs crate for gpu-allocator. Signature of VMABuffer and VMAImage creation have changed.
- Changed: Moved examples to separate crate: `vku-examples`.

### [0.1.2] Build script fix

### [0.1.1] Egui example patch
- Example egui renderer now features the color test window from https://www.egui.rs/#colors and passes.

## [0.1.0] Initial release