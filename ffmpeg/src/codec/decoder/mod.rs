pub mod video;
pub use video::Video;

pub mod audio;
pub use audio::Audio;

pub mod subtitle;
pub use subtitle::Subtitle;

pub mod slice;

pub mod conceal;
pub use conceal::Conceal;

pub mod check;
pub use check::Check;

pub mod opened;
pub use opened::Opened;

use std::ffi::CString;

use crate::ffi::*;
use std::ops::{Deref, DerefMut};
use std::ptr;

use crate::{
    codec::{traits, Context, Id},
    Codec, Dictionary, Discard, Error, Rational,
};

pub struct Decoder(pub Context);

impl Decoder {
    pub fn open(mut self) -> Result<Opened, Error> {
        unsafe {
            match avcodec_open2(self.as_mut_ptr(), ptr::null(), ptr::null_mut()) {
                0 => Ok(Opened(self)),
                e => Err(Error::from(e)),
            }
        }
    }

    pub fn open_as<D: traits::Decoder>(mut self, codec: D) -> Result<Opened, Error> {
        unsafe {
            if let Some(codec) = codec.decoder() {
                match avcodec_open2(self.as_mut_ptr(), codec.as_ptr(), ptr::null_mut()) {
                    0 => Ok(Opened(self)),
                    e => Err(Error::from(e)),
                }
            } else {
                Err(Error::DecoderNotFound)
            }
        }
    }

    pub fn open_as_with<D: traits::Decoder>(
        mut self,
        codec: D,
        options: Dictionary,
    ) -> Result<Opened, Error> {
        unsafe {
            if let Some(codec) = codec.decoder() {
                let mut opts = options.disown();
                let res = avcodec_open2(self.as_mut_ptr(), codec.as_ptr(), &mut opts);

                Dictionary::own(opts);

                match res {
                    0 => Ok(Opened(self)),
                    e => Err(Error::from(e)),
                }
            } else {
                Err(Error::DecoderNotFound)
            }
        }
    }

    pub fn video(self) -> Result<Video, Error> {
        if let Some(codec) = find(self.id()) {
            self.open_as(codec).and_then(|o| o.video())
        } else {
            Err(Error::DecoderNotFound)
        }
    }

    pub fn audio(self) -> Result<Audio, Error> {
        if let Some(codec) = find(self.id()) {
            self.open_as(codec).and_then(|o| o.audio())
        } else {
            Err(Error::DecoderNotFound)
        }
    }

    pub fn subtitle(self) -> Result<Subtitle, Error> {
        if let Some(codec) = find(self.id()) {
            self.open_as(codec).and_then(|o| o.subtitle())
        } else {
            Err(Error::DecoderNotFound)
        }
    }

    pub fn conceal(&mut self, value: Conceal) {
        unsafe {
            (*self.as_mut_ptr()).error_concealment = value.bits();
        }
    }

    pub fn check(&mut self, value: Check) {
        unsafe {
            (*self.as_mut_ptr()).err_recognition = value.bits();
        }
    }

    pub fn skip_loop_filter(&mut self, value: Discard) {
        unsafe {
            (*self.as_mut_ptr()).skip_loop_filter = value.into();
        }
    }

    pub fn skip_idct(&mut self, value: Discard) {
        unsafe {
            (*self.as_mut_ptr()).skip_idct = value.into();
        }
    }

    pub fn skip_frame(&mut self, value: Discard) {
        unsafe {
            (*self.as_mut_ptr()).skip_frame = value.into();
        }
    }

    pub fn time_base(&self) -> Rational {
        unsafe { Rational::from((*self.as_ptr()).time_base) }
    }
}

impl Deref for Decoder {
    type Target = Context;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.0
    }
}

impl DerefMut for Decoder {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.0
    }
}

impl AsRef<Context> for Decoder {
    fn as_ref(&self) -> &Context {
        self
    }
}

impl AsMut<Context> for Decoder {
    fn as_mut(&mut self) -> &mut Context {
        &mut self.0
    }
}

pub fn new() -> Decoder {
    Context::new().decoder()
}

pub fn find(id: Id) -> Option<Codec> {
    unsafe {
        let ptr = avcodec_find_decoder(id.into());

        if ptr.is_null() {
            None
        } else {
            Some(Codec::wrap(ptr))
        }
    }
}

pub fn find_by_name(name: &str) -> Option<Codec> {
    unsafe {
        let name = CString::new(name).unwrap();
        let ptr = avcodec_find_decoder_by_name(name.as_ptr());

        if ptr.is_null() {
            None
        } else {
            Some(Codec::wrap(ptr))
        }
    }
}
