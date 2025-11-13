use crate::core::output::TsOutputContext;
use std::path::PathBuf;
use tokio::sync::{broadcast, mpsc};

#[derive(Clone, Debug)]
pub struct OnSegmentComplete {
    live_id: String,
    segment_id: String,
    path: PathBuf,
}

pub type SegmentCompleteRx = mpsc::UnboundedReceiver<OnSegmentComplete>;
pub type SegmentCompleteTx = mpsc::UnboundedSender<OnSegmentComplete>;

impl OnSegmentComplete {
    pub fn channel() -> (SegmentCompleteTx, SegmentCompleteRx) {
        mpsc::unbounded_channel()
    }

    pub fn new(live_id: String, segment_id: String, path: PathBuf) -> Self {
        Self {
            live_id,
            segment_id,
            path,
        }
    }

    pub fn from_ctx(live_id: String, ctx: &TsOutputContext) -> Self {
        let path = ctx.path().clone();
        OnSegmentComplete::new(
            live_id.to_string(),
            path.file_name().unwrap().display().to_string(),
            path,
        )
    }

    pub fn live_id(&self) -> &str {
        &self.live_id
    }

    pub fn segment_id(&self) -> &str {
        &self.segment_id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Clone, Debug)]
pub struct OnStreamTerminate {
    live_id: String,
    error: Option<String>,
    path: PathBuf,
}

pub type StreamTerminateRx = broadcast::Receiver<OnStreamTerminate>;
pub type StreamTerminateTx = broadcast::Sender<OnStreamTerminate>;

impl OnStreamTerminate {
    pub fn channel(capacity: usize) -> (StreamTerminateTx, StreamTerminateRx) {
        broadcast::channel::<OnStreamTerminate>(capacity)
    }

    pub fn new(live_id: String, error: Option<String>, path: PathBuf) -> Self {
        Self {
            live_id,
            error,
            path,
        }
    }

    pub fn live_id(&self) -> &str {
        &self.live_id
    }

    pub fn error(&self) -> &Option<String> {
        &self.error
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Clone, Debug)]
pub struct OnStopStream {
    live_id: String,
}

pub type StopStreamRx = broadcast::Receiver<OnStopStream>;
pub type StopStreamTx = broadcast::Sender<OnStopStream>;

impl OnStopStream {
    pub fn channel(capacity: usize) -> (StopStreamTx, StopStreamRx) {
        broadcast::channel::<OnStopStream>(capacity)
    }

    pub fn new(live_id: String) -> Self {
        Self { live_id }
    }

    pub fn live_id(&self) -> &str {
        &self.live_id
    }
}
