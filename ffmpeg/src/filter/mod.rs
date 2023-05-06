pub mod flag;
pub use flag::Flags;

pub mod pad;
pub use pad::Pad;

pub mod context;
pub use context::{Context, Sink, Source};

pub mod graph;
pub use graph::Graph;

use crate::ffi::*;
use crate::Error;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::str::from_utf8_unchecked;

pub fn register_all() {
    unsafe {
        avfilter_register_all();
    }
}

pub fn register(filter: &Filter) -> Result<(), Error> {
    unsafe {
        match avfilter_register(filter.as_ptr() as *mut _) {
            0 => Ok(()),
            _ => Err(Error::InvalidData),
        }
    }
}

pub fn version() -> u32 {
    unsafe { avfilter_version() }
}

pub fn configuration() -> &'static str {
    unsafe { from_utf8_unchecked(CStr::from_ptr(avfilter_configuration()).to_bytes()) }
}

pub fn license() -> &'static str {
    unsafe { from_utf8_unchecked(CStr::from_ptr(avfilter_license()).to_bytes()) }
}

pub fn find(name: &str) -> Option<Filter> {
    unsafe {
        let name = CString::new(name).unwrap();
        let ptr = avfilter_get_by_name(name.as_ptr());

        if ptr.is_null() {
            None
        } else {
            Some(Filter::wrap(ptr as *mut _))
        }
    }
}

pub struct Filter {
    ptr: *mut AVFilter,
}

impl Filter {
    pub unsafe fn wrap(ptr: *mut AVFilter) -> Self {
        Filter { ptr }
    }

    pub unsafe fn as_ptr(&self) -> *const AVFilter {
        self.ptr as *const _
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut AVFilter {
        self.ptr
    }
}

impl Filter {
    pub fn name(&self) -> &str {
        unsafe { from_utf8_unchecked(CStr::from_ptr((*self.as_ptr()).name).to_bytes()) }
    }

    pub fn description(&self) -> Option<&str> {
        unsafe {
            let ptr = (*self.as_ptr()).description;

            if ptr.is_null() {
                None
            } else {
                Some(from_utf8_unchecked(CStr::from_ptr(ptr).to_bytes()))
            }
        }
    }

    pub fn inputs(&self) -> Option<PadIter> {
        unsafe {
            let ptr = (*self.as_ptr()).inputs;

            if ptr.is_null() {
                None
            } else {
                Some(PadIter::new((*self.as_ptr()).inputs))
            }
        }
    }

    pub fn outputs(&self) -> Option<PadIter> {
        unsafe {
            let ptr = (*self.as_ptr()).outputs;

            if ptr.is_null() {
                None
            } else {
                Some(PadIter::new((*self.as_ptr()).outputs))
            }
        }
    }

    pub fn flags(&self) -> Flags {
        unsafe { Flags::from_bits_truncate((*self.as_ptr()).flags) }
    }
}

pub struct PadIter<'a> {
    ptr: *const AVFilterPad,
    cur: isize,

    _marker: PhantomData<&'a ()>,
}

impl<'a> PadIter<'a> {
    pub fn new(ptr: *const AVFilterPad) -> Self {
        PadIter {
            ptr,
            cur: 0,
            _marker: PhantomData,
        }
    }
}

impl<'a> Iterator for PadIter<'a> {
    type Item = Pad<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.cur >= avfilter_pad_count(self.ptr) as isize {
                return None;
            }

            let pad = Pad::wrap(self.ptr, self.cur);
            self.cur += 1;

            Some(pad)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paditer() {
        register_all();
        assert_eq!(
            find("overlay")
                .unwrap()
                .inputs()
                .unwrap()
                .map(|input| input.name().unwrap().to_string())
                .collect::<Vec<_>>(),
            vec!("main", "overlay")
        );
    }
}
