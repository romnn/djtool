use crate::ffmpeg::ffi::*;
use libc::c_int;
use bitflags::bitflags;

bitflags! {
    pub struct Flags: c_int {
        const CORRUPT = AV_FRAME_FLAG_CORRUPT;
    }
}
