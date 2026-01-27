#![allow(dead_code)]

use crate::core::output::TsOutputContext;
use std::path::{Path, PathBuf};
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

    pub fn new(live_id: &str, segment_id: String, path: PathBuf) -> Self {
        Self {
            live_id: live_id.to_string(),
            segment_id,
            path,
        }
    }

    pub fn from_ctx(live_id: &str, ctx: &TsOutputContext) -> Self {
        let path = ctx.path().clone();
        OnSegmentComplete::new(
            live_id,
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
pub struct OnStreamStarted {
    live_id: String,
}

pub type StreamStartedRx = broadcast::Receiver<OnStreamStarted>;
pub type StreamStartedTx = broadcast::Sender<OnStreamStarted>;

impl OnStreamStarted {
    pub fn channel(capacity: usize) -> (StreamStartedTx, StreamStartedRx) {
        broadcast::channel::<OnStreamStarted>(capacity)
    }

    pub fn new(live_id: &str) -> Self {
        Self {
            live_id: live_id.to_string(),
        }
    }

    pub fn live_id(&self) -> &str {
        &self.live_id
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

    pub fn new<T: AsRef<Path>>(live_id: &str, error: Option<String>, path: T) -> Self {
        Self {
            live_id: live_id.to_string(),
            error,
            path: PathBuf::from(path.as_ref()),
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

    pub fn new(live_id: &str) -> Self {
        Self {
            live_id: live_id.to_string(),
        }
    }

    pub fn live_id(&self) -> &str {
        &self.live_id
    }
}
