use std::{
    fmt, mem, ptr, ptr::NonNull, sync::atomic::AtomicPtr, sync::atomic::Ordering::Acquire,
    sync::atomic::Ordering::Relaxed, sync::atomic::Ordering::SeqCst,
};

struct AtomicLinkedList<T> {
    head: AtomicPtr<Node<T>>,
}

unsafe impl<T> Send for AtomicLinkedList<T> where T: Send {}

// we must require "T: Send" because of the existence of "Lineage::set" and "Lineage::into_inner".
// imagine T is Sync but not Send and we own a value of type T on a thread B. further imagine we
// own a lineage on a thread A. we can now call "Lineage::set" on thread B to move the value into
// the lineage followed by calling "Lineage::into_inner" on thread A to take ownership of the value.
// we just sent the value from thread B to thread A even though T is not Send. to prevent this
// lineage must not be Sync.
unsafe impl<T> Sync for AtomicLinkedList<T> where T: Send + Sync {}

impl<T> Drop for AtomicLinkedList<T> {
    fn drop(&mut self) {
        mem::drop(LinkedList {
            head: NonNull::new(*self.head.get_mut()),
        })
    }
}

struct LinkedList<T> {
    head: Option<NonNull<Node<T>>>,
}

unsafe impl<T> Send for LinkedList<T> where T: Send {}

unsafe impl<T> Sync for LinkedList<T> where T: Sync {}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        unsafe {
            let mut cur = self.head;
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
/// the current value.
#[derive()]
pub struct Lineage<T> {
    inline: T,
    list: AtomicLinkedList<T>,
}

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
    /// Performs much better than [`set`][Lineage::set] but requires `&mut self`. Does not cause a heap
    /// allocation. The implementation is a more optimized version of the following:
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

            mem::drop(LinkedList {
                head: NonNull::new(ptr),
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
        if self.list.head.get_mut().is_null() {
            return;
        }

        mem::drop(self.drain());
    }

    /// Same as [`clear`][Lineage::clear] but returns ownership of past values.
    ///
    /// The values are iterated over from newest to oldest. The iterator can safely be dropped, all
    /// remaining values in the iterator will be dropped.
    pub fn drain(&mut self) -> impl Iterator<Item = T> {
        struct Drain<T> {
            list: LinkedList<T>,
            last: Option<T>,
        }

        impl<T> Iterator for Drain<T> {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                unsafe {
                    let ptr = self.list.head;
                    if let Some(ptr) = ptr {
                        let Node { value, next } = *Box::from_raw(ptr.as_ptr());
                        self.list.head = next;

                        Some(value)
                    } else {
                        self.last.take()
                    }
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                if self.list.head.is_some() {
                    debug_assert!(self.last.is_some());

                    (2, None)
                } else if self.last.is_some() {
                    (1, Some(1))
                } else {
                    (0, Some(0))
                }
            }

            fn last(self) -> Option<Self::Item>
            where
                Self: Sized,
            {
                debug_assert!({
                    if self.last.is_none() {
                        self.list.head.is_none()
                    } else {
                        true
                    }
                });

                self.last
            }
        }

        let mut ret = Drain {
            list: LinkedList {
                head: NonNull::new(mem::replace(self.list.head.get_mut(), ptr::null_mut())),
            },
            last: None,
        };
        if let Some(newest) = ret.next() {
            ret.last = Some(mem::replace(&mut self.inline, newest));
        } else {
            // the linked list is empty. this means "self.inline" is the only and therefor already
            // the newest value.
        }

        ret
    }

    /// Returns ownership of the current value.
    pub fn into_inner(mut self) -> T {
        if !self.list.head.get_mut().is_null() {
            self.clear();
        }
        self.inline
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
