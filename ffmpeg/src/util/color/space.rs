use std::ffi::CStr;
use std::str::from_utf8_unchecked;

use crate::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Space {
    RGB,
    BT709,
    Unspecified,
    Reserved,
    FCC,
    BT470BG,
    SMPTE170M,
    SMPTE240M,
    YCGCO,
    BT2020NCL,
    BT2020CL,
    SMPTE2085,

    ChromaDerivedNCL,
    ChromaDerivedCL,
    ICTCP,
}

impl Space {
    pub const YCOCG: Space = Space::YCGCO;

    pub fn name(&self) -> Option<&'static str> {
        if *self == Space::Unspecified {
            return None;
        }
        unsafe {
            let ptr = av_color_space_name((*self).into());
            ptr.as_ref()
                .map(|ptr| from_utf8_unchecked(CStr::from_ptr(ptr).to_bytes()))
        }
    }
}

impl From<AVColorSpace> for Space {
    fn from(value: AVColorSpace) -> Self {
        match value {
            AVColorSpace::AVCOL_SPC_RGB => Space::RGB,
            AVColorSpace::AVCOL_SPC_BT709 => Space::BT709,
            AVColorSpace::AVCOL_SPC_UNSPECIFIED => Space::Unspecified,
            AVColorSpace::AVCOL_SPC_RESERVED => Space::Reserved,
            AVColorSpace::AVCOL_SPC_FCC => Space::FCC,
            AVColorSpace::AVCOL_SPC_BT470BG => Space::BT470BG,
            AVColorSpace::AVCOL_SPC_SMPTE170M => Space::SMPTE170M,
            AVColorSpace::AVCOL_SPC_SMPTE240M => Space::SMPTE240M,
            AVColorSpace::AVCOL_SPC_YCGCO => Space::YCGCO,
            AVColorSpace::AVCOL_SPC_BT2020_NCL => Space::BT2020NCL,
            AVColorSpace::AVCOL_SPC_BT2020_CL => Space::BT2020CL,
            AVColorSpace::AVCOL_SPC_SMPTE2085 => Space::SMPTE2085,
            AVColorSpace::AVCOL_SPC_NB => Space::Unspecified,

            AVColorSpace::AVCOL_SPC_CHROMA_DERIVED_NCL => Space::ChromaDerivedNCL,
            AVColorSpace::AVCOL_SPC_CHROMA_DERIVED_CL => Space::ChromaDerivedCL,
            AVColorSpace::AVCOL_SPC_ICTCP => Space::ICTCP,
        }
    }
}

impl From<Space> for AVColorSpace {
    fn from(value: Space) -> AVColorSpace {
        match value {
            Space::RGB => AVColorSpace::AVCOL_SPC_RGB,
            Space::BT709 => AVColorSpace::AVCOL_SPC_BT709,
            Space::Unspecified => AVColorSpace::AVCOL_SPC_UNSPECIFIED,
            Space::Reserved => AVColorSpace::AVCOL_SPC_RESERVED,
            Space::FCC => AVColorSpace::AVCOL_SPC_FCC,
            Space::BT470BG => AVColorSpace::AVCOL_SPC_BT470BG,
            Space::SMPTE170M => AVColorSpace::AVCOL_SPC_SMPTE170M,
            Space::SMPTE240M => AVColorSpace::AVCOL_SPC_SMPTE240M,
            Space::YCGCO => AVColorSpace::AVCOL_SPC_YCGCO,
            Space::BT2020NCL => AVColorSpace::AVCOL_SPC_BT2020_NCL,
            Space::BT2020CL => AVColorSpace::AVCOL_SPC_BT2020_CL,
            Space::SMPTE2085 => AVColorSpace::AVCOL_SPC_SMPTE2085,

            Space::ChromaDerivedNCL => AVColorSpace::AVCOL_SPC_CHROMA_DERIVED_NCL,
            Space::ChromaDerivedCL => AVColorSpace::AVCOL_SPC_CHROMA_DERIVED_CL,
            Space::ICTCP => AVColorSpace::AVCOL_SPC_ICTCP,
        }
    }
}
