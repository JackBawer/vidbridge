use crate::{Decoder, Demuxer, Frame};
use tokio::task;

pub struct AsyncDecoder {
    inner: Option<Decoder>,
}

impl AsyncDecoder {
    pub async fn new(codec_name: String) -> Result<Self, String> {
        let inner = task::spawn_blocking(move || Decoder::new(&codec_name))
            .await
            .map_err(|e| format!("blocking task panicked: {e}"))??;
        Ok(Self { inner: Some(inner) })
    }

    /// Note: takes the sync Demuxer, not AsyncDemuxer — this call is cheap
    /// (just copying codec params), so it doesn't need its own AsyncDemuxer
    /// dependency. Caller passes in demuxer.inner if using AsyncDemuxer, or
    /// a plain Demuxer directly.
    pub async fn init_from_demuxer(&mut self, demuxer: &Demuxer) -> Result<(), i32> {
        let mut decoder = self.inner.take().expect("decoder already closed");
        // Demuxer isn't Send-safe to move here since we only have &Demuxer,
        // so we do this synchronously rather than via spawn_blocking — it's
        // a fast metadata copy, not real I/O, so this is an acceptable tradeoff.
        let result = decoder.init_from_demuxer(demuxer);
        self.inner = Some(decoder);
        result
    }

    pub async fn send_packet(&mut self, data: Option<Vec<u8>>, pts: i64) -> Result<(), i32> {
        let mut decoder = self.inner.take().expect("decoder already closed");
        let (result, decoder) = task::spawn_blocking(move || {
            let result = decoder.send_packet(data.as_deref(), pts);
            (result, decoder)
        })
        .await
        .expect("blocking task panicked");
        self.inner = Some(decoder);
        result
    }

    pub async fn receive_frame(&mut self) -> Result<Option<Frame>, i32> {
        let mut decoder = self.inner.take().expect("decoder already closed");
        let (result, decoder) = task::spawn_blocking(move || {
            let mut frame = Frame::new().expect("frame alloc failed");
            let result = decoder.receive_frame(&mut frame);
            match result {
                Ok(true) => (Ok(Some(frame)), decoder),
                Ok(false) => (Ok(None), decoder),
                Err(e) => (Err(e), decoder),
            }
        })
        .await
        .expect("blocking task panicked");
        self.inner = Some(decoder);
        result
    }
}
