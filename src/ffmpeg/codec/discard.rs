// use ffi::AVDiscard::*;
use crate::ffmpeg::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Discard {
    None,
    Default,
    NonReference,
    Bidirectional,
    NonIntra,
    NonKey,
    All,
}

impl From<AVDiscard> for Discard {
    fn from(value: AVDiscard) -> Self {
        match value {
            AVDiscard::AVDISCARD_NONE => Discard::None,
            AVDiscard::AVDISCARD_DEFAULT => Discard::Default,
            AVDiscard::AVDISCARD_NONREF => Discard::NonReference,
            AVDiscard::AVDISCARD_BIDIR => Discard::Bidirectional,
            AVDiscard::AVDISCARD_NONINTRA => Discard::NonIntra,
            AVDiscard::AVDISCARD_NONKEY => Discard::NonKey,
            AVDiscard::AVDISCARD_ALL => Discard::All,
        }
    }
}

impl From<Discard> for AVDiscard {
    fn from(value: Discard) -> AVDiscard {
        match value {
            Discard::None => AVDiscard::AVDISCARD_NONE,
            Discard::Default => AVDiscard::AVDISCARD_DEFAULT,
            Discard::NonReference => AVDiscard::AVDISCARD_NONREF,
            Discard::Bidirectional => AVDiscard::AVDISCARD_BIDIR,
            Discard::NonIntra => AVDiscard::AVDISCARD_NONINTRA,
            Discard::NonKey => AVDiscard::AVDISCARD_NONKEY,
            Discard::All => AVDiscard::AVDISCARD_ALL,
        }
    }
}
