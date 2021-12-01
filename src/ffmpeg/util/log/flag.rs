use crate::ffmpeg::ffi::*;
use libc::c_int;
use bitflags::bitflags;

bitflags! {
    pub struct Flags: c_int {
        const SKIP_REPEATED = AV_LOG_SKIP_REPEATED;
        const PRINT_LEVEL = AV_LOG_PRINT_LEVEL;
    }
}
