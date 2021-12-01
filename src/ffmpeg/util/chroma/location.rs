// use ffi::AVChromaLocation::*;
use crate::ffmpeg::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Location {
    Unspecified,
    Left,
    Center,
    TopLeft,
    Top,
    BottomLeft,
    Bottom,
}

impl From<AVChromaLocation> for Location {
    fn from(value: AVChromaLocation) -> Self {
        match value {
            AVChromaLocation::AVCHROMA_LOC_UNSPECIFIED => Location::Unspecified,
            AVChromaLocation::AVCHROMA_LOC_LEFT => Location::Left,
            AVChromaLocation::AVCHROMA_LOC_CENTER => Location::Center,
            AVChromaLocation::AVCHROMA_LOC_TOPLEFT => Location::TopLeft,
            AVChromaLocation::AVCHROMA_LOC_TOP => Location::Top,
            AVChromaLocation::AVCHROMA_LOC_BOTTOMLEFT => Location::BottomLeft,
            AVChromaLocation::AVCHROMA_LOC_BOTTOM => Location::Bottom,
            AVChromaLocation::AVCHROMA_LOC_NB => Location::Unspecified,
        }
    }
}

impl From<Location> for AVChromaLocation {
    fn from(value: Location) -> AVChromaLocation {
        match value {
            Location::Unspecified => AVChromaLocation::AVCHROMA_LOC_UNSPECIFIED,
            Location::Left => AVChromaLocation::AVCHROMA_LOC_LEFT,
            Location::Center => AVChromaLocation::AVCHROMA_LOC_CENTER,
            Location::TopLeft => AVChromaLocation::AVCHROMA_LOC_TOPLEFT,
            Location::Top => AVChromaLocation::AVCHROMA_LOC_TOP,
            Location::BottomLeft => AVChromaLocation::AVCHROMA_LOC_BOTTOMLEFT,
            Location::Bottom => AVChromaLocation::AVCHROMA_LOC_BOTTOM,
        }
    }
}
