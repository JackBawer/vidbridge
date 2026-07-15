use crate::ffi;
use crate::Encoder;
use std::ffi::CString;
use std::ptr::NonNull;

pub struct Muxer {
    ptr: NonNull<ffi::VideoMuxer>,
}

impl Muxer {
    pub fn create(output_path: &str, encoder: &Encoder, framerate: ffi::AVRational) -> Result<Self, String> {
        let c_path = CString::new(output_path).map_err(|e| e.to_string())?;
        let ptr = unsafe { ffi::muxer_create(c_path.as_ptr(), encoder.as_ptr(), framerate) };
        NonNull::new(ptr)
            .map(|ptr| Muxer { ptr })
            .ok_or_else(|| format!("failed to create muxer for {output_path}"))
    }

    pub fn write_packet(
        &mut self,
        data: &[u8],
        pts: i64,
        dts: i64,
        encoder_time_base: ffi::AVRational,
    ) -> Result<(), i32> {
        let ret = unsafe {
            ffi::muxer_write_packet(
                self.ptr.as_ptr(),
                data.as_ptr() as *mut u8,
                data.len() as i32,
                pts,
                dts,
                encoder_time_base,
            )
        };
        if ret < 0 { Err(ret) } else { Ok(()) }
    }
}

impl Drop for Muxer {
    fn drop(&mut self) {
        unsafe { ffi::muxer_free(self.ptr.as_ptr()) };
    }
}
