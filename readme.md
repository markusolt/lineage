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
