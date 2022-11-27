//! [`Lineage`]`<T>` is a type of cell that allows replacing the contained value while previous values are
//! still borrowed. This is safe because old values are stored until explicitly cleared.
//!
//! The original implementation was essentially a [`Vec`]`<`[`Box`]`<T>>`. The `Box` ensures that the values
//! are not moved when the `Vec` needs to reallocate to increase its capacity. The current implementation uses
//! a linked list instead of a `Vec` to avoid the additional allocations caused by the `Vec`. Also, the first
//! value is stored inline within the `Lineage`. Only calling [`Lineage::set`] causes a heap allocation.

mod lineage;

pub use crate::lineage::Lineage;

#[cfg(test)]
mod test;
