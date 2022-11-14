//! [`Lineage`]`<T, N>` is a type of cell that allows replacing the contained value while previous values are
//! still borrowed. This is safe because old values stored until explicitly cleared.
//!
//! The original implementation was essentially a [`Vec`]`<`[`Box`]`<T>>`. The `Box` ensures that the values
//! are not moved when the `Vec` needs to reallocate to increase its capacity. The current implementation uses
//! a linked list instead of a `Vec` to avoid the additional allocations caused by the `Vec`. Also, the current
//! implementation of `Lineage` contains an array to store the first `N` values within the `Lineage`. The const
//! generic `N` defaults to `1`. If `N` is at least `1` there is no need for any heap allocations when calling
//! `Lineage::new`. Heap allocations are only required for [`Lineage::set`], once the inline storage is full.

mod lineage;
mod unique;

pub use crate::lineage::Lineage;
pub(crate) use unique::Unique;

#[cfg(test)]
mod test;
