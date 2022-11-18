use std::{fmt, marker::PhantomData, mem, ptr::NonNull};

pub struct Unique<T> {
    ptr: NonNull<T>,
    _t: PhantomData<T>,
}

impl<T> Drop for Unique<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.ptr.as_ptr());
        }
    }
}

impl<T> fmt::Debug for Unique<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.get_ref(), f)
    }
}

unsafe impl<T> Send for Unique<T> where T: Send {}

unsafe impl<T> Sync for Unique<T> where T: Sync {}

impl<T> Unique<T> {
    pub fn new(value: T) -> Self {
        unsafe {
            Unique {
                ptr: NonNull::new_unchecked(Box::into_raw(Box::new(value))),
                _t: PhantomData,
            }
        }
    }

    pub fn get_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    pub fn get_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub fn into_inner(self) -> T {
        unsafe {
            let ret = *Box::from_raw(self.ptr.as_ptr());
            mem::forget(self);
            ret
        }
    }
}
