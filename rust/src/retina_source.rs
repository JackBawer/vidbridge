//! An alternate RTSP source that uses the pure-Rust `retina` crate instead
//! of going through FFmpeg's `avformat`. It's a drop-in alternative to
//! `Demuxer` for RTSP URLs specifically: files and non-RTSP sources should
//! keep using `Demuxer`/`AsyncDemuxer` as before.
//!
//! Output packets are AVC (4-byte length-prefixed NAL) format, matching
//! what `Decoder::init_from_extradata` + `Decoder::send_packet` expect —
//! NOT Annex-B, so don't mix this up with packets from `Demuxer`, which
//! come out of FFmpeg's RTSP demuxer in whatever format that produces.
//!
//! retina's public API has shifted across 0.4.x releases; if this doesn't
//! compile against the version pulled in by Cargo.lock, check
//! https://docs.rs/retina/latest/retina/client/ for the current shape of
//! `SessionOptions`/`SetupOptions`/`PlayOptions` and adjust accordingly —
//! the overall describe -> setup -> play -> demuxed() flow has been stable.

use futures::StreamExt;
use retina::client::{Credentials, PlayOptions, Session, SessionOptions, SetupOptions};
use retina::codec::{CodecItem, ParametersRef};
use retina::codec::VideoParameters;
use url::Url;

pub struct RetinaRtspSource {
    session: retina::client::Demuxed,
    video_stream_i: usize,
    codec_name: String,
    extradata: Vec<u8>,
    width: i32,
    height: i32,
}

impl RetinaRtspSource {
    /// Connects, describes, and starts playing the RTSP stream. Picks the
    /// first H.264 or H.265 video track it finds.
    pub async fn open(
        rtsp_url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, String> {
        let url = Url::parse(rtsp_url).map_err(|e| format!("bad RTSP url: {e}"))?;

        let creds = match (username, password) {
            (Some(username), Some(password)) => Some(Credentials {
                username: username.to_string(),
                password: password.to_string(),
            }),
            _ => None,
        };

        let session_options = SessionOptions::default().creds(creds);

        let mut session = Session::describe(url, session_options)
            .await
            .map_err(|e| format!("DESCRIBE failed: {e}"))?;

        let video_stream_i = session
            .streams()
            .iter()
            .position(|s| matches!(s.parameters(), Some(ParametersRef::Video(_))))
            .ok_or_else(|| "no video stream found in RTSP session".to_string())?;

        let (codec_name, extradata, width, height) = match session.streams()[video_stream_i]
            .parameters()
        {
            Some(ParametersRef::Video(params)) => {
                let name = video_codec_name(params)?;
                let (w, h) = params.pixel_dimensions();
                (name, params.extra_data().to_vec(), w as i32, h as i32)
            }
            _ => return Err("selected stream is not video".to_string()),
        };

        session
            .setup(video_stream_i, SetupOptions::default())
            .await
            .map_err(|e| format!("SETUP failed: {e}"))?;

        let session = session
            .play(PlayOptions::default())
            .await
            .map_err(|e| format!("PLAY failed: {e}"))?
            .demuxed()
            .map_err(|e| format!("demuxing setup failed: {e}"))?;

        Ok(Self {
            session,
            video_stream_i,
            codec_name,
            extradata,
            width,
            height,
        })
    }

    /// Codec name suitable for `Decoder::new` (e.g. "h264" or "hevc").
    pub fn codec_name(&self) -> &str {
        &self.codec_name
    }

    /// avcC/hvcC-style parameter set blob, suitable for
    /// `Decoder::init_from_extradata`.
    pub fn extradata(&self) -> &[u8] {
        &self.extradata
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// Reads the next video packet in AVC format. Returns `None` on
    /// stream error or clean end (RTSP normally never reaches a natural
    /// EOF, so callers should also enforce their own frame/time limit).
    pub async fn read_packet(&mut self) -> Option<(Vec<u8>, i64)> {
        loop {
            match self.session.next().await? {
                Ok(CodecItem::VideoFrame(frame)) => {
                    if frame.stream_id() != self.video_stream_i {
                        continue;
                    }
                    let pts = frame.timestamp().elapsed();
                    return Some((frame.into_data(), pts));
                }
                Ok(_) => continue, // audio/other stream items, ignore
                Err(_) => return None,
            }
        }
    }
}

fn video_codec_name(params: &VideoParameters) -> Result<String, String> {
    // rfc6381_codec() gives strings like "avc1.64002a" (H.264) or
    // "hev1.1.6.L93.B0" / "hvc1..." (H.265) — the 4-char prefix identifies
    // the codec family regardless of profile/level suffix.
    let rfc6381 = params.rfc6381_codec();
    if rfc6381.starts_with("avc1") {
        Ok("h264".to_string())
    } else if rfc6381.starts_with("hev1") || rfc6381.starts_with("hvc1") {
        Ok("hevc".to_string())
    } else {
        Err(format!("unsupported video codec in RTSP stream: {rfc6381}"))
    }
}
