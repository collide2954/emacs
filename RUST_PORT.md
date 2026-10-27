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
| `coding.c` | `coding.rs`, `coding/` | `encode_string_utf_8`/`decode_string_utf_8` (for `json.c`), file-name encoding |
| `bytecode.c` | `bytecode.rs` | the VM (stub) |

`emacs --batch` loads `loadup.el` through `bindings`, `window`, `files`, `macroexp` and
`cus-face`, and stops in `faces.el` on `internal-set-font-selection-order` (`xfaces.c`).

## Coding

The conversion engine is in place: `coding/engine.rs` (detection, EOL handling,
produce/consume, the decode and encode drivers, `decode_coding_object`,
`encode_coding_object`, `decode_coding_gap`), and one module per codec family:
`utf.rs`, `simple.rs` (raw-text, charset), `cjk.rs` (Shift-JIS, Big5, CCL), `mule.rs`
(emacs-mule) and `iso2022.rs`. `coding/api.rs` holds the Lisp API (`decode-coding-*`,
`encode-coding-*`, `detect-coding-*`, the encodability queries,
`find-operation-coding-system`, the SJIS/Big5 character functions).

`insert-file-contents` is a port of `fileio.c`: it decides the coding system with
`set-auto-coding-function` and `find-operation-coding-system`, decodes through
`decode_coding_gap`, sets `last-coding-system-used`, and handles REPLACE. `fileio.c` itself
is still mostly unported (modtime checks, `write-region` encoding, file locking), and
`update_compositions` waits for `composite.c`.

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

Done:

- `termhooks.rs` (emacs-core): the input event model.
- `keysym.rs`: key translation.
- `surface.rs`: the frame surface with the cairo operations pgtk uses (fill, clear,
  copy-bits for scrolling, clip rectangles, glyph masks, RGBA images, alpha background).
- `protocol.rs`, `display.rs`, `shell.rs`: the iced daemon (one window per frame, HiDPI
  aware) and the Lisp-thread side that owns the surfaces and receives input.
- `font.rs`: the font driver, after `ftcrfont.c`/`ftfont.c`. It lists and matches faces with
  fontconfig's numeric weight/slant/width/spacing and its generic family preferences, opens
  fonts with ftcrfont's metric rules, and provides glyph extents, rasterization (including
  color glyphs) and drawing onto a surface.

`cargo run -p emacs-gui --example shell` opens a frame from the Lisp thread, paints a test
pattern and prints the events it receives.

Next: showing a real Emacs frame needs the display engine in emacs-core (`frame.c`, `window.c`, `dispnew.c`, `xdisp.c`, `xfaces.c`, `font.c`,
`fontset.c`) and the command loop in `keyboard.c`.
