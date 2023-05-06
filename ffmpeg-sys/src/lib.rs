#![allow(clippy::missing_safety_doc)]

#[allow(
    warnings,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    clippy::nursery
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

mod avutil;
pub use avutil::*;
