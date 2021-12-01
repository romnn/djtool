mod traits;
pub use traits::{Gettable, Iterable, Settable, Target};

use crate::ffmpeg::ffi::*;
// use ffi::AVOptionType::*;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Type {
    Flags,
    Int,
    Int64,
    Double,
    Float,
    String,
    Rational,
    Binary,
    Dictionary,
    Constant,

    ImageSize,
    PixelFormat,
    SampleFormat,
    VideoRate,
    Duration,
    Color,
    ChannelLayout,
    c_ulong,
    bool,
}

impl From<AVOptionType> for Type {
    fn from(value: AVOptionType) -> Self {
        match value {
            AVOptionType::AV_OPT_TYPE_FLAGS => Type::Flags,
            AVOptionType::AV_OPT_TYPE_INT => Type::Int,
            AVOptionType::AV_OPT_TYPE_INT64 => Type::Int64,
            AVOptionType::AV_OPT_TYPE_DOUBLE => Type::Double,
            AVOptionType::AV_OPT_TYPE_FLOAT => Type::Float,
            AVOptionType::AV_OPT_TYPE_STRING => Type::String,
            AVOptionType::AV_OPT_TYPE_RATIONAL => Type::Rational,
            AVOptionType::AV_OPT_TYPE_BINARY => Type::Binary,
            AVOptionType::AV_OPT_TYPE_DICT => Type::Dictionary,
            AVOptionType::AV_OPT_TYPE_CONST => Type::Constant,
            AVOptionType::AV_OPT_TYPE_UINT64 => Type::c_ulong,
            AVOptionType::AV_OPT_TYPE_BOOL => Type::bool,

            AVOptionType::AV_OPT_TYPE_IMAGE_SIZE => Type::ImageSize,
            AVOptionType::AV_OPT_TYPE_PIXEL_FMT => Type::PixelFormat,
            AVOptionType::AV_OPT_TYPE_SAMPLE_FMT => Type::SampleFormat,
            AVOptionType::AV_OPT_TYPE_VIDEO_RATE => Type::VideoRate,
            AVOptionType::AV_OPT_TYPE_DURATION => Type::Duration,
            AVOptionType::AV_OPT_TYPE_COLOR => Type::Color,
            AVOptionType::AV_OPT_TYPE_CHANNEL_LAYOUT => Type::ChannelLayout,
        }
    }
}

impl From<Type> for AVOptionType {
    fn from(value: Type) -> AVOptionType {
        match value {
            Type::Flags => AVOptionType::AV_OPT_TYPE_FLAGS,
            Type::Int => AVOptionType::AV_OPT_TYPE_INT,
            Type::Int64 => AVOptionType::AV_OPT_TYPE_INT64,
            Type::Double => AVOptionType::AV_OPT_TYPE_DOUBLE,
            Type::Float => AVOptionType::AV_OPT_TYPE_FLOAT,
            Type::String => AVOptionType::AV_OPT_TYPE_STRING,
            Type::Rational => AVOptionType::AV_OPT_TYPE_RATIONAL,
            Type::Binary => AVOptionType::AV_OPT_TYPE_BINARY,
            Type::Dictionary => AVOptionType::AV_OPT_TYPE_DICT,
            Type::Constant => AVOptionType::AV_OPT_TYPE_CONST,
            Type::c_ulong => AVOptionType::AV_OPT_TYPE_UINT64,
            Type::bool => AVOptionType::AV_OPT_TYPE_BOOL,

            Type::ImageSize => AVOptionType::AV_OPT_TYPE_IMAGE_SIZE,
            Type::PixelFormat => AVOptionType::AV_OPT_TYPE_PIXEL_FMT,
            Type::SampleFormat => AVOptionType::AV_OPT_TYPE_SAMPLE_FMT,
            Type::VideoRate => AVOptionType::AV_OPT_TYPE_VIDEO_RATE,
            Type::Duration => AVOptionType::AV_OPT_TYPE_DURATION,
            Type::Color => AVOptionType::AV_OPT_TYPE_COLOR,
            Type::ChannelLayout => AVOptionType::AV_OPT_TYPE_CHANNEL_LAYOUT,
        }
    }
}
