use std::ptr;

use crate::ffi::*;
use crate::{format, Format};

pub struct AudioIter(*mut AVOutputFormat);

impl Iterator for AudioIter {
    type Item = Format;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        unsafe {
            let ptr = av_output_audio_device_next(self.0);

            if ptr.is_null() && !self.0.is_null() {
                None
            } else {
                self.0 = ptr;

                Some(Format::Output(format::Output::wrap(ptr)))
            }
        }
    }
}

pub fn audio() -> AudioIter {
    AudioIter(ptr::null_mut())
}

pub struct VideoIter(*mut AVOutputFormat);

impl Iterator for VideoIter {
    type Item = Format;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        unsafe {
            let ptr = av_output_video_device_next(self.0);

            if ptr.is_null() && !self.0.is_null() {
                None
            } else {
                self.0 = ptr;

                Some(Format::Output(format::Output::wrap(ptr)))
            }
        }
    }
}

pub fn video() -> VideoIter {
    VideoIter(ptr::null_mut())
}
