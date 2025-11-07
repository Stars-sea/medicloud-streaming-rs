use ffmpeg_sys_next::*;
use std::ffi::{CStr, c_int};

pub(crate) trait Context {
    fn get_ctx(&self) -> *mut AVFormatContext;

    fn nb_streams(&self) -> u32 {
        unsafe { (*self.get_ctx()).nb_streams }
    }

    fn stream(&self, id: u32) -> Option<*mut *mut AVStream> {
        if id < self.nb_streams() {
            Some(unsafe { (*self.get_ctx()).streams.offset(id as isize) })
        } else {
            None
        }
    }
    
    fn available(&self) -> bool {
        !self.get_ctx().is_null()
    }
}

pub(crate) fn ffmpeg_error(code: c_int) -> String {
    let mut buf = [0i8; 1024];
    unsafe {
        av_strerror(code, buf.as_mut_ptr(), buf.len());

        CStr::from_ptr(buf.as_mut_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
