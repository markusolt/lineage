//! This crate provides the struct [`Lineage<T>`], which is a cell that allows replacing the contained
//! value while the previous value is still immutably borrowed. This is safe because old values are
//! stored and not dropped until [`cleared`][Lineage::clear]. Useful to safely implemet bump allocators.
//!
//! ```
//! # use lineage::Lineage;
//! let lineage: Lineage<[u32; 3]> = Lineage::new([1, 2, 3]);
//! let s1: &[u32] = lineage.get();
//!
//! lineage.replace([4, 5, 6]);
//! let s2: &[u32] = lineage.get();
//!
//! assert_eq!(s1, &[1, 2, 3]);
//! assert_eq!(s2, &[4, 5, 6]);
//! ```

use std::cell::RefCell;
use std::fmt;
use std::ptr::NonNull;

/// A type of cell that allows replacing the contained value while borrowed.
#[derive()]
pub struct Lineage<T> {
    current: RefCell<Box<T>>,
    past: RefCell<Vec<Box<T>>>,
}

impl<T> fmt::Debug for Lineage<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current: &T = &self.current.borrow();

        f.debug_struct("Lineage").field("current", current).finish()
    }
}

impl<T> Lineage<T> {
    /// Create a new lineage with the specified starting value.
    pub fn new(value: T) -> Self {
        Lineage {
            current: RefCell::new(Box::new(value)),
            past: RefCell::new(Vec::new()),
        }
    }

    /// Borrow the current value.
    pub fn get(&self) -> &T {
        unsafe {
            // we borrow the current value for the lifetime of "&self" without locking the refcell. this
            // is clearly dangerous, because the refcell allows us to mutate and drop its content, which
            // we must now avoid doing.
            //
            // the only function that mutably accesses the refcell is "self.replace()". this function
            // swaps the value with a replacement, but it stores the previous value in "self.past".
            // references to the old value are therefor still valid.
            //
            // we are allowed to mutate the refcell in "self.clear" and "self.get_mut" because these
            // functions take "&mut self". this proves that self is no longer immutably borrowed.

            NonNull::new(&mut self.current.borrow()).unwrap().as_ref()
        }
    }

    /// Borrow the current value mutably.
    pub fn get_mut(&mut self) -> &mut T {
        self.current.get_mut()
    }

    /// Replace the contained value.
    ///
    /// Replacing the value does not invalidate immutable borrows to the previous value. The replaced
    /// value is kept alive until you call [`clear`][Lineage::clear].
    pub fn replace(&self, value: T) {
        self.past
            .borrow_mut()
            .push(self.current.replace(Box::new(value)));
    }

    /// Clear out old replaced values. Does not affect the current value.
    ///
    /// This can only be called if no borrows to previous values exist anymore. This is ensured by the
    /// `&mut self` requirement.
    pub fn clear(&mut self) -> impl '_ + Iterator<Item = T> + ExactSizeIterator {
        self.past.get_mut().drain(..).rev().map(|entry| *entry)
    }
}

impl<T> Clone for Lineage<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Lineage {
            current: self.current.clone(),
            past: RefCell::new(Vec::new()),
        }
    }
}

impl<T> Default for Lineage<T>
where
    T: Default,
{
    fn default() -> Self {
        Lineage {
            current: RefCell::new(Default::default()),
            past: RefCell::new(Vec::new()),
        }
    }
}
