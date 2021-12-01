use crate::ffmpeg::ffi::*;
use libc::c_int;
use bitflags::bitflags;

bitflags! {
    pub struct Flags: c_int {
        const FORCED = AV_SUBTITLE_FLAG_FORCED;
    }
}
