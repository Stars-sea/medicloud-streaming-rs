//! Context trait and utilities for FFmpeg format contexts.

use crate::core::stream::Stream;
use ffmpeg_sys_next::*;
use std::ffi::{c_int, CStr};

/// Trait for accessing AVFormatContext functionality.
///
/// # Safety
/// Implementors must ensure the context pointer is valid and properly managed.
pub(crate) trait Context {
    /// Returns the underlying AVFormatContext pointer.
    ///
    /// # Safety
    /// The pointer must remain valid for the lifetime of the implementor.
    fn get_ctx(&self) -> *mut AVFormatContext;

    /// Returns the number of streams in the context.
    fn nb_streams(&self) -> u32 {
        unsafe { (*self.get_ctx()).nb_streams }
    }

    /// Gets a stream by index.
    ///
    /// # Arguments
    /// * `id` - Stream index (must be < nb_streams())
    ///
    /// # Returns
    /// Some(Stream) if the index is valid, None otherwise.
    fn stream(&self, id: u32) -> Option<Stream> {
        if id < self.nb_streams() {
            let ptr = unsafe { (*self.get_ctx()).streams.offset(id as isize) };
            unsafe { Some(Stream::new(*ptr)) }
        } else {
            None
        }
    }

    /// Checks if the context is available (pointer is not null).
    fn available(&self) -> bool {
        !self.get_ctx().is_null()
    }
}

/// Converts an FFmpeg error code to a human-readable string.
///
/// # Arguments
/// * `code` - FFmpeg error code (negative value)
///
/// # Returns
/// Error message string
pub(crate) fn ffmpeg_error(code: c_int) -> String {
    let mut buf = [0i8; 1024];
    unsafe {
        av_strerror(code, buf.as_mut_ptr(), buf.len());

        CStr::from_ptr(buf.as_mut_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
