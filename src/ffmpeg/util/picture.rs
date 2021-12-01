// use ffi::AVPictureType::*;
use crate::ffmpeg::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Type {
    None,
    I,
    P,
    B,
    S,
    SI,
    SP,
    BI,
}

impl From<AVPictureType> for Type {
    #[inline(always)]
    fn from(value: AVPictureType) -> Type {
        match value {
            AVPictureType::AV_PICTURE_TYPE_NONE => Type::None,
            AVPictureType::AV_PICTURE_TYPE_I => Type::I,
            AVPictureType::AV_PICTURE_TYPE_P => Type::P,
            AVPictureType::AV_PICTURE_TYPE_B => Type::B,
            AVPictureType::AV_PICTURE_TYPE_S => Type::S,
            AVPictureType::AV_PICTURE_TYPE_SI => Type::SI,
            AVPictureType::AV_PICTURE_TYPE_SP => Type::SP,
            AVPictureType::AV_PICTURE_TYPE_BI => Type::BI,
        }
    }
}

impl From<Type> for AVPictureType {
    #[inline(always)]
    fn from(value: Type) -> AVPictureType {
        match value {
            Type::None => AVPictureType::AV_PICTURE_TYPE_NONE,
            Type::I => AVPictureType::AV_PICTURE_TYPE_I,
            Type::P => AVPictureType::AV_PICTURE_TYPE_P,
            Type::B => AVPictureType::AV_PICTURE_TYPE_B,
            Type::S => AVPictureType::AV_PICTURE_TYPE_S,
            Type::SI => AVPictureType::AV_PICTURE_TYPE_SI,
            Type::SP => AVPictureType::AV_PICTURE_TYPE_SP,
            Type::BI => AVPictureType::AV_PICTURE_TYPE_BI,
        }
    }
}
