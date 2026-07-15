use crate::ffi;
use crate::{Demuxer, Frame};
use std::ffi::CString;
use std::ptr::NonNull;

pub struct Decoder {
    ptr: NonNull<ffi::VideoDecoder>,
}

impl Decoder {
    pub fn new(codec_name: &str) -> Result<Self, String> {
        let c_name = CString::new(codec_name).map_err(|e| e.to_string())?;
        let ptr = unsafe { ffi::decoder_create(c_name.as_ptr()) };
        NonNull::new(ptr)
            .map(|ptr| Decoder { ptr })
            .ok_or_else(|| format!("failed to create decoder for codec {codec_name}"))
    }

    pub fn init_from_demuxer(&mut self, demuxer: &Demuxer) -> Result<(), i32> {
        let ret = unsafe { ffi::decoder_initialize_from_demuxer(self.ptr.as_ptr(), demuxer.as_ptr()) };
        if ret < 0 { Err(ret) } else { Ok(()) }
    }

    /// Send a packet for decoding. Pass None to flush at end-of-stream.
    pub fn send_packet(&mut self, data: Option<&[u8]>, pts: i64) -> Result<(), i32> {
        let ret = match data {
            Some(bytes) => unsafe {
                ffi::decoder_send_packet(
                    self.ptr.as_ptr(),
                    bytes.as_ptr(),
                    bytes.len() as i32,
                    pts,
                )
            },
            None => unsafe {
                ffi::decoder_send_packet(self.ptr.as_ptr(), std::ptr::null(), 0, 0)
            },
        };
        if ret < 0 { Err(ret) } else { Ok(()) }
    }

    /// Returns Ok(true) if a frame was decoded into `frame`, Ok(false) on
    /// EAGAIN/EOF (not an error, just "nothing ready right now"), Err on
    /// a genuine decode error.
    pub fn receive_frame(&mut self, frame: &mut Frame) -> Result<bool, i32> {
        let ret = unsafe { ffi::decoder_receive_frame(self.ptr.as_ptr(), frame.as_ptr()) };
        match ret {
            0 => Ok(true),
            e if e == neg_eagain() || e == neg_eof() => Ok(false),
            e => Err(e),
        }
    }
}

fn neg_eagain() -> i32 { unsafe { ffi::vidbridge_averror_eagain() } }
fn neg_eof() -> i32 { unsafe { ffi::vidbridge_averror_eof() } }

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { ffi::decoder_free(self.ptr.as_ptr()) };
    }
}
