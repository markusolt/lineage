//! This crate provides the struct [`Lineage<T>`], which is a cell that allows replacing the contained
//! value while the previous value is still borrowed. This is safe because the old values are stored and
//! only dropped at a later time. Useful to safely implement bump allocators.
//!
//! ```
//! # use lineage::Lineage;
//! let lineage: Lineage<[u32; 3]> = Lineage::new([1, 2, 3]);
//! let s1: &[u32] = lineage.get();
//!
//! lineage.set([4, 5, 6]);
//! let s2: &[u32] = lineage.get();
//!
//! assert_eq!(s1, &[1, 2, 3]);
//! assert_eq!(s2, &[4, 5, 6]);
//! ```

use std::{fmt, marker::PhantomData, sync::atomic::AtomicPtr, sync::atomic::Ordering};

use smallvec::SmallVec;
use usync::Mutex;

/// A type of cell that allows replacing the contained value without invalidating existing references.
#[derive()]
pub struct Lineage<T> {
    current: AtomicPtr<T>,
    past: Mutex<SmallVec<[Box<T>; 1]>>,

    // this field ensures that Lineage<T> is only Sync if T is Sync
    _t: PhantomData<T>,
}

impl<T> fmt::Debug for Lineage<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lineage")
            .field("current", self.get())
            .finish()
    }
}

impl<T> Lineage<T> {
    /// Create a new lineage with the specified starting value.
    pub fn new(value: T) -> Self {
        let mut value = Box::new(value);
        let ptr = value.as_mut() as *mut T;

        Lineage {
            current: AtomicPtr::new(ptr),
            past: Mutex::new(SmallVec::from_iter([value])),
            _t: PhantomData,
        }
    }

    /// Get a reference to the current value.
    pub fn get(&self) -> &T {
        let ptr = self.current.load(Ordering::Relaxed);

        unsafe {
            // converting the pointer to a &T is safe. the pointer is properly aligned and initialized
            // because it was created from a Box<T>. the pointer is still valid because the box is stored
            // in self.past and will not be dropped while self.current is still pointing to its contents.
            //
            // the chosen lifetime is the lifetime of &self. this means we must ensure the reference
            // remains valid while &self is borrowed. we achieve this by not dropping the Box<T> which
            // owns the referenced value until wither self is dropped, or self.clear is called. in both
            // cases &self is no longer borrowed.

            ptr.as_ref()
        }
        .unwrap()
    }

    /// Get a mutable reference to the current value.
    pub fn get_mut(&mut self) -> &mut T {
        let ptr = self.current.load(Ordering::Relaxed);

        unsafe {
            // converting the pointer to a &mut T is safe. the pointer is properly aligned and initialized
            // because it was created from a Box<T>.

            ptr.as_mut()
        }
        .unwrap()
    }

    /// Replace the contained value.
    ///
    /// Replacing the value does not invalidate existing references to the previous value. The previous
    /// value is kept alive until you call [`clear`][Lineage::clear].
    pub fn set(&self, value: T) {
        let mut value = Box::new(value);
        let ptr = value.as_mut() as *mut T;

        let mut past = self.past.lock();
        past.push(value);

        self.current.store(ptr, Ordering::Release);
    }

    /// Clear all replaced values. Does not affect the current value.
    ///
    /// This can only be called if no references to any of the previous values exist anymore. This is
    /// ensured by the `&mut self` requirement.
    pub fn clear(&mut self) -> impl '_ + Iterator<Item = T> + ExactSizeIterator {
        let past = self.past.get_mut();

        {
            // let's verify that self.current is pointing to the last entry in self.past. all other
            // entries will be dropped and must not be pointed to.

            assert_eq!(
                self.current.load(Ordering::Acquire),
                past.last_mut().unwrap().as_mut() as *mut T
            );
        }

        let len = past.len();
        past.drain(0..len - 1).rev().map(|entry| *entry)
    }
}

impl<T> Clone for Lineage<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Lineage::new(self.get().clone())
    }
}

impl<T> Default for Lineage<T>
where
    T: Default,
{
    fn default() -> Self {
        Lineage::new(T::default())
    }
}

impl<T> From<T> for Lineage<T> {
    fn from(value: T) -> Self {
        Lineage::new(value)
    }
}
