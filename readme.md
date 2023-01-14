# Lineage

A small rust crate that provides a type of cell that can replace its contained value while the previous value is still immutably borrowed:

```rust
pub fn new(value: T) -> Self { }

pub fn get(&self) -> &T { }

pub fn set(&self, value: T) { }

pub fn clear(&mut self) { }
```

Notice how a new value can be inserted into the cell with `set` using only `&self`.
This means the `Lineage` may still be borrowed by previous calls to `get` but you can replace the contained value anyways.
Internally the replaced value is added to a linked list which is not cleared until you call `clear` or drop the `Lineage`.

```rust
let lineage: Lineage<String> = Lineage::new(String::from("ONE"));
let s1 = lineage.get();

lineage.set(String::from("TWO"));
let s2 = lineage.get();

assert_eq!(s1, "ONE");
assert_eq!(s2, "TWO");
```

## Safety

As is expected for this kind of utility crate, the implementation makes use of `unsafe`.
We have a number of tests to look for undefined behavior which can all be run natively or with [miri](https://github.com/rust-lang/miri).
`miri` is a fantastic tool to execute rust applications in a virtual runtime that is sensitive to various kinds of undefined behavior.

```
# run tests normally
cargo test

# install miri
rustup toolchain install nightly
rustup +nightly component add miri

# run tests in miri
cargo clean
cargo +nightly miri test
```
