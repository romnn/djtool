// use ffi::AVRounding::*;
use crate::ffmpeg::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum Rounding {
    Zero,
    Infinity,
    Down,
    Up,
    NearInfinity,
    PassMinMax,
}

impl From<AVRounding> for Rounding {
    #[inline(always)]
    fn from(value: AVRounding) -> Self {
        match value {
            AVRounding::AV_ROUND_ZERO => Rounding::Zero,
            AVRounding::AV_ROUND_INF => Rounding::Infinity,
            AVRounding::AV_ROUND_DOWN => Rounding::Down,
            AVRounding::AV_ROUND_UP => Rounding::Up,
            AVRounding::AV_ROUND_NEAR_INF => Rounding::NearInfinity,
            AVRounding::AV_ROUND_PASS_MINMAX => Rounding::PassMinMax,
        }
    }
}

impl From<Rounding> for AVRounding {
    #[inline(always)]
    fn from(value: Rounding) -> AVRounding {
        match value {
            Rounding::Zero => AVRounding::AV_ROUND_ZERO,
            Rounding::Infinity => AVRounding::AV_ROUND_INF,
            Rounding::Down => AVRounding::AV_ROUND_DOWN,
            Rounding::Up => AVRounding::AV_ROUND_UP,
            Rounding::NearInfinity => AVRounding::AV_ROUND_NEAR_INF,
            Rounding::PassMinMax => AVRounding::AV_ROUND_PASS_MINMAX,
        }
    }
}
