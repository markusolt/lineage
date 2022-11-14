# Lineage

A rust crate that provides a new type of cell that can replace its contained value while the previous value is still immutably borrowed. Useful to safely create bump allocators.

```rust
let lineage: Lineage<[u32; 3]> = Lineage::new([1, 2, 3]);
let s1: &[u32] = lineage.get();

lineage.replace([4, 5, 6]);
let s2: &[u32] = lineage.get();

assert_eq!(s1, &[1, 2, 3]);
assert_eq!(s2, &[4, 5, 6]);
```

## Safety

There is a collection of tests that can be run normally, or using [miri](https://github.com/rust-lang/miri). These tests should discover most kinds of undefined behavior.

The current implementation makes liberal use of `unsafe`. Originally the implementation was far simpler, but `miri` detected some aliased references. The solution seems to be to rely more heavily on raw pointers, which of course requires more `unsafe`. The inline storage for example used to be a simple abstraction using an `ArrayVec<T, N>`, but this caused aliased references. Now we have a `Mutex<usize>` and a separate `[MaybeUninit<T>; N]` field so that we can write into the inline storage without creating a reference which may alias with references to past values. A much uglier implementation, but apparently necessary to avoid undefined behavior. Currently all tests pass `miri` without any errors or warnings.

If you have `miri` and a nightly toolchain installed, you can run the tests with the following command:

```
cargo +nightly miri test
```
