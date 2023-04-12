use std::ffi::CStr;
use std::str::from_utf8_unchecked;

use crate::ffi::*;

#[allow(non_camel_case_types)]
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum TransferCharacteristic {
    Reserved0,
    BT709,
    Unspecified,
    Reserved,
    GAMMA22,
    GAMMA28,
    SMPTE170M,
    SMPTE240M,
    Linear,
    Log,
    LogSqrt,
    IEC61966_2_4,
    BT1361_ECG,
    IEC61966_2_1,
    BT2020_10,
    BT2020_12,
    SMPTE2084,
    SMPTE428,
    ARIB_STD_B67,
}

impl TransferCharacteristic {
    pub fn name(&self) -> Option<&'static str> {
        if *self == TransferCharacteristic::Unspecified {
            return None;
        }
        unsafe {
            let ptr = av_color_transfer_name((*self).into());
            ptr.as_ref()
                .map(|ptr| from_utf8_unchecked(CStr::from_ptr(ptr).to_bytes()))
        }
    }
}

impl From<AVColorTransferCharacteristic> for TransferCharacteristic {
    fn from(value: AVColorTransferCharacteristic) -> TransferCharacteristic {
        match value {
            AVColorTransferCharacteristic::AVCOL_TRC_RESERVED0 => TransferCharacteristic::Reserved0,
            AVColorTransferCharacteristic::AVCOL_TRC_BT709 => TransferCharacteristic::BT709,
            AVColorTransferCharacteristic::AVCOL_TRC_UNSPECIFIED => {
                TransferCharacteristic::Unspecified
            }
            AVColorTransferCharacteristic::AVCOL_TRC_RESERVED => TransferCharacteristic::Reserved,
            AVColorTransferCharacteristic::AVCOL_TRC_GAMMA22 => TransferCharacteristic::GAMMA22,
            AVColorTransferCharacteristic::AVCOL_TRC_GAMMA28 => TransferCharacteristic::GAMMA28,
            AVColorTransferCharacteristic::AVCOL_TRC_SMPTE170M => TransferCharacteristic::SMPTE170M,
            AVColorTransferCharacteristic::AVCOL_TRC_SMPTE240M => TransferCharacteristic::SMPTE240M,
            AVColorTransferCharacteristic::AVCOL_TRC_LINEAR => TransferCharacteristic::Linear,
            AVColorTransferCharacteristic::AVCOL_TRC_LOG => TransferCharacteristic::Log,
            AVColorTransferCharacteristic::AVCOL_TRC_LOG_SQRT => TransferCharacteristic::LogSqrt,
            AVColorTransferCharacteristic::AVCOL_TRC_IEC61966_2_4 => {
                TransferCharacteristic::IEC61966_2_4
            }
            AVColorTransferCharacteristic::AVCOL_TRC_BT1361_ECG => {
                TransferCharacteristic::BT1361_ECG
            }
            AVColorTransferCharacteristic::AVCOL_TRC_IEC61966_2_1 => {
                TransferCharacteristic::IEC61966_2_1
            }
            AVColorTransferCharacteristic::AVCOL_TRC_BT2020_10 => TransferCharacteristic::BT2020_10,
            AVColorTransferCharacteristic::AVCOL_TRC_BT2020_12 => TransferCharacteristic::BT2020_12,
            AVColorTransferCharacteristic::AVCOL_TRC_NB => TransferCharacteristic::Reserved0,
            AVColorTransferCharacteristic::AVCOL_TRC_SMPTE2084 => TransferCharacteristic::SMPTE2084,
            AVColorTransferCharacteristic::AVCOL_TRC_SMPTE428 => TransferCharacteristic::SMPTE428,
            AVColorTransferCharacteristic::AVCOL_TRC_ARIB_STD_B67 => {
                TransferCharacteristic::ARIB_STD_B67
            }
        }
    }
}

impl From<TransferCharacteristic> for AVColorTransferCharacteristic {
    fn from(value: TransferCharacteristic) -> AVColorTransferCharacteristic {
        match value {
            TransferCharacteristic::Reserved0 => AVColorTransferCharacteristic::AVCOL_TRC_RESERVED0,
            TransferCharacteristic::BT709 => AVColorTransferCharacteristic::AVCOL_TRC_BT709,
            TransferCharacteristic::Unspecified => {
                AVColorTransferCharacteristic::AVCOL_TRC_UNSPECIFIED
            }
            TransferCharacteristic::Reserved => AVColorTransferCharacteristic::AVCOL_TRC_RESERVED,
            TransferCharacteristic::GAMMA22 => AVColorTransferCharacteristic::AVCOL_TRC_GAMMA22,
            TransferCharacteristic::GAMMA28 => AVColorTransferCharacteristic::AVCOL_TRC_GAMMA28,
            TransferCharacteristic::SMPTE170M => AVColorTransferCharacteristic::AVCOL_TRC_SMPTE170M,
            TransferCharacteristic::SMPTE240M => AVColorTransferCharacteristic::AVCOL_TRC_SMPTE240M,
            TransferCharacteristic::Linear => AVColorTransferCharacteristic::AVCOL_TRC_LINEAR,
            TransferCharacteristic::Log => AVColorTransferCharacteristic::AVCOL_TRC_LOG,
            TransferCharacteristic::LogSqrt => AVColorTransferCharacteristic::AVCOL_TRC_LOG_SQRT,
            TransferCharacteristic::IEC61966_2_4 => {
                AVColorTransferCharacteristic::AVCOL_TRC_IEC61966_2_4
            }
            TransferCharacteristic::BT1361_ECG => {
                AVColorTransferCharacteristic::AVCOL_TRC_BT1361_ECG
            }
            TransferCharacteristic::IEC61966_2_1 => {
                AVColorTransferCharacteristic::AVCOL_TRC_IEC61966_2_1
            }
            TransferCharacteristic::BT2020_10 => AVColorTransferCharacteristic::AVCOL_TRC_BT2020_10,
            TransferCharacteristic::BT2020_12 => AVColorTransferCharacteristic::AVCOL_TRC_BT2020_12,
            TransferCharacteristic::SMPTE2084 => AVColorTransferCharacteristic::AVCOL_TRC_SMPTE2084,
            TransferCharacteristic::SMPTE428 => AVColorTransferCharacteristic::AVCOL_TRC_SMPTE428,
            TransferCharacteristic::ARIB_STD_B67 => {
                AVColorTransferCharacteristic::AVCOL_TRC_ARIB_STD_B67
            }
        }
    }
}
