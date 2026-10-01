# Third-party notices

## Syzygy tablebase probe

Native builds optionally use `pyrrhic-rs` 0.2.0, licensed under MIT. Its source
was initially transliterated from Pyrrhic, which incorporates Fathom probe code.
Fathom credits Ronald de Man and basil; later modifications credit Jon Dart and
Andrew Grant. Full Pyrrhic, Fathom, libc, and memmap2 notices are retained under `licenses/`.

The native adapter also links `libc` 0.2.189 and `memmap2` 0.9.11. Both are
available under MIT or Apache-2.0; both upstream notices are retained in
`licenses/`. The Rust rules/evaluation/search and WASM crates do not compile or
call the native tablebase adapter.

## Bundled evaluation models

The embedded evaluation models carry the separate attribution and license in
`assets/models/LICENSE`.
