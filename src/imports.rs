pub(crate) use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};
pub(crate) use ash::vk::*;
pub(crate) use ash::{Device, Entry, Instance};

pub(crate) use log::{error, info, trace, warn};
pub(crate) use raw_window_handle::RawDisplayHandle;
pub(crate) use raw_window_handle::RawWindowHandle;
pub(crate) use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    mem::size_of,
    result::Result,
};
pub(crate) use vma::{
    Allocation, AllocationCreateFlags, AllocationCreateInfo, AllocationInfo, Allocator, MemoryUsage,
};
pub(crate) fn char_array_to_string(chars: &[i8; 256]) -> Result<String, Error> {
    let string_raw = unsafe { CStr::from_ptr(chars.as_ptr()) };
    let string = string_raw.to_str()?;
    Ok(string.to_string())
}
pub(crate) use crate::error::*;
