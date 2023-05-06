pub mod disposition;
pub use disposition::Disposition;

use crate::ffi::*;
use crate::{
    codec::{self, packet},
    format::context::common::Context,
    Dictionary, DictionaryRef, Discard, Rational,
};
use libc::c_int;
use std::mem;
use std::ops::Deref;

#[derive(Debug)]
pub struct Stream<'a> {
    context: &'a Context,
    index: usize,
}

impl<'a> Stream<'a> {
    pub unsafe fn wrap(context: &Context, index: usize) -> Stream {
        Stream { context, index }
    }

    pub unsafe fn as_ptr(&self) -> *const AVStream {
        *(*self.context.as_ptr()).streams.add(self.index)
    }
}

impl<'a> Stream<'a> {
    pub fn id(&self) -> i32 {
        unsafe { (*self.as_ptr()).id }
    }

    pub fn codec(&self) -> codec::Context {
        unsafe { codec::Context::wrap((*self.as_ptr()).codec, Some(self.context.destructor())) }
    }

    pub fn parameters(&self) -> codec::Parameters {
        unsafe {
            codec::Parameters::wrap((*self.as_ptr()).codecpar, Some(self.context.destructor()))
        }
    }

    pub fn index(&self) -> usize {
        unsafe { (*self.as_ptr()).index as usize }
    }

    pub fn time_base(&self) -> Rational {
        unsafe { Rational::from((*self.as_ptr()).time_base) }
    }

    pub fn start_time(&self) -> i64 {
        unsafe { (*self.as_ptr()).start_time }
    }

    pub fn duration(&self) -> i64 {
        unsafe { (*self.as_ptr()).duration }
    }

    pub fn frames(&self) -> i64 {
        unsafe { (*self.as_ptr()).nb_frames }
    }

    pub fn disposition(&self) -> Disposition {
        unsafe { Disposition::from_bits_truncate((*self.as_ptr()).disposition) }
    }

    pub fn discard(&self) -> Discard {
        unsafe { Discard::from((*self.as_ptr()).discard) }
    }

    pub fn side_data(&self) -> SideDataIter {
        SideDataIter::new(self)
    }

    pub fn rate(&self) -> Rational {
        unsafe { Rational::from(av_stream_get_r_frame_rate(self.as_ptr())) }
    }

    pub fn avg_frame_rate(&self) -> Rational {
        unsafe { Rational::from((*self.as_ptr()).avg_frame_rate) }
    }

    pub fn metadata(&self) -> DictionaryRef {
        unsafe { DictionaryRef::wrap((*self.as_ptr()).metadata) }
    }
}

impl<'a> PartialEq for Stream<'a> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.as_ptr() == other.as_ptr() }
    }
}

impl<'a> Eq for Stream<'a> {}

pub struct SideDataIter<'a> {
    stream: &'a Stream<'a>,
    current: c_int,
}

impl<'a> SideDataIter<'a> {
    pub fn new<'sd, 's: 'sd>(stream: &'s Stream) -> SideDataIter<'sd> {
        SideDataIter { stream, current: 0 }
    }
}

impl<'a> Iterator for SideDataIter<'a> {
    type Item = packet::SideData<'a>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        unsafe {
            if self.current >= (*self.stream.as_ptr()).nb_side_data {
                return None;
            }

            self.current += 1;

            Some(packet::SideData::wrap(
                (*self.stream.as_ptr())
                    .side_data
                    .offset((self.current - 1) as isize),
            ))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe {
            let length = (*self.stream.as_ptr()).nb_side_data as usize;

            (
                length - self.current as usize,
                Some(length - self.current as usize),
            )
        }
    }
}

impl<'a> ExactSizeIterator for SideDataIter<'a> {}

pub struct StreamMut<'a> {
    context: &'a mut Context,
    index: usize,

    immutable: Stream<'a>,
}

impl<'a> StreamMut<'a> {
    pub unsafe fn wrap(context: &mut Context, index: usize) -> StreamMut {
        StreamMut {
            context: mem::transmute_copy(&context),
            index,

            immutable: Stream::wrap(mem::transmute_copy(&context), index),
        }
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut AVStream {
        *(*self.context.as_mut_ptr()).streams.add(self.index)
    }
}

impl<'a> StreamMut<'a> {
    pub fn set_time_base<R: Into<Rational>>(&mut self, value: R) {
        unsafe {
            (*self.as_mut_ptr()).time_base = value.into().into();
        }
    }

    pub fn set_rate<R: Into<Rational>>(&mut self, value: R) {
        unsafe {
            av_stream_set_r_frame_rate(self.as_mut_ptr(), value.into().into());
        }
    }

    pub fn set_avg_frame_rate<R: Into<Rational>>(&mut self, value: R) {
        unsafe {
            (*self.as_mut_ptr()).avg_frame_rate = value.into().into();
        }
    }

    pub fn set_parameters<P: Into<codec::Parameters>>(&mut self, parameters: P) {
        let parameters = parameters.into();

        unsafe {
            avcodec_parameters_copy((*self.as_mut_ptr()).codecpar, parameters.as_ptr());
        }
    }

    pub fn set_metadata(&mut self, metadata: Dictionary) {
        unsafe {
            let metadata = metadata.disown();
            (*self.as_mut_ptr()).metadata = metadata;
        }
    }
}

impl<'a> Deref for StreamMut<'a> {
    type Target = Stream<'a>;

    fn deref(&self) -> &Self::Target {
        &self.immutable
    }
}
