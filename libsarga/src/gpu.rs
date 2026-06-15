use crate::syscall::*;

pub const SYS_DRMCTL: u64 = 400;

pub const DRM_GET_DISPLAY_INFO: u64 = 0x0100;
pub const DRM_CREATE_DUMB: u64     = 0x0101;
pub const DRM_DESTROY_DUMB: u64   = 0x0103;
pub const DRM_FLIP: u64           = 0x0104;
pub const DRM_SET_MODE: u64       = 0x0105;
pub const DRM_MAP_DUMB: u64       = 0x0106;
pub const DRM_PAGE_FLIP: u64      = 0x0107;
pub const DRM_GEM_CREATE: u64     = 0x0108;
pub const DRM_GEM_MMAP: u64       = 0x0109;

#[repr(C)]
pub struct DisplayInfo {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
pub struct DumbInfo {
    pub id: u64,
    pub size: u64,
    pub addr: u64,
}

pub fn get_display_info() -> Result<DisplayInfo, i64> {
    let mut info = DisplayInfo { width: 0, height: 0 };
    let r = unsafe {
        syscall3(SYS_DRMCTL, 0, DRM_GET_DISPLAY_INFO, &mut info as *mut DisplayInfo as u64)
    };
    if r < 0 { Err(-r) } else { Ok(info) }
}

pub fn create_dumb() -> Result<DumbInfo, i64> {
    let mut info = DumbInfo { id: 0, size: 0, addr: 0 };
    let r = unsafe {
        syscall3(SYS_DRMCTL, 0, DRM_CREATE_DUMB, &mut info as *mut DumbInfo as u64)
    };
    if r < 0 { Err(-r) } else { Ok(info) }
}

pub fn destroy_dumb() -> Result<(), i64> {
    let r = unsafe { syscall2(SYS_DRMCTL, 0, DRM_DESTROY_DUMB) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn flip() -> Result<(), i64> {
    let r = unsafe { syscall2(SYS_DRMCTL, 0, DRM_FLIP) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn set_mode(w: u32, h: u32, bpp: u32) -> Result<(), i64> {
    let r = unsafe { syscall5(SYS_DRMCTL, 0, DRM_SET_MODE, w as u64, h as u64, bpp as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn map_dumb(id: u64) -> Result<*mut u32, i64> {
    let r = unsafe { syscall3(SYS_DRMCTL, id, DRM_MAP_DUMB, 0) };
    if r < 0 { Err(-r) } else { Ok(r as *mut u32) }
}

pub fn page_flip(id: u64) -> Result<(), i64> {
    let r = unsafe { syscall2(SYS_DRMCTL, id, DRM_PAGE_FLIP) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn gem_create(size: u64) -> Result<u64, i64> {
    let r = unsafe { syscall2(SYS_DRMCTL, size, DRM_GEM_CREATE) };
    if r < 0 { Err(-r) } else { Ok(r as u64) }
}

pub fn gem_mmap(id: u64) -> Result<*mut u32, i64> {
    let r = unsafe { syscall2(SYS_DRMCTL, id, DRM_GEM_MMAP) };
    if r < 0 { Err(-r) } else { Ok(r as *mut u32) }
}

pub const DRM_SET_ACCENT_COLOR: u64 = 0x010A;

pub fn set_accent_color(color: u32) -> Result<(), i64> {
    let r = unsafe { syscall3(SYS_DRMCTL, 0, DRM_SET_ACCENT_COLOR, color as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub const DRM_SET_WALLPAPER: u64 = 0x010B;

pub fn set_wallpaper(path: &str) -> Result<(), i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall3(SYS_DRMCTL, 0, DRM_SET_WALLPAPER, buf.as_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}
