pub(crate) use anyhow::{anyhow, Result};
pub(crate) use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};
pub(crate) use ash::util::read_spv;
pub(crate) use ash::vk::*;
pub(crate) use ash::{Device, Entry, Instance};

pub(crate) use log::{error, info, trace, warn};
pub(crate) use raw_window_handle::RawDisplayHandle;
pub(crate) use raw_window_handle::RawWindowHandle;
pub(crate) use std::{
    borrow::Cow,
    ffi::{CStr, CString},
    fs::{create_dir_all, read_dir, read_to_string, remove_dir_all},
    io::Cursor,
    mem::size_of,
    path::Path,
    str::Utf8Error,
};
pub(crate) use vk_mem_alloc::{
    Allocation, AllocationCreateFlags, AllocationCreateInfo, AllocationInfo, Allocator, MemoryUsage,
};
pub(crate) fn char_array_to_string(chars: &[i8; 256]) -> Result<String, Utf8Error> {
    let string_raw = unsafe { CStr::from_ptr(chars.as_ptr()) };
    let string = string_raw.to_str()?.to_owned();
    Ok(string)
}
pub(crate) use optick_attr::profile;
