#![allow(clippy::missing_safety_doc, clippy::must_use_candidate)]
#![allow(
    warnings,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery
)]

use djtool_ffmpeg_sys as ffi;

pub mod util;
pub use util::channel_layout::{self, ChannelLayout};
pub use util::chroma;
pub use util::color;
pub use util::dictionary;
pub use util::dictionary::Mut as DictionaryMut;
pub use util::dictionary::Owned as Dictionary;
pub use util::dictionary::Ref as DictionaryRef;
pub use util::error::{self, Error};
pub use util::frame::{self, Frame};
pub use util::log;
pub use util::mathematics::{self, rescale, Rescale, Rounding};
pub use util::media;
pub use util::option;
pub use util::picture;
pub use util::rational::{self, Rational};
pub use util::time;

pub mod format;
pub use format::Format;
pub use format::chapter::{Chapter, ChapterMut};
pub use format::stream::{Stream, StreamMut};

pub mod codec;
pub use codec::audio_service::AudioService;
pub use codec::Codec;
pub use codec::discard::Discard;
pub use codec::field_order::FieldOrder;
pub use codec::packet::{self, Packet};
pub use codec::picture::Picture;
pub use codec::subtitle::{self, Subtitle};
pub use codec::threading;
pub use codec::{decoder, encoder};

pub mod device;

pub mod filter;
pub use filter::Filter;

pub mod software;

fn init_error() {
    util::error::register_all();
}

fn init_format() {
    format::register_all();
}

fn init_device() {
    device::register_all();
}

fn init_filter() {
    filter::register_all();
}

pub fn init() -> Result<(), Error> {
    init_error();
    init_format();
    init_device();
    init_filter();
    Ok(())
}
