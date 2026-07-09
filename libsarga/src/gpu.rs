//! Graphics Hardware Acceleration (DRM/GEM).

use crate::syscall::*;
use crate::errno::Error;

/// System call number for DRM control.
pub const SYS_DRMCTL: u64 = 400;

/// Command to get display information.
pub const DRM_GET_DISPLAY_INFO: u64 = 0x0100;
/// Command to create a dumb buffer.
pub const DRM_CREATE_DUMB: u64     = 0x0101;
/// Command to destroy a dumb buffer.
pub const DRM_DESTROY_DUMB: u64   = 0x0103;
/// Command to flip the front and back buffers.
pub const DRM_FLIP: u64           = 0x0104;
/// Command to set the display mode.
pub const DRM_SET_MODE: u64       = 0x0105;
/// Command to map a dumb buffer into memory.
pub const DRM_MAP_DUMB: u64       = 0x0106;
/// Command to trigger a page flip.
pub const DRM_PAGE_FLIP: u64      = 0x0107;
/// Command to create a GEM object.
pub const DRM_GEM_CREATE: u64     = 0x0108;
/// Command to map a GEM object.
pub const DRM_GEM_MMAP: u64       = 0x0109;
/// Command to perform a GPU-accelerated blit.
pub const DRM_GPU_BLIT: u64       = 0x0110;
/// Command to perform a GPU-accelerated fill.
pub const DRM_GPU_FILL: u64       = 0x0111;

/// Display information structure.
#[repr(C)]
pub struct DisplayInfo {
    /// Screen width in pixels
    pub width: u32,
    /// Screen height in pixels
    pub height: u32,
}

/// Dumb buffer information.
#[repr(C)]
pub struct DumbInfo {
    /// Buffer handle
    pub id: u64,
    /// Buffer size in bytes
    pub size: u64,
    /// Virtual memory address
    pub addr: u64,
}

/// GPU Blit command structure.
#[repr(C)]
pub struct GpuBlit {
    pub src_id: u64,
    pub dst_id: u64,
    pub src_x: u32,
    pub src_y: u32,
    pub dst_x: u32,
    pub dst_y: u32,
    pub width: u32,
    pub height: u32,
}

/// Retrieves display configuration.
pub fn get_display_info() -> Result<DisplayInfo, Error> {
    let mut info = DisplayInfo { width: 0, height: 0 };
    // SAFETY: drmctl syscall is safe here
    let r = unsafe {
        syscall3(SYS_DRMCTL, 0, DRM_GET_DISPLAY_INFO, &mut info as *mut DisplayInfo as u64)
    };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(info) }
}

/// Creates a new dumb buffer for graphics.
pub fn create_dumb(width: u32, height: u32, bpp: u32) -> Result<DumbInfo, Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe {
        syscall5(SYS_DRMCTL, 0, DRM_CREATE_DUMB, width as u64, height as u64, bpp as u64)
    };
    if r < 0 { return Err(Error::from_i64(r)); }
    Ok(DumbInfo { id: r as u64, size: (width * height * bpp / 8) as u64, addr: 0 })
}

/// Accelerated blit using the GPU.
pub fn gpu_blit(blit: &GpuBlit) -> Result<(), Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall3(SYS_DRMCTL, 0, DRM_GPU_BLIT, blit as *const _ as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Accelerated fill using the GPU.
pub fn gpu_fill(id: u64, x: u32, y: u32, w: u32, h: u32, color: u32) -> Result<(), Error> {
    // SAFETY: drmctl syscall is safe here
    // We package args in a simple way or use a struct
    let r = unsafe { syscall6(SYS_DRMCTL, id, DRM_GPU_FILL, x as u64 | ((y as u64) << 32), w as u64 | ((h as u64) << 32), color as u64, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Destroys a dumb buffer.
pub fn destroy_dumb(id: u64) -> Result<(), Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall3(SYS_DRMCTL, id, DRM_DESTROY_DUMB, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Flips the display buffers.
pub fn flip() -> Result<(), Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall2(SYS_DRMCTL, 0, DRM_FLIP) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Sets the display resolution and bit depth.
pub fn set_mode(w: u32, h: u32, bpp: u32) -> Result<(), Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall5(SYS_DRMCTL, 0, DRM_SET_MODE, w as u64, h as u64, bpp as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Maps a dumb buffer to user memory.
pub fn map_dumb(id: u64) -> Result<*mut u32, Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall3(SYS_DRMCTL, id, DRM_MAP_DUMB, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as *mut u32) }
}

/// Command to set accent color.
pub const DRM_SET_ACCENT_COLOR: u64 = 0x010A;

/// Sets the system accent color.
pub fn set_accent_color(color: u32) -> Result<(), Error> {
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall3(SYS_DRMCTL, 0, DRM_SET_ACCENT_COLOR, color as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Command to set wallpaper.
pub const DRM_SET_WALLPAPER: u64 = 0x010B;

/// Sets the desktop wallpaper image.
pub fn set_wallpaper(path: &str) -> Result<(), Error> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(Error::EINVAL); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    // SAFETY: drmctl syscall is safe here
    let r = unsafe { syscall3(SYS_DRMCTL, 0, DRM_SET_WALLPAPER, buf.as_ptr() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}
