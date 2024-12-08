# LeanString

**⚠️This is a work in progress.**

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
- [ ] API compatibility with `String`.
- [ ] Fuzz testing.
- [ ] Benchmarking.
- [ ] Documentation.
- etc...

## Example

```rust
// TODO
```

## Which should I use?

TODO: Compare `LeanString` with `String`, `EcoString`, `CompactString`, etc...

## Special Thanks

The idea and implementation of `LeanString` is inspired by the following projects:

- [EcoString](https://crates.io/crates/ecow)
- [CompactString](https://crates.io/crates/compact_str)

I would like to thank the authors of these projects for their great work.

## License

This crate is licensed under the MIT license.
