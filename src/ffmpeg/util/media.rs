// use ffi::AVMediaType::*;
use crate::ffmpeg::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Type {
    Unknown,
    Video,
    Audio,
    Data,
    Subtitle,
    Attachment,
}

impl From<AVMediaType> for Type {
    #[inline(always)]
    fn from(value: AVMediaType) -> Self {
        match value {
            AVMediaType::AVMEDIA_TYPE_UNKNOWN => Type::Unknown,
            AVMediaType::AVMEDIA_TYPE_VIDEO => Type::Video,
            AVMediaType::AVMEDIA_TYPE_AUDIO => Type::Audio,
            AVMediaType::AVMEDIA_TYPE_DATA => Type::Data,
            AVMediaType::AVMEDIA_TYPE_SUBTITLE => Type::Subtitle,
            AVMediaType::AVMEDIA_TYPE_ATTACHMENT => Type::Attachment,
            AVMediaType::AVMEDIA_TYPE_NB => Type::Unknown,
        }
    }
}

impl From<Type> for AVMediaType {
    #[inline(always)]
    fn from(value: Type) -> AVMediaType {
        match value {
            Type::Unknown => AVMediaType::AVMEDIA_TYPE_UNKNOWN,
            Type::Video => AVMediaType::AVMEDIA_TYPE_VIDEO,
            Type::Audio => AVMediaType::AVMEDIA_TYPE_AUDIO,
            Type::Data => AVMediaType::AVMEDIA_TYPE_DATA,
            Type::Subtitle => AVMediaType::AVMEDIA_TYPE_SUBTITLE,
            Type::Attachment => AVMediaType::AVMEDIA_TYPE_ATTACHMENT,
        }
    }
}
