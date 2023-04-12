pub mod decoder;
pub use decoder::Decoder;

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
use crate::{codec::Context, codec::Id, Codec};

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
