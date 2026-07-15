use crate::ffi;
use std::ptr::NonNull;

pub struct Frame {
    ptr: NonNull<ffi::RawFrame>,
}

impl Frame {
    pub fn new() -> Result<Self, String> {
        let ptr = unsafe { ffi::frame_create() };
        NonNull::new(ptr)
            .map(|ptr| Frame { ptr })
            .ok_or_else(|| "frame_create returned NULL".to_string())
    }

    /// Returns the raw plane data. Borrowed from the underlying AVFrame —
    /// valid only as long as this Frame is alive and hasn't been reused
    /// by another decoder_receive_frame/encoder call.
    pub fn data(&self, plane: i32) -> Option<*mut u8> {
        let ptr = unsafe { ffi::frame_get_data(self.ptr.as_ptr(), plane) };
        if ptr.is_null() { None } else { Some(ptr) }
    }

    pub fn linesize(&self, plane: i32) -> Option<i32> {
        let ls = unsafe { ffi::frame_get_linesize(self.ptr.as_ptr(), plane) };
        if ls < 0 { None } else { Some(ls) }
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::RawFrame {
        self.ptr.as_ptr()
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { ffi::frame_free(self.ptr.as_ptr()) };
    }
}
