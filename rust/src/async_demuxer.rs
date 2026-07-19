use crate::Demuxer;
use tokio::task;

pub struct AsyncDemuxer {
    inner: Option<Demuxer>,
}

impl AsyncDemuxer {
    pub async fn open(path: String) -> Result<Self, String> {
        let inner = task::spawn_blocking(move || Demuxer::open(&path))
            .await
            .map_err(|e| format!("blocking task panicked: {e}"))??;
        Ok(Self { inner: Some(inner) })
    }

    pub fn inner(&self) -> &Demuxer {
        self.inner.as_ref().expect("demuxer already closed")
    }

    pub async fn width(&self) -> i32 {
        // Cheap, non-blocking call — safe to run directly without spawn_blocking.
        self.inner.as_ref().expect("demuxer already closed").width()
    }

    pub async fn height(&self) -> i32 {
        self.inner.as_ref().expect("demuxer already closed").height()
    }

    pub async fn codec_name(&self) -> String {
        self.inner.as_ref().expect("demuxer already closed").codec_name()
    }

    pub async fn framerate(&self) -> crate::AVRational {
        self.inner.as_ref().expect("demuxer already closed").framerate()
    }

    /// Reads the next packet. This is the actual blocking I/O call
    /// (network read for RTSP, disk read for files) — always goes
    /// through spawn_blocking.
    pub async fn read_packet(&mut self) -> Option<(Vec<u8>, i64)> {
        let mut demuxer = self.inner.take()?;
        let (result, demuxer) = task::spawn_blocking(move || {
            let result = demuxer.read_packet();
            (result, demuxer)
        })
        .await
        .expect("blocking task panicked");
        self.inner = Some(demuxer);
        result
    }
}
