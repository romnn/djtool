use std::ffi::CStr;
use std::marker::PhantomData;
use std::slice;
use std::str::from_utf8_unchecked;

use super::Frame;
use crate::ffi::*;
use crate::DictionaryRef;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Type {
    PanScan,
    A53CC,
    Stereo3D,
    MatrixEncoding,
    DownMixInfo,
    ReplayGain,
    DisplayMatrix,
    AFD,
    MotionVectors,
    SkipSamples,
    AudioServiceType,
    MasteringDisplayMetadata,
    GOPTimecode,
    Spherical,

    ContentLightLevel,
    IccProfile,

    // #[cfg(feature = "ffmpeg_4_0")]
    QPTableProperties,
    // #[cfg(feature = "ffmpeg_4_0")]
    QPTableData,

    // #[cfg(feature = "ffmpeg_4_1")]
    S12mTimecode,

    // #[cfg(feature = "ffmpeg_4_2")]
    DynamicHdrPlus,
    // #[cfg(feature = "ffmpeg_4_2")]
    RegionsOfInterest,

    // #[cfg(feature = "ffmpeg_4_3")]
    VideoEncParams,

    // #[cfg(feature = "ffmpeg_4_4")]
    SeiUnregistered,
    // #[cfg(feature = "ffmpeg_4_4")]
    FilmGrainParams,
}

impl Type {
    #[inline]
    pub fn name(&self) -> &'static str {
        unsafe {
            from_utf8_unchecked(CStr::from_ptr(av_frame_side_data_name((*self).into())).to_bytes())
        }
    }
}

impl From<AVFrameSideDataType> for Type {
    #[inline(always)]
    fn from(value: AVFrameSideDataType) -> Self {
        match value {
            AVFrameSideDataType::AV_FRAME_DATA_PANSCAN => Type::PanScan,
            AVFrameSideDataType::AV_FRAME_DATA_A53_CC => Type::A53CC,
            AVFrameSideDataType::AV_FRAME_DATA_STEREO3D => Type::Stereo3D,
            AVFrameSideDataType::AV_FRAME_DATA_MATRIXENCODING => Type::MatrixEncoding,
            AVFrameSideDataType::AV_FRAME_DATA_DOWNMIX_INFO => Type::DownMixInfo,
            AVFrameSideDataType::AV_FRAME_DATA_REPLAYGAIN => Type::ReplayGain,
            AVFrameSideDataType::AV_FRAME_DATA_DISPLAYMATRIX => Type::DisplayMatrix,
            AVFrameSideDataType::AV_FRAME_DATA_AFD => Type::AFD,
            AVFrameSideDataType::AV_FRAME_DATA_MOTION_VECTORS => Type::MotionVectors,
            AVFrameSideDataType::AV_FRAME_DATA_SKIP_SAMPLES => Type::SkipSamples,
            AVFrameSideDataType::AV_FRAME_DATA_AUDIO_SERVICE_TYPE => Type::AudioServiceType,
            AVFrameSideDataType::AV_FRAME_DATA_MASTERING_DISPLAY_METADATA => {
                Type::MasteringDisplayMetadata
            }
            AVFrameSideDataType::AV_FRAME_DATA_GOP_TIMECODE => Type::GOPTimecode,
            AVFrameSideDataType::AV_FRAME_DATA_SPHERICAL => Type::Spherical,

            AVFrameSideDataType::AV_FRAME_DATA_CONTENT_LIGHT_LEVEL => Type::ContentLightLevel,
            AVFrameSideDataType::AV_FRAME_DATA_ICC_PROFILE => Type::IccProfile,

            // #[cfg(feature = "ffmpeg_4_0")]
            AVFrameSideDataType::AV_FRAME_DATA_QP_TABLE_PROPERTIES => Type::QPTableProperties,
            // #[cfg(feature = "ffmpeg_4_0")]
            AVFrameSideDataType::AV_FRAME_DATA_QP_TABLE_DATA => Type::QPTableData,

            // #[cfg(feature = "ffmpeg_4_1")]
            AVFrameSideDataType::AV_FRAME_DATA_S12M_TIMECODE => Type::S12mTimecode,

            // #[cfg(feature = "ffmpeg_4_2")]
            AVFrameSideDataType::AV_FRAME_DATA_DYNAMIC_HDR_PLUS => Type::DynamicHdrPlus,
            // #[cfg(feature = "ffmpeg_4_2")]
            AVFrameSideDataType::AV_FRAME_DATA_REGIONS_OF_INTEREST => Type::RegionsOfInterest,

            // #[cfg(feature = "ffmpeg_4_3")]
            AVFrameSideDataType::AV_FRAME_DATA_VIDEO_ENC_PARAMS => Type::VideoEncParams,

            // #[cfg(feature = "ffmpeg_4_4")]
            AVFrameSideDataType::AV_FRAME_DATA_SEI_UNREGISTERED => Type::SeiUnregistered,
            // #[cfg(feature = "ffmpeg_4_4")]
            AVFrameSideDataType::AV_FRAME_DATA_FILM_GRAIN_PARAMS => Type::FilmGrainParams,
        }
    }
}

impl From<Type> for AVFrameSideDataType {
    #[inline(always)]
    fn from(value: Type) -> AVFrameSideDataType {
        match value {
            Type::PanScan => AVFrameSideDataType::AV_FRAME_DATA_PANSCAN,
            Type::A53CC => AVFrameSideDataType::AV_FRAME_DATA_A53_CC,
            Type::Stereo3D => AVFrameSideDataType::AV_FRAME_DATA_STEREO3D,
            Type::MatrixEncoding => AVFrameSideDataType::AV_FRAME_DATA_MATRIXENCODING,
            Type::DownMixInfo => AVFrameSideDataType::AV_FRAME_DATA_DOWNMIX_INFO,
            Type::ReplayGain => AVFrameSideDataType::AV_FRAME_DATA_REPLAYGAIN,
            Type::DisplayMatrix => AVFrameSideDataType::AV_FRAME_DATA_DISPLAYMATRIX,
            Type::AFD => AVFrameSideDataType::AV_FRAME_DATA_AFD,
            Type::MotionVectors => AVFrameSideDataType::AV_FRAME_DATA_MOTION_VECTORS,
            Type::SkipSamples => AVFrameSideDataType::AV_FRAME_DATA_SKIP_SAMPLES,
            Type::AudioServiceType => AVFrameSideDataType::AV_FRAME_DATA_AUDIO_SERVICE_TYPE,
            Type::MasteringDisplayMetadata => {
                AVFrameSideDataType::AV_FRAME_DATA_MASTERING_DISPLAY_METADATA
            }
            Type::GOPTimecode => AVFrameSideDataType::AV_FRAME_DATA_GOP_TIMECODE,
            Type::Spherical => AVFrameSideDataType::AV_FRAME_DATA_SPHERICAL,

            Type::ContentLightLevel => AVFrameSideDataType::AV_FRAME_DATA_CONTENT_LIGHT_LEVEL,
            Type::IccProfile => AVFrameSideDataType::AV_FRAME_DATA_ICC_PROFILE,

            // #[cfg(feature = "ffmpeg_4_0")]
            Type::QPTableProperties => AVFrameSideDataType::AV_FRAME_DATA_QP_TABLE_PROPERTIES,
            // #[cfg(feature = "ffmpeg_4_0")]
            Type::QPTableData => AVFrameSideDataType::AV_FRAME_DATA_QP_TABLE_DATA,

            // #[cfg(feature = "ffmpeg_4_1")]
            Type::S12mTimecode => AVFrameSideDataType::AV_FRAME_DATA_S12M_TIMECODE,

            // #[cfg(feature = "ffmpeg_4_2")]
            Type::DynamicHdrPlus => AVFrameSideDataType::AV_FRAME_DATA_DYNAMIC_HDR_PLUS,
            // #[cfg(feature = "ffmpeg_4_2")]
            Type::RegionsOfInterest => AVFrameSideDataType::AV_FRAME_DATA_REGIONS_OF_INTEREST,

            // #[cfg(feature = "ffmpeg_4_3")]
            Type::VideoEncParams => AVFrameSideDataType::AV_FRAME_DATA_VIDEO_ENC_PARAMS,

            // #[cfg(feature = "ffmpeg_4_4")]
            Type::SeiUnregistered => AVFrameSideDataType::AV_FRAME_DATA_SEI_UNREGISTERED,
            // #[cfg(feature = "ffmpeg_4_4")]
            Type::FilmGrainParams => AVFrameSideDataType::AV_FRAME_DATA_FILM_GRAIN_PARAMS,
        }
    }
}

pub struct SideData<'a> {
    ptr: *mut AVFrameSideData,

    _marker: PhantomData<&'a Frame>,
}

impl<'a> SideData<'a> {
    #[inline(always)]
    pub unsafe fn wrap(ptr: *mut AVFrameSideData) -> Self {
        SideData {
            ptr,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub unsafe fn as_ptr(&self) -> *const AVFrameSideData {
        self.ptr as *const _
    }

    #[inline(always)]
    pub unsafe fn as_mut_ptr(&mut self) -> *mut AVFrameSideData {
        self.ptr
    }
}

impl<'a> SideData<'a> {
    #[inline]
    pub fn kind(&self) -> Type {
        unsafe { Type::from((*self.as_ptr()).type_) }
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        unsafe { slice::from_raw_parts((*self.as_ptr()).data, (*self.as_ptr()).size as usize) }
    }

    #[inline]
    pub fn metadata(&self) -> DictionaryRef {
        unsafe { DictionaryRef::wrap((*self.as_ptr()).metadata) }
    }
}
