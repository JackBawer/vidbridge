use crate::{Encoder, Frame, AVRational};
use tokio::task;

pub struct AsyncEncoder {
    inner: Option<Encoder>,
}

impl AsyncEncoder {
    pub async fn new(
        codec_name: String,
        width: i32,
        height: i32,
        fps: AVRational,
        bitrate: i32,
        needs_global_header: bool,
    ) -> Result<Self, String> {
        let inner = task::spawn_blocking(move || {
            Encoder::new(&codec_name, width, height, fps, bitrate, needs_global_header)
        })
        .await
        .map_err(|e| format!("blocking task panicked: {e}"))??;
        Ok(Self { inner: Some(inner) })
    }

    pub fn inner(&self) -> &Encoder {
        self.inner.as_ref().expect("encoder already closed")
    }

    pub async fn send_frame(&mut self, frame: Option<Frame>) -> Result<(), i32> {
        let mut encoder = self.inner.take().expect("encoder already closed");
        let (result, encoder) = task::spawn_blocking(move || {
            let result = encoder.send_frame(frame.as_ref());
            (result, encoder)
        })
        .await
        .expect("blocking task panicked");
        self.inner = Some(encoder);
        result
    }

    pub async fn receive_packet(&mut self) -> Result<Option<(Vec<u8>, i64, i64)>, i32> {
        let mut encoder = self.inner.take().expect("encoder already closed");
        let (result, encoder) = task::spawn_blocking(move || {
            let result = encoder.receive_packet();
            (result, encoder)
        })
        .await
        .expect("blocking task panicked");
        self.inner = Some(encoder);
        result
    }

    pub fn time_base(&self) -> AVRational {
        self.inner.as_ref().expect("encoder already closed").time_base()
    }
}
