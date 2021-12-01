use crate::ffmpeg::ffi::*;
// use ffi::AVFieldOrder::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum FieldOrder {
    Unknown,
    Progressive,
    TT,
    BB,
    TB,
    BT,
}

impl From<AVFieldOrder> for FieldOrder {
    fn from(value: AVFieldOrder) -> Self {
        match value {
            AVFieldOrder::AV_FIELD_UNKNOWN => FieldOrder::Unknown,
            AVFieldOrder::AV_FIELD_PROGRESSIVE => FieldOrder::Progressive,
            AVFieldOrder::AV_FIELD_TT => FieldOrder::TT,
            AVFieldOrder::AV_FIELD_BB => FieldOrder::BB,
            AVFieldOrder::AV_FIELD_TB => FieldOrder::TB,
            AVFieldOrder::AV_FIELD_BT => FieldOrder::BT,
        }
    }
}

impl From<FieldOrder> for AVFieldOrder {
    fn from(value: FieldOrder) -> AVFieldOrder {
        match value {
            FieldOrder::Unknown => AVFieldOrder::AV_FIELD_UNKNOWN,
            FieldOrder::Progressive => AVFieldOrder::AV_FIELD_PROGRESSIVE,
            FieldOrder::TT => AVFieldOrder::AV_FIELD_TT,
            FieldOrder::BB => AVFieldOrder::AV_FIELD_BB,
            FieldOrder::TB => AVFieldOrder::AV_FIELD_TB,
            FieldOrder::BT => AVFieldOrder::AV_FIELD_BT,
        }
    }
}
