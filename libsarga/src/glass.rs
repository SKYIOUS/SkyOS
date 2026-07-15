use crate::syscall::*;
use core::cell::Cell;

pub struct ShadowParams {
    pub radius: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub color: u32,
    pub opacity: f32,
}

const SYS_GLASS_SET_OPACITY: u64 = 130;
const SYS_GLASS_SET_BLUR: u64 = 131;
const SYS_GLASS_SET_SHADOW: u64 = 132;
const SYS_GLASS_FLUSH: u64 = 133;
const SYS_GLASS_POLL: u64 = 134;

pub struct GlassWindow {
    window_id: u64,
    pub opacity: Cell<f32>,
    pub blur: Cell<u32>,
    pub shadow: Cell<Option<ShadowParams>>,
}

impl GlassWindow {
    pub fn new(window_id: u64) -> Self {
        GlassWindow {
            window_id,
            opacity: Cell::new(1.0),
            blur: Cell::new(0),
            shadow: Cell::new(None),
        }
    }

    pub fn set_opacity(&self, opacity: f32) {
        let fixed = (opacity.clamp(0.0, 1.0) * 65536.0) as u32;
        let _ = unsafe { syscall2(SYS_GLASS_SET_OPACITY, self.window_id, fixed as u64) };
        self.opacity.set(opacity);
    }

    pub fn set_blur_radius(&self, radius: u32) {
        let _ = unsafe { syscall2(SYS_GLASS_SET_BLUR, self.window_id, radius as u64) };
        self.blur.set(radius);
    }

    pub fn set_shadow(&self, params: ShadowParams) {
        let packed = ShadowParams {
            radius: params.radius,
            offset_x: params.offset_x,
            offset_y: params.offset_y,
            color: params.color,
            opacity: params.opacity,
        };
        let ptr = &packed as *const ShadowParams as u64;
        let _ = unsafe { syscall2(SYS_GLASS_SET_SHADOW, self.window_id, ptr) };
        self.shadow.set(Some(params));
    }

    pub fn flush(&self) -> Result<u64, i64> {
        let ret = unsafe { syscall1(SYS_GLASS_FLUSH, self.window_id) };
        if ret < 0 { Err(-ret) } else { Ok(ret as u64) }
    }

    pub fn poll(&self, fence_id: u64) -> bool {
        let ret = unsafe { syscall2(SYS_GLASS_POLL, self.window_id, fence_id) };
        ret != 0
    }
}
