#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::approx_constant)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

extern crate libc;
// use lazy_static::lazy_static;

// lazy_static! {
//     /// This is an example for using doc comment attributes
//     static ref TEST: &'static str = concat!(env!("OUT_DIR"), "/bindings.rs");
// }
// panic!(format!("{}", TEST));

#[allow(dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

#[macro_use]
mod avutil;
pub use avutil::*;
