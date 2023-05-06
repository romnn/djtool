use crate::{AVRational, AV_TIME_BASE};
use libc::c_int;

// pub const AV_NOPTS_VALUE: i64 = 0x8000_0000_0000_0000_u64 as i64;
pub const AV_NOPTS_VALUE: i64 = -9_223_372_036_854_775_808_i64;

pub const AV_TIME_BASE_Q: AVRational = AVRational {
    num: 1,
    den: AV_TIME_BASE as c_int,
};
