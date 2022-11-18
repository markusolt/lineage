use crate::Unique;
use std::{
    cell::UnsafeCell, fmt, marker::PhantomData, mem::MaybeUninit, ptr, ptr::addr_of_mut,
    sync::atomic::AtomicPtr, sync::atomic::AtomicUsize, sync::atomic::Ordering::Acquire,
    sync::atomic::Ordering::Release, sync::Mutex,
};

struct Node<T> {
    value: T,
    prev: Option<Unique<Node<T>>>,
}

/// A type of cell that allows replacing the contained value without invalidating references to
/// previous values.
///
/// The optional const generic `N` specifies how many values can be stored inline. Defaults to `1`,
/// which means creating a lineage with [`Lineage::new`] does not perform any heap allocations.
/// Replacing the value with [`Lineage::set`] will cause a heap allocation because the new value
/// will be stored in a [`Box`]. The const generic `N` can be set to a higher value to allow for
/// multiple replacings of the value before needing heap allocations.
#[derive()]
pub struct Lineage<T, const N: usize = 1> {
    ptr_heap: AtomicPtr<T>,
    ptr_local: AtomicUsize,
    inline: UnsafeCell<[MaybeUninit<T>; N]>,
    mutex: Mutex<(usize, Option<Unique<Node<T>>>)>,
    _t: PhantomData<T>,
}

impl<T, const N: usize> Drop for Lineage<T, N> {
    fn drop(&mut self) {
        unsafe {
            if let Ok(mut lock) = self.mutex.get_mut() {
                let (inline_len, _) = &mut lock;

                let inline = self.inline.get_mut();

                if N > 0 {
                    debug_assert!(*inline_len > 0);

                    for i in 0..*inline_len {
                        inline[i].assume_init_drop();
                    }
                    *inline_len = 0;
                } else {
                    debug_assert!(*inline_len == N);
                }
            } else {
                // the mutex is poisened. there is no way to know how much of the inline storage
                // is used, we have no choice but to leak everything stored inline.
            }
        }
    }
}

unsafe impl<T, const N: usize> Send for Lineage<T, N> where T: Send {}

unsafe impl<T, const N: usize> Sync for Lineage<T, N> where T: Send + Sync {}

impl<T, const N: usize> fmt::Debug for Lineage<T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Lineage").field(self.get()).finish()
    }
}

impl<T, const N: usize> Lineage<T, N> {
    /// Create a new `Lineage` with the provided value.
    pub fn new(value: T) -> Self {
        unsafe {
            let mut ret = Lineage {
                ptr_heap: AtomicPtr::new(ptr::null_mut()),
                ptr_local: AtomicUsize::new(0),
                inline: UnsafeCell::new([(); N].map(|_| MaybeUninit::uninit())),
                mutex: Mutex::new((0, None)),
                _t: PhantomData,
            };

            {
                let (inline_len, head) = ret.mutex.get_mut().unwrap_unchecked();
                let inline = ret.inline.get_mut();

                if N > 0 {
                    inline[0].write(value);
                    *inline_len = 1;

                    *ret.ptr_local.get_mut() = 0;
                } else {
                    *head = Some(Unique::new(Node { value, prev: None }));

                    *ret.ptr_heap.get_mut() =
                        addr_of_mut!(head.as_mut().unwrap_unchecked().get_mut().value);
                }
            }

            ret
        }
    }

    /// Get a reference to the current value.
    pub fn get(&self) -> &T {
        unsafe {
            let ptr_heap = self.ptr_heap.load(Acquire);
            if ptr_heap.is_null() {
                let ptr_local = self.ptr_local.load(Acquire);

                debug_assert!(N > 0);
                debug_assert!(ptr_local < N);

                addr_of_mut!((*self.inline.get())[ptr_local])
                    .as_ref()
                    .unwrap_unchecked()
                    .assume_init_ref()
            } else {
                ptr_heap.as_ref().unwrap_unchecked()
            }
        }
    }

    /// Get a mutable reference to the current value.
    ///
    /// Performs better than [`get`][Lineage::get] but requires `&mut self`.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe {
            let ptr_heap = *self.ptr_heap.get_mut();
            if ptr_heap.is_null() {
                let ptr_local = *self.ptr_local.get_mut();

                debug_assert!(N > 0);
                debug_assert!(ptr_local < N);
                debug_assert!(self.mutex.get_mut().unwrap().0 == ptr_local + 1);

                addr_of_mut!((*self.inline.get())[ptr_local])
                    .as_mut()
                    .unwrap_unchecked()
                    .assume_init_mut()
            } else {
                ptr_heap.as_mut().unwrap_unchecked()
            }
        }
    }

    /// Replace the value.
    ///
    /// Replacing the value does not invalidate existing references to the previous value. The previous
    /// value is kept alive until you call [`clear`][Lineage::clear] or drop the `Lineage`.
    pub fn set(&self, value: T) {
        unsafe {
            let (inline_len, head): &mut (_, _) = &mut self
                .mutex
                .lock()
                .unwrap_or_else(|_| panic!("poisened mutex"));

            if N > 0 && *inline_len < N {
                debug_assert!(*inline_len > 0);

                let ptr: *mut MaybeUninit<T> = addr_of_mut!((*self.inline.get())[*inline_len]);
                ptr.write(MaybeUninit::new(value));
                *inline_len += 1;

                self.ptr_local.store(*inline_len - 1, Release);
            } else {
                debug_assert!(*inline_len == N);
                if N == 0 {
                    debug_assert!(head.is_some());
                }

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
    }

    /// Replace the value.
    ///
    /// Performs better than [`set`][Lineage::set] but requires `&mut self`. Also drops old values
    /// similar to [`clear`][Lineage::clear].
    pub fn set_mut(&mut self, value: T) {
        unsafe {
            let (inline_len, head) = self.mutex.get_mut().unwrap();
            let inline = self.inline.get_mut();

            if N > 0 {
                debug_assert!(*inline_len > 0);
                debug_assert!(*inline_len <= N);

                *head = None;

                for i in 0..*inline_len {
                    inline[i].assume_init_drop();
                }
                inline[0].write(value);
                *inline_len = 1;

                *self.ptr_local.get_mut() = 0;
                *self.ptr_heap.get_mut() = ptr::null_mut();
            } else {
                debug_assert!(!self.ptr_heap.get_mut().is_null());
                debug_assert!(head.is_some());

                let node: &mut Node<T> = head.as_mut().unwrap_unchecked().get_mut();

                node.value = value;
                node.prev = None;

                *self.ptr_heap.get_mut() = addr_of_mut!(node.value);
            }
        }
    }

    /// Drop all past values. Does not affect the current value.
    ///
    /// This can only be called if no references to any of the previous values exist anymore. This is
    /// ensured by the `&mut self` requirement.
    pub fn clear(&mut self) {
        unsafe {
            let (inline_len, head) = self.mutex.get_mut().unwrap();
            let inline = self.inline.get_mut();

            if (N == 0 && head.as_mut().unwrap_unchecked().get_mut().prev.is_none())
                || (N == 1 && head.is_none())
                || (N > 1 && *inline_len == 1)
            {
                return;
            }

            if N > 0 {
                debug_assert!(*inline_len > 0);
                debug_assert!(*inline_len <= N);

                let value = if let Some(unique) = head.take() {
                    unique.into_inner().value
                } else {
                    *inline_len -= 1;
                    inline[*inline_len].assume_init_read()
                };

                for i in 0..*inline_len {
                    inline[i].assume_init_drop();
                }
                inline[0].write(value);
                *inline_len = 1;

                *self.ptr_local.get_mut() = 0;
                *self.ptr_heap.get_mut() = ptr::null_mut();
            } else {
                debug_assert!(!self.ptr_heap.get_mut().is_null());
                debug_assert!(head.is_some());

                head.as_mut().unwrap_unchecked().get_mut().prev = None;
            }
        }
    }

    /// Return ownership of the current value.
    pub fn into_inner(mut self) -> T {
        unsafe {
            let (inline_len, head) = self.mutex.get_mut().unwrap();
            let inline = self.inline.get_mut();

            if let Some(unique) = head.take() {
                debug_assert!(*inline_len == N);

                unique.into_inner().value
            } else {
                debug_assert!(*inline_len > 0);
                debug_assert!(*inline_len <= N);

                *inline_len -= 1;
                inline[*inline_len].assume_init_read()
            }
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
