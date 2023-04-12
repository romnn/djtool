#[allow(warnings, dead_code)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub use bindings::*;

mod avutil;
pub use avutil::*;
