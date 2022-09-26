//!

use std::{fmt, mem::MaybeUninit};

use static_assertions::assert_not_impl_any;

#[derive()]
pub struct ArrayVec<T, const N: usize> {
    // safety: at all times only the entries in "self.array[0..self.len]" are initialized.
    array: [MaybeUninit<T>; N],
    len: usize,
}

assert_not_impl_any!(ArrayVec<std::cell::Cell<usize>, 1>: Sync);
assert_not_impl_any!(ArrayVec<std::rc::Rc<usize>, 1>: Send, Sync);

impl<T, const N: usize> Drop for ArrayVec<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> fmt::Debug for ArrayVec<T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T, const N: usize> ArrayVec<T, N> {
    pub fn new() -> Self {
        ArrayVec {
            array: [(); N].map(|_| MaybeUninit::uninit()),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn try_push(&mut self, value: T) -> Option<T> {
        if self.len < N {
            self.array[self.len].write(value);
            self.len += 1;

            None
        } else {
            Some(value)
        }
    }

    pub fn last_mut(&mut self) -> Option<&mut T> {
        if self.len > 0 {
            unsafe { Some(self.array[self.len - 1].assume_init_mut()) }
        } else {
            None
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            Some(unsafe { self.array[self.len].assume_init_read() })
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        for value in self.array[0..self.len].iter_mut() {
            unsafe { value.assume_init_drop() };
        }
        self.len = 0;
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.array[0..self.len]
            .iter()
            .map(|value| unsafe { value.assume_init_ref() })
    }
}
