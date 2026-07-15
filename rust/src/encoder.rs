use crate::ffi;
use crate::Frame;
use std::ffi::CString;
use std::ptr::NonNull;

pub struct Encoder {
    ptr: NonNull<ffi::VideoEncoder>,
}

impl Encoder {
    pub fn new(
        codec_name: &str,
        width: i32,
        height: i32,
        fps: ffi::AVRational,
        bitrate: i32,
        needs_global_header: bool,
    ) -> Result<Self, String> {
        let c_name = CString::new(codec_name).map_err(|e| e.to_string())?;
        let ptr = unsafe {
            ffi::encoder_create(
                c_name.as_ptr(),
                width,
                height,
                fps,
                bitrate,
                needs_global_header as i32,
            )
        };
        NonNull::new(ptr)
            .map(|ptr| Encoder { ptr })
            .ok_or_else(|| format!("failed to create encoder for codec {codec_name}"))
    }

    /// Send a frame for encoding. Pass None to flush at end-of-stream.
    pub fn send_frame(&mut self, frame: Option<&Frame>) -> Result<(), i32> {
        let frame_ptr = frame.map_or(std::ptr::null_mut(), |f| f.as_ptr());
        let ret = unsafe { ffi::encoder_send_frame(self.ptr.as_ptr(), frame_ptr) };
        if ret < 0 { Err(ret) } else { Ok(()) }
    }

    /// Copies out one encoded packet if ready. Returns None on EAGAIN/EOF.
    pub fn receive_packet(&mut self) -> Result<Option<(Vec<u8>, i64, i64)>, i32> {
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut size: i32 = 0;
        let mut pts: i64 = 0;
        let mut dts: i64 = 0;

        let ret = unsafe {
            ffi::encoder_receive_packet(self.ptr.as_ptr(), &mut data, &mut size, &mut pts, &mut dts)
        };

        if ret == 0 {
            let bytes = unsafe { std::slice::from_raw_parts(data, size as usize).to_vec() };
            Ok(Some((bytes, pts, dts)))
        } else if ret == unsafe { ffi::vidbridge_averror_eagain() } || ret == unsafe { ffi::vidbridge_averror_eof() } {
            Ok(None)
        } else if ret < 0 {
            Err(ret)
        } else {
            Ok(None)
        }
}

    pub(crate) fn as_ptr(&self) -> *mut ffi::VideoEncoder {
        self.ptr.as_ptr()
    }

    pub fn time_base(&self) -> ffi::AVRational {
        unsafe { ffi::encoder_get_time_base(self.ptr.as_ptr()) }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { ffi::encoder_free(self.ptr.as_ptr()) };
    }
}
