use std::{
    fmt, mem, ptr, ptr::NonNull, sync::atomic::AtomicPtr, sync::atomic::Ordering::Acquire,
    sync::atomic::Ordering::Relaxed, sync::atomic::Ordering::SeqCst,
};

struct AtomicLinkedList<T> {
    head: AtomicPtr<Node<T>>,
}

impl<T> Drop for AtomicLinkedList<T> {
    fn drop(&mut self) {
        unsafe {
            let mut cur = NonNull::new(*self.head.get_mut());
            while let Some(ptr) = cur {
                let Node { value, next } = *Box::from_raw(ptr.as_ptr());

                mem::drop(value);
                cur = next;
            }
        }
    }
}

struct Node<T> {
    value: T,
    next: Option<NonNull<Node<T>>>,
}

/// A type of cell that allows replacing the contained value without invalidating references to
/// previous values.
#[derive()]
pub struct Lineage<T> {
    inline: T,
    list: AtomicLinkedList<T>,
}

unsafe impl<T> Send for Lineage<T> where T: Send {}

unsafe impl<T> Sync for Lineage<T> where T: Send + Sync {}

impl<T> fmt::Debug for Lineage<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Lineage").field(self.get()).finish()
    }
}

impl<T> Lineage<T> {
    /// Creates a new `Lineage` with the provided value.
    pub fn new(value: T) -> Self {
        Lineage {
            inline: value,
            list: AtomicLinkedList {
                head: AtomicPtr::new(ptr::null_mut()),
            },
        }
    }

    /// Gets a reference to the current value.
    pub fn get(&self) -> &T {
        unsafe {
            self.list
                .head
                .load(Acquire)
                .as_ref()
                .map(|node| &node.value)
                .unwrap_or(&self.inline)
        }
    }

    /// Gets a mutable reference to the current value.
    ///
    /// Performs better than [`get`][Lineage::get] but requires `&mut self`.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe {
            self.list
                .head
                .get_mut()
                .as_mut()
                .map(|node| &mut node.value)
                .unwrap_or(&mut self.inline)
        }
    }

    /// Replaces the value.
    ///
    /// Replacing the value does not invalidate existing references to the previous value. The previous
    /// value is kept alive until you call [`clear`][Lineage::clear] or drop the `Lineage`. The new value
    /// is stored in a [`Box`] which causes a heap allocation.
    pub fn set(&self, value: T) {
        unsafe {
            let mut next = self.list.head.load(Acquire);
            let mut node = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
                value,
                next: NonNull::new(next),
            })));

            while let Err(err) =
                self.list
                    .head
                    .compare_exchange_weak(next, node.as_ptr(), SeqCst, Relaxed)
            {
                if next != err {
                    debug_assert!(!err.is_null());

                    next = err;
                    node.as_mut().next = Some(NonNull::new_unchecked(next));
                }
            }
        }
    }

    /// Replaces the value.
    ///
    /// Performs much better than [`set`][Lineage::set] but requires `&mut self`. The implementation
    /// is a more optimized version of the following code:
    ///
    /// ```
    /// # use lineage::Lineage;
    /// fn set_mut<T>(lineage: &mut Lineage<T>, value: T) {
    ///     lineage.clear();
    ///     *lineage.get_mut() = value;
    /// }
    /// ```
    pub fn set_mut(&mut self, value: T) {
        let ptr = *self.list.head.get_mut();
        if !ptr.is_null() {
            *self.list.head.get_mut() = ptr::null_mut();

            mem::drop(AtomicLinkedList {
                head: AtomicPtr::new(ptr),
            });
        }

        self.inline = value;
    }

    /// Drops all past values. Does not affect the current value.
    ///
    /// This can only be called if no references to any of the previous values exist anymore. This is
    /// ensured by the `&mut self` requirement. Should be called as often as possible to avoid having
    /// many past values kept alive unnecessarily.
    pub fn clear(&mut self) {
        if let Some(value) = self.pop_and_clear() {
            self.inline = value;
        }
    }

    /// Returns ownership of the current value.
    pub fn into_inner(mut self) -> T {
        if let Some(value) = self.pop_and_clear() {
            value
        } else {
            self.inline
        }
    }

    fn pop_and_clear(&mut self) -> Option<T> {
        unsafe {
            let ptr = *self.list.head.get_mut();
            if ptr.is_null() {
                return None;
            } else {
                let Node { value, next } = *Box::from_raw(ptr);
                *self.list.head.get_mut() = ptr::null_mut();

                if let Some(ptr) = next {
                    mem::drop(AtomicLinkedList {
                        head: AtomicPtr::new(ptr.as_ptr()),
                    });
                }

                Some(value)
            }
        }
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
