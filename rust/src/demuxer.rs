use crate::ffi;
use std::ffi::{CStr, CString};
use std::ptr::NonNull;

pub struct Demuxer {
    ptr: NonNull<ffi::VideoDemuxer>,
}

impl Demuxer {
    pub fn open(path: &str) -> Result<Self, String> {
        let c_path = CString::new(path).map_err(|e| e.to_string())?;
        let ptr = unsafe { ffi::demuxer_create(c_path.as_ptr()) };
        NonNull::new(ptr)
            .map(|ptr| Demuxer { ptr })
            .ok_or_else(|| format!("failed to open demuxer for {path}"))
    }

    pub fn width(&self) -> i32 {
        unsafe { ffi::demuxer_get_width(self.ptr.as_ptr()) }
    }

    pub fn height(&self) -> i32 {
        unsafe { ffi::demuxer_get_height(self.ptr.as_ptr()) }
    }

    pub fn framerate(&self) -> ffi::AVRational {
        unsafe { ffi::demuxer_get_framerate(self.ptr.as_ptr()) }
    }

    pub fn codec_name(&self) -> String {
        unsafe {
            let ptr = ffi::demuxer_get_codec_name(self.ptr.as_ptr());
            if ptr.is_null() {
                return "unknown".to_string();
            }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    /// Reads the next video packet. Returns None at EOF.
    /// The returned Vec<u8> is an owned copy — the underlying C packet
    /// buffer is only valid until the next call to this function, so we
    /// copy out immediately rather than exposing the raw borrowed pointer.
    pub fn read_packet(&mut self) -> Option<(Vec<u8>, i64)> {
        let (bytes, pts) = self.read_packet_ref()?;
        Some((bytes.to_vec(), pts))
    }

    /// Zero-copy variant of `read_packet`: borrows straight from the C
    /// packet buffer instead of allocating a Vec. The returned slice is
    /// only valid until the next call to any &mut method on this Demuxer
    /// (the borrow checker enforces this via the lifetime on `&mut self`),
    /// which mirrors the real constraint the underlying C buffer has —
    /// FFmpeg reuses it on the next av_read_frame call.
    ///
    /// Use this when you're about to hand the bytes straight to
    /// Decoder::send_packet in the same iteration and don't need to hold
    /// onto them past that — e.g. the sync transcode.rs loop. Prefer
    /// `read_packet` (owned) when the data needs to outlive this call,
    /// e.g. crossing into a spawn_blocking closure like AsyncDemuxer does.
    pub fn read_packet_ref(&mut self) -> Option<(&[u8], i64)> {
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut size: i32 = 0;
        let mut pts: i64 = 0;

        let ret = unsafe {
            ffi::demuxer_read_packet(self.ptr.as_ptr(), &mut data, &mut size, &mut pts)
        };
        if ret != 0 || data.is_null() {
            return None;
        }

        let bytes = unsafe { std::slice::from_raw_parts(data, size as usize) };
        Some((bytes, pts))
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::VideoDemuxer {
        self.ptr.as_ptr()
    }
}

impl Drop for Demuxer {
    fn drop(&mut self) {
        unsafe { ffi::demuxer_free(self.ptr.as_ptr()) };
    }
}
