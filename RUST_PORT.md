# Rust port of GNU Emacs

The `rust` branch of this fork ports the C core of GNU Emacs to Rust.
The Lisp tree under `lisp/` is used unmodified.

## Decisions

- A Rust core (`crates/emacs-core`) boots the existing `lisp/` tree through `loadup.el`.
- Lisp objects are tagged handles (`Value(u64)`) into typed arenas with a tracing collector.
  The workspace forbids `unsafe` code.
- The interpreter and bytecode come first. Native compilation (Cranelift) comes later.
- The GUI is reimplemented on iced 0.14. The terminal frontend is dropped, and `--batch` stays.
- A C file is deleted in its own commit once its port is complete.
- Every commit passes `cargo fmt`, `cargo clippy -D warnings` and `cargo test`, and bumps the workspace version.

## Layout

| Crate | Purpose |
| --- | --- |
| `crates/emacs-core` | Interpreter, data types, buffers and primitives |
| `crates/emacs` | The `emacs` binary; its build script generates the charset maps |
| `crates/emacs-charsets` | Port of `admin/charsets` (`mapconv` and its awk scripts); writes `etc/charsets/*.map`, `cp51932.el` and `eucjp-ms.el` |

Build and run:

```
cargo build
target/debug/emacs --batch
```

## Status

C files fully replaced and deleted: `keymap.c`, `regex-emacs.c`, `search.c`, `casetab.c`,
`category.c`, `timefns.c`, `charset.c`, `ccl.c`.

Ported but not yet complete (C kept):

| C file | Rust | Missing |
| --- | --- | --- |
| `alloc.c`, `data.c`, `eval.c`, `fns.c`, `lread.c`, `print.c`, `floatfns.c`, `chartab.c` | same names | assorted primitives |
| `character.c` | `charfns.rs` | `get-byte`, display-table aware `string-width` |
| `buffer.c`, `marker.c`, `insdel.c`, `textprop.c`, `editfns.c`, `fileio.c`, `emacs.c` | same names | assorted primitives |
| `keyboard.c` | `keyboard.rs` | everything except event modifier parsing |
| `syntax.c` | `syntax.rs` | scanning (`forward-word`, `skip-chars-*`, `scan-lists`, `parse-partial-sexp`, ...) |
| `casefiddle.c` | `casefiddle.rs` | a real port; the current version is a placeholder using Rust's Unicode tables |
| `coding.c` | `coding.rs` | the conversion engine (see below) |
| `bytecode.c` | `bytecode.rs` | the VM (stub) |

`emacs --batch` loads `loadup.el` through `subr`, `keymap`, `custom`, `mule`, `mule-conf`,
`env` and `format`, and stops in `bindings.el` on `function-key-map` (a `keyboard.c` variable).

## Where the coding work stopped

Done: the coding-system registry (`define-coding-system-internal` and friends), the
conversion state fields on `CodingSystem`, and the buffer primitives the engine needs
(`insert_from_gap`, `del_range_2`).

In progress, kept out of the tree in a local git stash (`coding engine WIP`):
`coding/engine.rs` (detection, EOL handling, produce/consume, the decode and encode
drivers, `decode_coding_object`/`encode_coding_object`, the conversion work buffer) and
`coding/utf.rs` (UTF-8 and UTF-16 codecs).

To resume: `git stash pop`, add `mod engine; mod utf;` to `coding.rs`, then port the
remaining codecs from `coding.c`: raw-text and charset (lines 5264-5656), SJIS and Big5
(4575-5068), CCL (5068-5264), emacs-mule (1829-2848), ISO-2022 (2848-4575). After that,
the Lisp API (8634-10720): `decode-coding-string`/`region`, `encode-coding-*`,
`detect-coding-*`, `find-coding-systems-region-internal`, `unencodable-char-position`,
`check-coding-systems-region`, `find-operation-coding-system`, and the SJIS/Big5 character
functions. `update_compositions` waits for `composite.c`.

## Next

The iced GUI (`crates/emacs-gui`).
