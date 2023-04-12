use std::marker::PhantomData;
use std::slice;

use super::Packet;
use crate::ffi::*;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Type {
    Palette,
    NewExtraData,
    ParamChange,
    H263MbInfo,
    ReplayGain,
    DisplayMatrix,
    Stereo3d,
    AudioServiceType,
    QualityStats,
    FallbackTrack,
    CBPProperties,
    SkipSamples,
    JpDualMono,
    StringsMetadata,
    SubtitlePosition,
    MatroskaBlockAdditional,
    WebVTTIdentifier,
    WebVTTSettings,
    MetadataUpdate,
    MPEGTSStreamID,
    MasteringDisplayMetadata,
    DataSpherical,
    DataNb,

    ContentLightLevel,
    A53CC,

    // #[cfg(feature = "ffmpeg_4_0")]
    EncryptionInitInfo,
    // #[cfg(feature = "ffmpeg_4_0")]
    EncryptionInfo,

    // #[cfg(feature = "ffmpeg_4_1")]
    AFD,

    // #[cfg(feature = "ffmpeg_4_3")]
    PRFT,
    // #[cfg(feature = "ffmpeg_4_3")]
    ICC_PROFILE,
    // #[cfg(feature = "ffmpeg_4_3")]
    DOVI_CONF,

    // #[cfg(feature = "ffmpeg_4_4")]
    S12M_TIMECODE,
}

impl From<AVPacketSideDataType> for Type {
    fn from(value: AVPacketSideDataType) -> Self {
        match value {
            AVPacketSideDataType::AV_PKT_DATA_PALETTE => Type::Palette,
            AVPacketSideDataType::AV_PKT_DATA_NEW_EXTRADATA => Type::NewExtraData,
            AVPacketSideDataType::AV_PKT_DATA_PARAM_CHANGE => Type::ParamChange,
            AVPacketSideDataType::AV_PKT_DATA_H263_MB_INFO => Type::H263MbInfo,
            AVPacketSideDataType::AV_PKT_DATA_REPLAYGAIN => Type::ReplayGain,
            AVPacketSideDataType::AV_PKT_DATA_DISPLAYMATRIX => Type::DisplayMatrix,
            AVPacketSideDataType::AV_PKT_DATA_STEREO3D => Type::Stereo3d,
            AVPacketSideDataType::AV_PKT_DATA_AUDIO_SERVICE_TYPE => Type::AudioServiceType,
            AVPacketSideDataType::AV_PKT_DATA_QUALITY_STATS => Type::QualityStats,
            AVPacketSideDataType::AV_PKT_DATA_FALLBACK_TRACK => Type::FallbackTrack,
            AVPacketSideDataType::AV_PKT_DATA_CPB_PROPERTIES => Type::CBPProperties,
            AVPacketSideDataType::AV_PKT_DATA_SKIP_SAMPLES => Type::SkipSamples,
            AVPacketSideDataType::AV_PKT_DATA_JP_DUALMONO => Type::JpDualMono,
            AVPacketSideDataType::AV_PKT_DATA_STRINGS_METADATA => Type::StringsMetadata,
            AVPacketSideDataType::AV_PKT_DATA_SUBTITLE_POSITION => Type::SubtitlePosition,
            AVPacketSideDataType::AV_PKT_DATA_MATROSKA_BLOCKADDITIONAL => {
                Type::MatroskaBlockAdditional
            }
            AVPacketSideDataType::AV_PKT_DATA_WEBVTT_IDENTIFIER => Type::WebVTTIdentifier,
            AVPacketSideDataType::AV_PKT_DATA_WEBVTT_SETTINGS => Type::WebVTTSettings,
            AVPacketSideDataType::AV_PKT_DATA_METADATA_UPDATE => Type::MetadataUpdate,
            AVPacketSideDataType::AV_PKT_DATA_MPEGTS_STREAM_ID => Type::MPEGTSStreamID,
            AVPacketSideDataType::AV_PKT_DATA_MASTERING_DISPLAY_METADATA => {
                Type::MasteringDisplayMetadata
            }
            AVPacketSideDataType::AV_PKT_DATA_SPHERICAL => Type::DataSpherical,
            AVPacketSideDataType::AV_PKT_DATA_NB => Type::DataNb,

            AVPacketSideDataType::AV_PKT_DATA_CONTENT_LIGHT_LEVEL => Type::ContentLightLevel,
            AVPacketSideDataType::AV_PKT_DATA_A53_CC => Type::A53CC,

            // #[cfg(feature = "ffmpeg_4_0")]
            AVPacketSideDataType::AV_PKT_DATA_ENCRYPTION_INIT_INFO => Type::EncryptionInitInfo,
            // #[cfg(feature = "ffmpeg_4_0")]
            AVPacketSideDataType::AV_PKT_DATA_ENCRYPTION_INFO => Type::EncryptionInfo,

            // #[cfg(feature = "ffmpeg_4_1")]
            AVPacketSideDataType::AV_PKT_DATA_AFD => Type::AFD,

            // #[cfg(feature = "ffmpeg_4_3")]
            AVPacketSideDataType::AV_PKT_DATA_PRFT => Type::PRFT,
            // #[cfg(feature = "ffmpeg_4_3")]
            AVPacketSideDataType::AV_PKT_DATA_ICC_PROFILE => Type::ICC_PROFILE,
            // #[cfg(feature = "ffmpeg_4_3")]
            AVPacketSideDataType::AV_PKT_DATA_DOVI_CONF => Type::DOVI_CONF,

            // #[cfg(feature = "ffmpeg_4_4")]
            AVPacketSideDataType::AV_PKT_DATA_S12M_TIMECODE => Type::S12M_TIMECODE,
        }
    }
}

impl From<Type> for AVPacketSideDataType {
    fn from(value: Type) -> AVPacketSideDataType {
        match value {
            Type::Palette => AVPacketSideDataType::AV_PKT_DATA_PALETTE,
            Type::NewExtraData => AVPacketSideDataType::AV_PKT_DATA_NEW_EXTRADATA,
            Type::ParamChange => AVPacketSideDataType::AV_PKT_DATA_PARAM_CHANGE,
            Type::H263MbInfo => AVPacketSideDataType::AV_PKT_DATA_H263_MB_INFO,
            Type::ReplayGain => AVPacketSideDataType::AV_PKT_DATA_REPLAYGAIN,
            Type::DisplayMatrix => AVPacketSideDataType::AV_PKT_DATA_DISPLAYMATRIX,
            Type::Stereo3d => AVPacketSideDataType::AV_PKT_DATA_STEREO3D,
            Type::AudioServiceType => AVPacketSideDataType::AV_PKT_DATA_AUDIO_SERVICE_TYPE,
            Type::QualityStats => AVPacketSideDataType::AV_PKT_DATA_QUALITY_STATS,
            Type::FallbackTrack => AVPacketSideDataType::AV_PKT_DATA_FALLBACK_TRACK,
            Type::CBPProperties => AVPacketSideDataType::AV_PKT_DATA_CPB_PROPERTIES,
            Type::SkipSamples => AVPacketSideDataType::AV_PKT_DATA_SKIP_SAMPLES,
            Type::JpDualMono => AVPacketSideDataType::AV_PKT_DATA_JP_DUALMONO,
            Type::StringsMetadata => AVPacketSideDataType::AV_PKT_DATA_STRINGS_METADATA,
            Type::SubtitlePosition => AVPacketSideDataType::AV_PKT_DATA_SUBTITLE_POSITION,
            Type::MatroskaBlockAdditional => {
                AVPacketSideDataType::AV_PKT_DATA_MATROSKA_BLOCKADDITIONAL
            }
            Type::WebVTTIdentifier => AVPacketSideDataType::AV_PKT_DATA_WEBVTT_IDENTIFIER,
            Type::WebVTTSettings => AVPacketSideDataType::AV_PKT_DATA_WEBVTT_SETTINGS,
            Type::MetadataUpdate => AVPacketSideDataType::AV_PKT_DATA_METADATA_UPDATE,
            Type::MPEGTSStreamID => AVPacketSideDataType::AV_PKT_DATA_MPEGTS_STREAM_ID,
            Type::MasteringDisplayMetadata => {
                AVPacketSideDataType::AV_PKT_DATA_MASTERING_DISPLAY_METADATA
            }
            Type::DataSpherical => AVPacketSideDataType::AV_PKT_DATA_SPHERICAL,
            Type::DataNb => AVPacketSideDataType::AV_PKT_DATA_NB,

            Type::ContentLightLevel => AVPacketSideDataType::AV_PKT_DATA_CONTENT_LIGHT_LEVEL,
            Type::A53CC => AVPacketSideDataType::AV_PKT_DATA_A53_CC,

            // #[cfg(feature = "ffmpeg_4_0")]
            Type::EncryptionInitInfo => AVPacketSideDataType::AV_PKT_DATA_ENCRYPTION_INIT_INFO,
            // #[cfg(feature = "ffmpeg_4_0")]
            Type::EncryptionInfo => AVPacketSideDataType::AV_PKT_DATA_ENCRYPTION_INFO,

            // #[cfg(feature = "ffmpeg_4_1")]
            Type::AFD => AVPacketSideDataType::AV_PKT_DATA_AFD,

            // #[cfg(feature = "ffmpeg_4_3")]
            Type::PRFT => AVPacketSideDataType::AV_PKT_DATA_PRFT,
            // #[cfg(feature = "ffmpeg_4_3")]
            Type::ICC_PROFILE => AVPacketSideDataType::AV_PKT_DATA_ICC_PROFILE,
            // #[cfg(feature = "ffmpeg_4_3")]
            Type::DOVI_CONF => AVPacketSideDataType::AV_PKT_DATA_DOVI_CONF,

            // #[cfg(feature = "ffmpeg_4_4")]
            Type::S12M_TIMECODE => AVPacketSideDataType::AV_PKT_DATA_S12M_TIMECODE,
        }
    }
}

pub struct SideData<'a> {
    ptr: *mut AVPacketSideData,

    _marker: PhantomData<&'a Packet>,
}

impl<'a> SideData<'a> {
    pub unsafe fn wrap(ptr: *mut AVPacketSideData) -> Self {
        SideData {
            ptr,
            _marker: PhantomData,
        }
    }

    pub unsafe fn as_ptr(&self) -> *const AVPacketSideData {
        self.ptr as *const _
    }
}

impl<'a> SideData<'a> {
    pub fn kind(&self) -> Type {
        unsafe { Type::from((*self.as_ptr()).type_) }
    }

    pub fn data(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((*self.as_ptr()).data, (*self.as_ptr()).size as usize) }
    }
}
