# Rust port of GNU Emacs

The `rust` branch of this fork ports the C core of GNU Emacs to Rust.
The Lisp tree under `lisp/` is used unmodified.

## Decisions

- A Rust core (`crates/emacs-core`) boots the existing `lisp/` tree through `loadup.el`.
- Lisp objects are tagged handles (`Value(u64)`) into typed arenas with a tracing collector.
  The workspace forbids `unsafe` code, except in `crates/emacs-w32` and `crates/emacs-unix`,
  which wrap the few Win32 calls (through `windows-sys`) and unix calls (through `libc`) the
  port needs in safe functions.
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
| `crates/emacs-w32` | Safe wrappers around Win32 calls (code pages, locale names, known folders, the registry, the process environment, process probing, no-replace renames, the tick count); empty on other platforms |
| `crates/emacs-unix` | Safe wrappers around unix calls std lacks (`kill`, utmpx, `CLOCK_BOOTTIME`, `sysinfo`, the BSD boot-time sysctl, no-replace renames); empty on Windows |

The unix code is type-checked from Windows with `cargo clippy --target
x86_64-unknown-linux-gnu`; the macOS and BSD branches of `emacs-unix` and `boot_time.rs`
have not been compiled yet.

Build and run:

```
cargo build
target/debug/emacs --batch
```

The checkout must keep LF line endings (`git config core.autocrlf false` on Windows, as
`nt/INSTALL` requires): files with a `-unix` coding cookie keep their CRs otherwise, and
autoload generation garbles them.

Generate `lisp/subdirs.el` and the loaddefs files the way `lisp/Makefile` does, with the
port itself. Run `build-aux/update-subdirs DIR` for `lisp` and each of its subdirectories
except `cedet` and `leim` (and theirs), then:

```
target/debug/emacs --batch --no-site-file --no-site-lisp \
  -l lisp/emacs-lisp/loaddefs-gen.el -f loaddefs-generate--emacs-batch DIRS
```

where DIRS is `lisp` and all its subdirectories except `obsolete` and `term`, sorted as
make's `$(sort)` sorts them. This writes `lisp/loaddefs.el`, `cl-loaddefs.el`,
`theme-loaddefs.el` and the other `*-loaddefs.el` files (all ignored by git); loadup then
loads `loaddefs.el` instead of `ldefs-boot.el`. The result matches `ldefs-boot.el` except
for upstream changes since that file was refreshed and four `define-minor-mode` doc
strings that gain a `\\{...-map}` line, because the source-loaded preloaded Lisp has
already loaded `compile`, `flymake` and `rect` (C's byte-compiled preload doesn't).

## Status

C files fully replaced and deleted: `keymap.c`, `regex-emacs.c`, `search.c`, `casetab.c`,
`category.c`, `timefns.c`, `charset.c`, `ccl.c`, `filelock.c`.

Ported but not yet complete (C kept):

| C file | Rust | Missing |
| --- | --- | --- |
| `alloc.c`, `data.c`, `eval.c`, `fns.c`, `lread.c`, `print.c`, `floatfns.c`, `chartab.c` | same names | assorted primitives |
| `character.c` | `charfns.rs` | `get-byte`, display-table aware `string-width` |
| `emacs.c` | `emacs.rs` | argument handling (`sort_args`, `argmatch`, the option sequence in `main`, `init_cmdargs`) is ported, but `-t` and the daemon options exit with an error because there is no tty frontend and no daemon support; `kill-emacs` and `shut_down_emacs` are ported, except that a string ARG is never stuffed as input (C only does that on a tty) and `shut_down_emacs` has no subprocesses to kill; on Windows, RESTART starts the new Emacs from the saved argv with Rust's argument quoting and inherited standard handles, not from the raw `GetCommandLine` string |
| `buffer.c`, `marker.c`, `insdel.c`, `textprop.c`, `editfns.c`, `fileio.c` | same names | assorted primitives; the overlay DEFUNs in `overlay.rs` are still approximations (only `overlays_in`, `overlays_at` and `sort_overlays` follow C), and `replace_range` is approximated by `replace_range_chars`; `do-auto-save`, the auto-save DEFUNs, `insert-file-contents` and `write-region` follow C; `prepare_to_modify_buffer` locks the visited file as C does but doesn't save the region text for `select-active-regions` |
| `cmds.c` | `editfns.rs` | nothing |
| `dired.c` | `dired.rs` | on Windows, inode, device and link count come from w32.c's fallback (std can't read them without `unsafe`) |
| `callint.c` | `callint.rs` | `call-interactively` (needs the command loop) |
| `process.c` | `process.rs` | everything but `get-buffer-process` (which finds nothing, as no process objects exist yet), `num-processors`, which ports gnulib's `nproc` on top of `std::thread::available_parallelism` (so on Windows it ignores the process affinity mask), and `wait_reading_process_output`, which is the subprocess-free variant C uses on MS-DOS (it runs timers and sleeps, and never wakes for keyboard input because there is no input queue yet) |
| `term.c` | `term.rs` | `tty-frame-list-z-order`, `tty-frame-at` (need frame z-order); there is no tty frontend, so `decode_tty_terminal` never finds a tty |
| `w32.c` | `w32.rs` | everything but `init_environment`, which doesn't report a missing temporary directory (that needs `command-error-default-function`), and leaves out `init_user_info` and w32term.c's mouse-button count; it sets variables with `SetEnvironmentVariableA` where C uses `_putenv`, and `getenv-internal` falls back to the process environment as C's does on Windows |
| `w32proc.c` | `w32proc.rs` | the DEFUNs other than `w32-long-file-name`, `w32-get-console-codepage`, `w32-set-console-codepage` and their output-codepage counterparts; it has the variables |
| `keyboard.c` | `keyboard.rs` | everything except event modifier parsing, `safe_run_hooks`, the timer runner (`timer_check`), `detect_input_pending` and the input-mode DEFUNs (`current-input-mode`, `set-input-mode` and friends, which only affect ttys, so they change nothing but the interrupt flag); there is no input event queue yet, so pending input means `quit-flag` or a kboard's `kbd_queue_has_data`, and idle timers never run because Emacs never becomes idle |
| `syntax.c` | `syntax.rs` | the regex engine still ignores `syntax-table` text properties |
| `indent.c` | `indent.rs` | `compute-motion`, `vertical-motion`, `line-number-display-width` (they need the display iterator); an image `display` spec signals an error |
| `casefiddle.c` | `casefiddle.rs` | a real port; the current version is a placeholder using Rust's Unicode tables |
| `coding.c` | `coding.rs`, `coding/` | `encode_string_utf_8`/`decode_string_utf_8` (for `json.c`), file-name encoding |
| `bytecode.c` | `bytecode.rs` | nothing but two opcodes obsolete since Emacs 24.1: `temp-output-buffer-setup`/`-show` signal an error; `debug-on-next-call` is not honored (as elsewhere in the port). The VM keeps its operands on `Lisp::stack` (so they are GC roots) and recurses for calls where C chains frames on its own stack; primitives are called through the subr table by name, not through function cells, as C calls `Fcar` and friends directly |
| `terminal.c` | `terminal.rs` | `delete-terminal` (needs `delete-frame`) |
| `frame.c` | `frame.rs` | `delete-frame`, `x-create-frame`; `adjust_frame_size` is ported but stops with `unreachable!` where it would call a window system's size hook or resize child frames, which don't exist yet |
| `window.c` | `window.rs` | splitting, resizing (`resize_frame_windows` handles only a leaf root window, the only kind the port can make), `window-list`; window configurations (`current-window-configuration`, `set-window-configuration`, `window-configuration-equal-p` and the dead-windows table) follow C |
| `minibuf.c` | `minibuf.rs` | interactive reading (needs the command loop; it signals an error) |
| `font.c` | `font.rs` | font drivers, entities, `font_load_for_lface`; `internal-char-font` is ported |
| `fontset.c` | `fontset.rs` | `new-fontset`, `set-fontset-font`, `fontset-info`, `fontset-font` |
| `xfaces.c` | `xfaces.rs` | GUI and tty face realization, `face-font`, `x-list-fonts`, `x-family-fonts`, `internal-face-x-get-resource`; `face_at_buffer_position` is ported |
| `xdisp.c` | `xdisp.rs` | redisplay and the display iterator; it has the variables, the redisplay flags, `add_to_log`, `message_dolog`, the `message3`/`message_to_stderr` echo path, the message stack (`push_message`, `restore_message`) and `invisible-p` |
| `composite.c` | `composite.rs` | shaping on window-system frames |

Window-system frames don't exist yet, so the face, font and composition code stops with
`unreachable!` on the branches that need them. Realized faces work for the initial frame,
which gets plain realized faces as in C.

`doc.c` is fully ported (`doc.rs`). `dispnew.rs` has the `dispnew.c` variables,
`frame-or-buffer-changed-p`, `ding`, `internal-show-cursor(-p)`, `sleep-for`, `sit_for`
(which never redisplays, as there is no redisplay yet) and `redraw-frame` (which marks the
frame and its windows for redisplay but has no glyph matrices or terminal hooks to clear).
`w32fns.rs` defines `w32-ansi-code-page` and `w32-multibyte-code-page` on Windows.
`filelock.rs` ports `filelock.c` (locks are symlinks on unix and nonce-renamed regular files
on Windows, as in C), with gnulib's `get_boot_time` as `boot_time.rs`. Whether a lock's owner
is alive is checked with `kill(pid, 0)` through `emacs-unix`, or with C's `sys_kill`
emulation (`OpenProcess`) through `emacs-w32`. Lock file names come from files.el's
`make-lock-file-name`, so a bare `-nl` Emacs can't write files, just like C's `temacs`.
`image.rs`, `gnutls.rs`, `treesit.rs`, `macros.rs` and `w32fns.rs` hold only those files'
variables, which the unmodified Lisp needs (cus-start.el checks them). No image, GnuTLS or
tree-sitter library is linked, so their version variables are -1.

GC roots for non-local exits: a signal or throw pushes its values on `Lisp::flow_roots`,
because Rust code can run Lisp (for example `unbind_to`) while an `Err` is still on its
way to the handler, where C's conservative stack scan would keep it alive. A catch keeps
the caught values rooted while its handler runs; `pop_handler` and a finished catch
truncate back to the handler's level. Unbinding (and so `unwind-protect` cleanups) happens
at the handler, as in C's `unwind_to_catch`, but Rust frames truncate `Lisp::stack` as the
exit passes through them, so when a signal or throw picks its target it copies the
evaluated arguments of every backtrace frame it will unwind into a rooted list; a cleanup
that calls `backtrace` then sees the same arguments C's still-intact frames show.

ERT runs upstream test files only partly: `current-window-configuration` isn't ported,
and a failing `should` is reported as "aborted with non-local exit" instead of "failed".
(Without generated loaddefs, cl-lib's autoloads such as `cl-coerce` are missing too; see
Build and run.)

`crates/emacs-unidata` replaces the awk half of `admin/unidata`: `crates/emacs/build.rs`
writes `lisp/international/charscript.el` and `emoji-zwj.el`. The `uni-*.el` and
`charprop.el` files still need `unidata-gen.el` run by a working Emacs.

`emacs --batch` loads every preloaded file in `loadup.el`, including the `w32-*` files and
`cus-start.el`, runs `Snarf-documentation`, then runs startup to the end. `main.rs`
decodes the command line as C's `main` does and `Lisp::boot` runs `init_cmdargs`, so
`emacs --batch --eval EXPR`, `--version` and `--help` work, and a plain `emacs --batch`
ends in `kill-emacs` and exits successfully, as C does. The port behaves like an
undumped `temacs`: `top-level` is `(load "loadup.el")` unless `-nl` is given.
`Lisp::noninteractive` is C's `noninteractive` and the Lisp variable is `noninteractive1`;
they differ after `-x`, as in C.

On Windows, `init_environment` sets `LANG`, `TMPDIR`, `HOME` and the other registry and
default values as C does, and `set-locale-environment` sets `locale-coding-system` from the
console code page (as C does in batch mode), so startup converts `default-directory` from
C's raw `GetCurrentDirectory` form.

Byte compilation works: bytecomp.el, loaded from source, compiles its own hot functions
and then `byte-compile` and `byte-compile-file` compile correctly (symbols with positions
are accepted wherever C's `SYMBOLP` accepts them: `check_symbol`, `eval`, `funcall`,
`macroexpand` and the `symbolp` opcode). The `.elc` files that `byte-compile-file` writes
load and run with `emacs --batch -l`, lazy `#@` docstrings included.

`emacs` without `--batch` stops while loading `elisp-mode.el` with
`(wrong-type-argument stringp nil)`. This is expected for now: the preloaded Lisp loads
from source, so eager macro-expansion of `flymake-log` loads `flymake.el`, which requires
`project.el`, whose `defcustom` calls `locate-user-emacs-file` before
`user-emacs-directory` is set (outside batch mode that function checks the directory). C Emacs never meets this, because it runs `loadup.el` only in batch to make the
dump and an interactive start restores the dump. Interactive startup needs the heap
snapshot (the pdumper replacement) or byte-compiled Lisp.

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
`decode_coding_gap`, sets `last-coding-system-used`, and handles REPLACE. `write-region` is a
port too: `choose_write_coding_system`, `build_annotations` (with `format-annotate-function`
and `write-region-post-annotation-function`), `a_write` and `e_write` encoding through
`encode_coding_object`, numeric APPEND offsets, MUSTBENEW and the modtime bookkeeping, which
`verify-visited-file-modtime`, `visited-file-modtime` and `set-visited-file-modtime` read and
set. `find-file`, `save-buffer` and `revert-buffer` work in batch, with file locking.
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
