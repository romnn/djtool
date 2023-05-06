pub mod flag;
pub use self::flag::Flags;

pub mod id;
pub use self::id::Id;

pub mod packet;

pub mod subtitle;

pub mod picture;

pub mod discard;

pub mod context;
pub use self::context::Context;

pub mod capabilities;
pub use self::capabilities::Capabilities;

pub mod parameters;
pub use self::parameters::Parameters;

pub mod video;
pub use self::video::Video;

pub mod audio;
pub use self::audio::Audio;

pub mod audio_service;
pub mod field_order;

pub mod compliance;
pub use self::compliance::Compliance;

pub mod debug;
pub use self::debug::Debug;

pub mod profile;
pub use self::profile::Profile;

pub mod threading;

pub mod decoder;
pub mod encoder;
pub mod traits;

use std::ffi::CStr;
use std::str::from_utf8_unchecked;

use crate::ffi::*;
use crate::{media, Error};

#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Codec {
    ptr: *mut AVCodec,
}

unsafe impl Send for Codec {}
unsafe impl Sync for Codec {}

impl Codec {
    pub unsafe fn wrap(ptr: *mut AVCodec) -> Self {
        Codec { ptr }
    }

    pub unsafe fn as_ptr(&self) -> *const AVCodec {
        self.ptr as *const _
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut AVCodec {
        self.ptr
    }
}

impl Codec {
    pub fn is_encoder(&self) -> bool {
        unsafe { av_codec_is_encoder(self.as_ptr()) != 0 }
    }

    pub fn is_decoder(&self) -> bool {
        unsafe { av_codec_is_decoder(self.as_ptr()) != 0 }
    }

    pub fn name(&self) -> &str {
        unsafe { from_utf8_unchecked(CStr::from_ptr((*self.as_ptr()).name).to_bytes()) }
    }

    pub fn description(&self) -> &str {
        unsafe {
            let long_name = (*self.as_ptr()).long_name;
            if long_name.is_null() {
                ""
            } else {
                from_utf8_unchecked(CStr::from_ptr(long_name).to_bytes())
            }
        }
    }

    pub fn medium(&self) -> media::Type {
        unsafe { media::Type::from((*self.as_ptr()).type_) }
    }

    pub fn id(&self) -> Id {
        unsafe { Id::from((*self.as_ptr()).id) }
    }

    pub fn is_video(&self) -> bool {
        self.medium() == media::Type::Video
    }

    pub fn video(self) -> Result<Video, Error> {
        unsafe {
            if self.medium() == media::Type::Video {
                Ok(Video::new(self))
            } else {
                Err(Error::InvalidData)
            }
        }
    }

    pub fn is_audio(&self) -> bool {
        self.medium() == media::Type::Audio
    }

    pub fn audio(self) -> Result<Audio, Error> {
        unsafe {
            if self.medium() == media::Type::Audio {
                Ok(Audio::new(self))
            } else {
                Err(Error::InvalidData)
            }
        }
    }

    pub fn max_lowres(&self) -> i32 {
        unsafe { av_codec_get_max_lowres(self.as_ptr()) }
    }

    pub fn capabilities(&self) -> Capabilities {
        unsafe { Capabilities::from_bits_truncate((*self.as_ptr()).capabilities as u32) }
    }

    pub fn profiles(&self) -> Option<ProfileIter> {
        unsafe {
            if (*self.as_ptr()).profiles.is_null() {
                None
            } else {
                Some(ProfileIter::new(self.id(), (*self.as_ptr()).profiles))
            }
        }
    }
}

pub struct ProfileIter {
    id: Id,
    ptr: *const AVProfile,
}

impl ProfileIter {
    pub fn new(id: Id, ptr: *const AVProfile) -> Self {
        ProfileIter { id, ptr }
    }
}

impl Iterator for ProfileIter {
    type Item = Profile;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        unsafe {
            if (*self.ptr).profile == FF_PROFILE_UNKNOWN {
                return None;
            }

            let profile = Profile::from((self.id, (*self.ptr).profile));
            self.ptr = self.ptr.offset(1);

            Some(profile)
        }
    }
}

pub fn version() -> u32 {
    unsafe { avcodec_version() }
}

pub fn configuration() -> &'static str {
    unsafe { from_utf8_unchecked(CStr::from_ptr(avcodec_configuration()).to_bytes()) }
}

pub fn license() -> &'static str {
    unsafe { from_utf8_unchecked(CStr::from_ptr(avcodec_license()).to_bytes()) }
}
