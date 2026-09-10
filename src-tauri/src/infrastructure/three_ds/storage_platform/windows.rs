use super::super::provisioning::SetupError;
use std::{
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
#[repr(C)]
struct FileInfo {
    attributes: u32,
    created: [u32; 2],
    accessed: [u32; 2],
    written: [u32; 2],
    volume: u32,
    size_high: u32,
    size_low: u32,
    links: u32,
    index_high: u32,
    index_low: u32,
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetFileInformationByHandle(handle: *mut std::ffi::c_void, info: *mut FileInfo) -> i32;
}
pub fn fingerprint(path: &Path) -> Result<String, SetupError> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x02000000)
        .open(path)
        .map_err(|_| SetupError::InvalidCard)?;
    let mut info = std::mem::MaybeUninit::<FileInfo>::uninit();
    // The handle remains alive; the API writes the complete fixed-layout struct.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
        return Err(SetupError::InvalidCard);
    }
    let info = unsafe { info.assume_init() };
    Ok(format!(
        "{}:{}:{}",
        info.volume, info.index_high, info.index_low
    ))
}
