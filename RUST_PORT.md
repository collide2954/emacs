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

## Removed legacy

`nextstep/`, `msdos/`, `oldXMenu/`, `lwlib/`, `java/`, `cross/` and `exec/` are gone, along
with the NS, Haiku, Android, MS-DOS, Cygwin, Windows-console and termcap/terminfo sources.
The X and pgtk sources stay as references until the iced backend covers what they do.

## GUI (iced)

`crates/emacs-gui` replaces the platform frontends. It follows the pgtk design: iced/winit
stands in for GTK, tiny-skia for cairo, and cosmic-text for FreeType and HarfBuzz.

- The Lisp machine runs on its own thread. iced owns the main thread, opens one window
  per frame, and forwards input.
- Input: a key press becomes an X keysym and is classified as `pgtkterm.c`'s
  `key_press_event` does, then travels to the Lisp thread as a `termhooks::InputEvent`.
  keyboard.c's keysym tables turn it into a Lisp event. Alt is meta and the logo key is super.
- Output: redisplay draws each frame into a tiny-skia pixmap on the Lisp thread, as pgtk
  draws into a cairo image surface. A flush hands the pixels to iced, which shows them.
- Fonts: a cosmic-text font driver supplies metrics and glyphs, in the role of `ftcrfont.c`.
- The frame primitives (`x-create-frame` and friends, as in `pgtkfns.c`) live in the GUI
  crate and are registered into the interpreter at startup.

Done: the input event model (`termhooks.rs`) and key translation (`keysym.rs`).

Next: the pixmap surface and its drawing primitives, the iced shell with the Lisp-thread
bridge, and the cosmic-text font driver. Showing a real frame also needs the display engine
in emacs-core (`frame.c`, `window.c`, `dispnew.c`, `xdisp.c`, `xfaces.c`, `font.c`,
`fontset.c`) and the command loop in `keyboard.c`.
