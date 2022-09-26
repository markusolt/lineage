//! This crate provides the struct [`Lineage<T>`], which is a cell that allows replacing the contained
//! value while the previous value is still borrowed. This is safe because the old values are stored and
//! only dropped at a later time.

mod arrayvec;
use arrayvec::ArrayVec;

use std::{
    fmt, marker::PhantomData, ptr, sync::atomic::AtomicPtr, sync::atomic::Ordering, sync::Mutex,
};

use static_assertions::assert_not_impl_any;

/// A type of cell that allows replacing the contained value without invalidating references to
/// the previous value.
///
/// The optional const generic `N` specifies how many values can be stored inline. Defaults to `1`,
/// which means creating a lineage with `Lineage::new(value)` does not perform any heap allocations.
/// Replacing the value will however cause a heap allocation because new values will be stored in a
/// `Vec<Box<T>>`. The const generic `N` can be set to a higher value to allow for multiple replacings
/// of the value before needing a `Vec<Box<T>>`.
#[derive()]
pub struct Lineage<T, const N: usize = 1> {
    current: AtomicPtr<T>,
    past: Mutex<(ArrayVec<T, N>, Vec<Box<T>>)>,
    _t: PhantomData<T>,
}

assert_not_impl_any!(Lineage<std::cell::Cell<usize>>: Sync);
assert_not_impl_any!(Lineage<std::rc::Rc<usize>>: Send, Sync);

impl<T, const N: usize> fmt::Debug for Lineage<T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Lineage").field(self.get()).finish()
    }
}

impl<T, const N: usize> Lineage<T, N> {
    /// Create a new lineage with the specified value.
    pub fn new(value: T) -> Self {
        let mut ret = Lineage {
            current: AtomicPtr::new(ptr::null_mut()),
            past: Mutex::new((ArrayVec::new(), Vec::new())),
            _t: PhantomData,
        };
        ret.set_mut(value);

        ret
    }

    /// Get a reference to the current value.
    pub fn get(&self) -> &T {
        unsafe {
            //

            self.current
                .load(Ordering::Acquire)
                .as_ref()
                .unwrap_unchecked()
        }
    }

    /// Get a mutable reference to the current value.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe {
            //

            self.current.get_mut().as_mut().unwrap_unchecked()
        }
    }

    /// Replace the contained value.
    ///
    /// Replacing the value does not invalidate existing references to the previous value. The previous
    /// value is kept alive until you call [`clear`][Lineage::clear].
    pub fn set(&self, value: T) {
        let past: &mut (_, _) = &mut self.past.lock().unwrap();

        let ptr = match past.0.try_push(value) {
            None => past.0.last_mut().unwrap() as *mut T,
            Some(value) => {
                let mut value = Box::new(value);
                let ptr = value.as_mut() as *mut T;
                past.1.push(value);

                ptr
            }
        };
        self.current.store(ptr, Ordering::Release);
    }

    /// Replace the contained value.
    ///
    /// Performs better than [`set`][Lineage::set] and drops old values similar to
    /// [`clear`][Lineage::clear] but can only be called on `&mut self`.
    pub fn set_mut(&mut self, value: T) {
        let past = self.past.get_mut().unwrap();

        past.0.clear();
        past.1.clear();
        let ptr = match past.0.try_push(value) {
            None => past.0.last_mut().unwrap() as *mut T,
            Some(value) => {
                let mut value = Box::new(value);
                let ptr = value.as_mut() as *mut T;
                past.1.push(value);

                ptr
            }
        };
        *self.current.get_mut() = ptr;
    }

    /// Drop all past values. Does not affect the current value.
    ///
    /// This can only be called if no references to any of the previous values exist anymore. This is
    /// ensured by the `&mut self` requirement.
    pub fn clear(&mut self) {
        let past = self.past.get_mut().unwrap();

        if (N == 0 && past.1.len() == 1)
            || (N == 1 && past.1.len() == 0)
            || (N > 1 && past.0.len() == 1)
        {
            return;
        }

        if N == 0 {
            let current: Box<T> = past.1.pop().unwrap();

            past.1.clear();
            past.1.push(current);
        } else {
            let current: T = if let Some(current) = past.1.pop() {
                *current
            } else {
                past.0.pop().unwrap()
            };

            self.set_mut(current);
        }
    }

    /// Return ownership of the current value.
    pub fn into_inner(mut self) -> T {
        let past = self.past.get_mut().unwrap();

        if let Some(current) = past.1.pop() {
            *current
        } else {
            past.0.pop().unwrap()
        }
    }
}

impl<T, const N: usize> Clone for Lineage<T, N>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Lineage::new(self.get().clone())
    }
}

impl<T, const N: usize> Default for Lineage<T, N>
where
    T: Default,
{
    fn default() -> Self {
        Lineage::new(T::default())
    }
}

impl<T, const N: usize> From<T> for Lineage<T, N> {
    fn from(value: T) -> Self {
        Lineage::new(value)
    }
}
