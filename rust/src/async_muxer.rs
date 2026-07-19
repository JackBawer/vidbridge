use crate::{Encoder, Muxer, AVRational};
use tokio::task;

pub struct AsyncMuxer {
    inner: Option<Muxer>,
}

impl AsyncMuxer {
    // Note: takes &Encoder synchronously for the same reason as
    // AsyncDecoder::init_from_demuxer — muxer_create needs a live
    // reference to the encoder's codec_ctx, which can't cross the
    // spawn_blocking 'static boundary as a borrow. Header writing is
    // typically cheap relative to actual encode work, so running this
    // step synchronously is an acceptable tradeoff.
    pub async fn create(output_path: String, encoder: &Encoder, framerate: AVRational) -> Result<Self, String> {
        let inner = Muxer::create(&output_path, encoder, framerate)?;
        Ok(Self { inner: Some(inner) })
    }

    pub async fn write_packet(
        &mut self,
        data: Vec<u8>,
        pts: i64,
        dts: i64,
        encoder_time_base: AVRational,
    ) -> Result<(), i32> {
        let mut muxer = self.inner.take().expect("muxer already closed");
        let (result, muxer) = task::spawn_blocking(move || {
            let result = muxer.write_packet(&data, pts, dts, encoder_time_base);
            (result, muxer)
        })
        .await
        .expect("blocking task panicked");
        self.inner = Some(muxer);
        result
    }
}
