# LeanString

[![Crates.io](https://img.shields.io/crates/v/lean_string.svg)](https://crates.io/crates/lean_string)
[![Documentation](https://docs.rs/lean_string/badge.svg)](https://docs.rs/lean_string)

Compact, clone-on-write string.

## Properties

`LeanString` has the following properties:

- `size_of::<LeanString>() == size_of::<[usize; 2]>()` (2 words).
  - one `usize` smaller than `String`.
- Stores up to 16 bytes inline (on the stack).
  - 8 bytes if 32-bit architecture.
  - Strings larger than 16 bytes are stored on the heap.
- Clone-on-Write (CoW)
  - `LeanString` uses a reference-counted heap buffer (like `Arc`).
  - When a `LeanString` is cloned, the heap buffer is shared.
  - When a `LeanString` is mutated, the heap buffer is copied if it is shared.
- `O(1)`, zero allocation construction from `&'static str`.
- Nich optimized for `Option<LeanString>`.
  - `size_of::<Option<LeanString>>() == size_of::<LeanString>()`
- High API compatibility for `String`.

## TODOs

- [ ] Support 32-bit architecture.
- [ ] More API compatibility with `String`.
- [ ] Fuzz testing.
- [ ] Benchmarking.
- [ ] Documentation.
- etc...

## Example

```rust
use lean_string::LeanString;

// This is a zero-allocation operation, stored inlined.
let small = LeanString::from("Hello");

// More than 16 bytes, stored on the heap (64-bit architecture).
let large = LeanString::from("This is a not long but can't store inlined!");

// Clone is O(1), heap buffer is shared.
let mut shared = large.clone();

// Mutating a shared string will copy the heap buffer. (CoW)
assert_eq!(shared.pop(), Some('!'));
assert_eq!(shared, "This is a not long but can't store inlined");
assert_eq!(large, shared + "!");
```

## Which should I use?

TODO: Compare `LeanString` with `String`, `EcoString`, `CompactString`, etc...

| Name                | Size     | Inline   | `&'static str` | CoW |
| ------------------- | -------- | -------- | -------------- | ----|
| `String`            | 24 bytes | No       | No             | No  |
| `Cow<'static, str>` | 24 bytes | No       | Yes            | Yes |
| `CompactString`     | 24 bytes | 24 bytes | Yes            | No  |
| `EcoString`         | 16 bytes | 15 bytes | No             | Yes |
| `LeanString`        | 16 bytes | 16 bytes | Yes            | Yes |


## Special Thanks

The idea and implementation of `LeanString` is inspired by the following projects:

- [EcoString](https://crates.io/crates/ecow)
- [CompactString](https://crates.io/crates/compact_str)

I would like to thank the authors of these projects for their great work.

## License

This crate is licensed under the MIT license.
