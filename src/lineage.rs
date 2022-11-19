use crate::Unique;
use std::{
    fmt, ptr, ptr::addr_of_mut, sync::atomic::AtomicPtr, sync::atomic::Ordering::Acquire,
    sync::atomic::Ordering::Release, sync::Mutex,
};

#[cold]
fn panic_poisened_mutex<T, E>(_: E) -> T {
    panic!("poisened mutex");
}

struct Node<T> {
    value: T,
    #[allow(unused)]
    prev: Option<Unique<Node<T>>>,
}

/// A type of cell that allows replacing the contained value without invalidating references to
/// previous values.
#[derive()]
pub struct Lineage<T> {
    inline: T,
    ptr_heap: AtomicPtr<T>,
    head: Mutex<Option<Unique<Node<T>>>>,
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
    /// Create a new `Lineage` with the provided value.
    pub fn new(value: T) -> Self {
        Lineage {
            inline: value,
            ptr_heap: AtomicPtr::new(ptr::null_mut()),
            head: Mutex::new(None),
        }
    }

    /// Get a reference to the current value.
    pub fn get(&self) -> &T {
        unsafe { self.ptr_heap.load(Acquire).as_ref().unwrap_or(&self.inline) }
    }

    /// Get a mutable reference to the current value.
    ///
    /// Performs better than [`get`][Lineage::get] but requires `&mut self`.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe {
            let ptr = *self.ptr_heap.get_mut();

            debug_assert!({
                if let Ok(head) = self.head.get_mut() {
                    head.as_mut()
                        .map(|unique| addr_of_mut!((*unique.get_ptr()).value))
                        .unwrap_or(ptr::null_mut())
                        == ptr
                } else {
                    // poisened mutex
                    true
                }
            });

            ptr.as_mut().unwrap_or(&mut self.inline)
        }
    }

    /// Replace the value.
    ///
    /// Replacing the value does not invalidate existing references to the previous value. The previous
    /// value is kept alive until you call [`clear`][Lineage::clear] or drop the `Lineage`.
    pub fn set(&self, value: T) {
        unsafe {
            let mut lock = self.head.lock().unwrap_or_else(panic_poisened_mutex);
            let head: &mut Option<Unique<Node<T>>> = &mut lock;

            debug_assert!(head.is_none() == self.ptr_heap.load(Acquire).is_null());

            *head = Some(Unique::new(Node {
                value,
                prev: head.take(),
            }));

            self.ptr_heap.store(
                addr_of_mut!((*head.as_ref().unwrap_unchecked().get_ptr()).value),
                Release,
            );
        }
    }

    /// Replace the value.
    ///
    /// Performs better than [`set`][Lineage::set] but requires `&mut self`. Also drops old values
    /// similar to [`clear`][Lineage::clear].
    pub fn set_mut(&mut self, value: T) {
        let head = self.head.get_mut().unwrap_or_else(panic_poisened_mutex);

        debug_assert!(head.is_none() == self.ptr_heap.get_mut().is_null());

        *head = None;
        *self.ptr_heap.get_mut() = ptr::null_mut();

        self.inline = value;
    }

    /// Drop all past values. Does not affect the current value.
    ///
    /// This can only be called if no references to any of the previous values exist anymore. This is
    /// ensured by the `&mut self` requirement.
    pub fn clear(&mut self) {
        let head = self.head.get_mut().unwrap_or_else(panic_poisened_mutex);

        if let Some(head) = head.take() {
            debug_assert!(!self.ptr_heap.get_mut().is_null());

            self.inline = head.into_inner().value;
            *self.ptr_heap.get_mut() = ptr::null_mut();
        }

        debug_assert!(head.is_none());
        debug_assert!(self.ptr_heap.get_mut().is_null());
    }

    /// Return ownership of the current value.
    pub fn into_inner(mut self) -> T {
        let head = self.head.get_mut().unwrap_or_else(panic_poisened_mutex);

        head.take()
            .map(|unique| unique.into_inner().value)
            .unwrap_or(self.inline)
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
