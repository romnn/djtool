use crate::ffi::*;
use crate::format::context::common::Context;
use crate::{Dictionary, DictionaryMut, DictionaryRef, Rational};

use std::mem;
use std::ops::Deref;

// WARNING: index refers to the offset in the chapters array (starting from 0)
// it is not necessarly equal to the id (which may start at 1)
pub struct Chapter<'a> {
    context: &'a Context,
    index: usize,
}

impl<'a> Chapter<'a> {
    pub unsafe fn wrap(context: &Context, index: usize) -> Chapter {
        Chapter { context, index }
    }

    pub unsafe fn as_ptr(&self) -> *const AVChapter {
        *(*self.context.as_ptr()).chapters.add(self.index)
    }
}

impl<'a> Chapter<'a> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn id(&self) -> i32 {
        unsafe { (*self.as_ptr()).id }
    }

    pub fn time_base(&self) -> Rational {
        unsafe { Rational::from((*self.as_ptr()).time_base) }
    }

    pub fn start(&self) -> i64 {
        unsafe { (*self.as_ptr()).start }
    }

    pub fn end(&self) -> i64 {
        unsafe { (*self.as_ptr()).end }
    }

    pub fn metadata(&self) -> DictionaryRef {
        unsafe { DictionaryRef::wrap((*self.as_ptr()).metadata) }
    }
}

impl<'a> PartialEq for Chapter<'a> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.as_ptr() == other.as_ptr() }
    }
}

// WARNING: index refers to the offset in the chapters array (starting from 0)
// it is not necessarly equal to the id (which may start at 1)
pub struct ChapterMut<'a> {
    context: &'a mut Context,
    index: usize,

    immutable: Chapter<'a>,
}

impl<'a> ChapterMut<'a> {
    pub unsafe fn wrap(context: &mut Context, index: usize) -> ChapterMut {
        ChapterMut {
            context: mem::transmute_copy(&context),
            index,

            immutable: Chapter::wrap(mem::transmute_copy(&context), index),
        }
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut AVChapter {
        *(*self.context.as_mut_ptr()).chapters.add(self.index)
    }
}

impl<'a> ChapterMut<'a> {
    pub fn set_id(&mut self, value: i32) {
        unsafe {
            (*self.as_mut_ptr()).id = value;
        }
    }

    pub fn set_time_base<R: Into<Rational>>(&mut self, value: R) {
        unsafe {
            (*self.as_mut_ptr()).time_base = value.into().into();
        }
    }

    pub fn set_start(&mut self, value: i64) {
        unsafe {
            (*self.as_mut_ptr()).start = value;
        }
    }

    pub fn set_end(&mut self, value: i64) {
        unsafe {
            (*self.as_mut_ptr()).end = value;
        }
    }

    pub fn set_metadata<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, value: V) {
        // dictionary.set() allocates the AVDictionary the first time a key/value is inserted
        // so we want to update the metadata dictionary afterwards
        unsafe {
            let mut dictionary = Dictionary::own(self.metadata().as_mut_ptr());
            dictionary.set(key.as_ref(), value.as_ref());
            (*self.as_mut_ptr()).metadata = dictionary.disown();
        }
    }

    pub fn metadata(&mut self) -> DictionaryMut {
        unsafe { DictionaryMut::wrap((*self.as_mut_ptr()).metadata) }
    }
}

impl<'a> Deref for ChapterMut<'a> {
    type Target = Chapter<'a>;

    fn deref(&self) -> &Self::Target {
        &self.immutable
    }
}
