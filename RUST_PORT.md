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

Then byte-compile the preloaded Lisp. First compile `emacs-lisp/macroexp.el`, `cconv.el`,
`byte-opt.el` and `bytecomp.el`, one `target/debug/emacs --batch --no-site-file
--no-site-lisp -f batch-byte-compile FILE` each, as `lisp/Makefile`'s `compile-first`
does. Then compile the files a batch boot lists in `load-history` (except the loaddefs
files and `subdirs.el`) with `load-prefer-newer` and `byte-compile-warnings` set as in
`BYTE_COMPILE_FLAGS`; one session for all of them works, since loadup has already loaded
them all, as upstream's bootstrap Emacs has. The `.elc` files are ignored by git. This
takes about 8 minutes with a debug build. `.elc` files written before 0.1.69 have wrong
docstring positions (their bytecode strings hold raw bytes instead of octal escapes), so
rebuild them.

Then generate the Unicode property tables as `admin/unidata/Makefile` does, with the port,
from `admin/unidata` (where the build has written `unidata.txt`). Byte-compile
`unidata-gen.el` with `--batch -f batch-byte-compile unidata-gen.el`, then run one process
per file:

```
target/debug/emacs --batch --no-site-file --no-site-lisp -L . -l unidata-gen \
  -f unidata-gen-file ../../lisp/international/uni-NAME.el .
```

for each `uni-*.el` that `unidata-gen.el` lists, then `-f unidata-gen-charprop` for
`charprop.el`, `-l emoji.el -f emoji--generate-file` for `emoji-labels.el`, and
`-l unidata-gen.el` with `-f unidata-gen-scripts`, `-f unidata-gen-confusable` and
`-f unidata-gen-idna-mapping` for `uni-scripts.el`, `uni-confusable.el` and
`idna-mapping.el`. Each takes a few seconds except `emoji-labels.el` (about two minutes).
The files are ignored by git. Once `charprop.el` exists, loadup loads it, `characters.el`
builds its tables from the Unicode data, and Tramp loads.

## Status

C files fully replaced and deleted: `keymap.c`, `regex-emacs.c`, `search.c`, `casetab.c`,
`category.c`, `timefns.c`, `charset.c`, `ccl.c`, `filelock.c`. The gnulib hashes behind
`secure-hash` are ported too (their C sources stay in `lib/`): `md5.rs`, `sha1.rs`,
`sha256.rs` (SHA-224 and SHA-256), `sha512.rs` (SHA-384 and SHA-512) and `sha3.rs`.

Ported but not yet complete (C kept):

| C file | Rust | Missing |
| --- | --- | --- |
| `alloc.c`, `data.c`, `eval.c`, `fns.c`, `floatfns.c` | same names | assorted primitives |
| `chartab.c` | `chartab.rs` | `map_char_table`'s C-callback form: Rust callers use `map_char_table_collect`, which gathers the ranges first; `char_table_ref` callers that only borrow `Lisp` decode a compressed uniprop block without replacing it |
| `atimer.c` | `atimer.rs` | `debug-timer-check` (checking builds only) and the timerfd and POSIX-timer paths: on every platform a thread stands in for `SIGALRM`, as `w32proc.c`'s `setitimer` emulation does on Windows, and sets `pending_signals`, which `maybe_quit` acts on; sleeps in `wait_reading_process_output` do not yet wake for a ripe timer |
| `print.c` | `print.rs` | assorted primitives (`print_error_message` and `error-message-string` are ported); the printer builds its output as characters before writing it to the destination, but applies `print_prepare`'s and `print_string`'s destination rules: raw bytes become octal escapes in multibyte buffers and in `prin1-to-string` (which `.elc` docstring positions depend on) and reach a function as Latin-1 characters |
| `lread.c` | `lread.rs` | assorted primitives; `load`, `openp` and `locate-file-internal` follow C (handlers, `load-prefer-newer`, the `.elc` version check, emacs-mule `.elc` files from before Emacs 22, text-mode reading of source files on Windows), but a loaded file is read whole and decoded up front rather than byte by byte, and the lexical-binding cookie check is an approximation of `lisp_file_lexical_cookie` |
| `character.c` | `charfns.rs` | `get-byte`, display-table aware `string-width` |
| `emacs.c` | `emacs.rs` | argument handling (`sort_args`, `argmatch`, the option sequence in `main`, `init_cmdargs`) is ported, but `-t` and the daemon options exit with an error because there is no tty frontend and no daemon support; `kill-emacs` and `shut_down_emacs` are ported, except that a string ARG is never stuffed as input (C only does that on a tty) and `shut_down_emacs` has no subprocesses to kill; on Windows, RESTART starts the new Emacs from the saved argv with Rust's argument quoting and inherited standard handles, not from the raw `GetCommandLine` string |
| `buffer.c`, `marker.c`, `insdel.c`, `textprop.c`, `editfns.c`, `fileio.c` | same names | assorted primitives; the overlay DEFUNs in `overlay.rs` are still approximations (only `overlays_in`, `overlays_at` and `sort_overlays` follow C), and `replace_range` is approximated by `replace_range_chars`; `do-auto-save`, the auto-save DEFUNs, `insert-file-contents` and `write-region` follow C; `prepare_to_modify_buffer` locks the visited file as C does but doesn't save the region text for `select-active-regions` |
| `cmds.c` | `editfns.rs` | nothing |
| `dired.c` | `dired.rs` | on Windows, inode, device and link count come from w32.c's fallback (std can't read them without `unsafe`) |
| `callint.c` | `callint.rs` | nothing |
| `process.c` | `process.rs` | everything but `processp` and the Lisp fields of `struct Lisp_Process` (nothing creates a process yet), `get-buffer-process` (which finds nothing, as no process objects exist yet), `num-processors`, which ports gnulib's `nproc` on top of `std::thread::available_parallelism` (so on Windows it ignores the process affinity mask), and `wait_reading_process_output`, which is the subprocess-free variant C uses on MS-DOS (it runs timers, and when it reads keyboard input it waits on the window system's event channel, where C waits on a file descriptor) |
| `term.c` | `term.rs` | `tty-frame-list-z-order`, `tty-frame-at` (need frame z-order); there is no tty frontend, so `decode_tty_terminal` never finds a tty |
| `w32.c` | `w32.rs` | everything but `init_environment`, which doesn't report a missing temporary directory (that needs `command-error-default-function`), and leaves out `init_user_info` and w32term.c's mouse-button count; it sets variables with `SetEnvironmentVariableA` where C uses `_putenv`, and `getenv-internal` falls back to the process environment as C's does on Windows |
| `w32proc.c` | `w32proc.rs` | the DEFUNs other than `w32-long-file-name`, `w32-get-console-codepage`, `w32-set-console-codepage` and their output-codepage counterparts; it has the variables |
| `sysdep.c` | `sysdep.rs` | everything but the random-number functions (`get_random`, `get_random_ulong`, `seed_random`, `init_random`); the generator under them follows `w32.c`'s AS183 `random` and the C runtime's `rand` on Windows, and reimplements glibc's `random` elsewhere so each Lisp instance keeps its own state; `getrandom` (for `secure-hash`'s `iv-auto`) uses `CryptGenRandom` on Windows |
| `doprnt.c` | `doprnt.rs` | truncation to a fixed-size buffer (only `vmessage`'s interactive path and `sysdep.c`'s `snprintf` fallback need it); `Lisp::doprnt` grows its output like `evxprintf`, and `Lisp::error`/`verror` take C's formats with typed `doprnt::Arg`s |
| `callproc.c` | `emacs.rs` (`run_process`) | a placeholder: `call-process` and `call-process-region` run the program through Rust's process API, decode its output as UTF-8 and append its standard error after its standard output; only the returned status follows C (`WEXITSTATUS` of the wait status, which `w32proc.c` builds on Windows, or `strsignal`'s description of the signal) |
| `keyboard.c` | `keyboard.rs`, `keyboard/` | the command loop is ported: the event queue the GUI fills through pgtk's `read_socket`, `read_char`, `read_key_sequence` (with `input-decode-map`, `local-function-key-map` and `key-translation-map`), `command_loop`, `command_loop_1` (with `adjust_point_for_property`), `recursive-edit`, `top-level`, `exit-recursive-edit`, `abort-recursive-edit` and `keys_of_keyboard`. Not ported yet: menus (keymap menu prompting and mouse menus), mouse, touch and user-signal events, help-echo display, the hourglass, executing keyboard macros, `record_menu_key`, stack-overflow recovery and `input_blocked_p` (the port never blocks input); the parts that need redisplay (`resize_echo_area_exactly`, the minibuffer message timeout) stop with `unreachable!`, and forcing a window start doesn't reset the buffer's unchanged regions. As in C builds with threads, a batch session enters `command_loop_1` once the top-level form returns and waits for input, so a `-nl` script must end with `kill-emacs`. `init_keyboard` only sets `command_loop_level` to -1 (the rest is done when `Lisp` is built). Kboards live in a vector, so `delete_kboard` marks one deleted and wipes it instead of unlinking it |
| `macros.c` | `macros.rs` | everything but the variables, `cancel-kbd-macro-events` and `end_kbd_macro` (defining and running keyboard macros needs the command loop) |
| `syntax.c` | `syntax.rs` | the regex engine still ignores `syntax-table` text properties |
| `indent.c` | `indent.rs` | `compute-motion`, `vertical-motion`, `line-number-display-width` (they need the display iterator); an image `display` spec signals an error |
| `casefiddle.c` | `casefiddle.rs` | a real port; the current version is a placeholder using Rust's Unicode tables |
| `coding.c` | `coding.rs`, `coding/` | `encode_string_utf_8`/`decode_string_utf_8` (for `json.c`), file-name encoding |
| `bytecode.c` | `bytecode.rs` | nothing but two opcodes obsolete since Emacs 24.1: `temp-output-buffer-setup`/`-show` signal an error; `debug-on-next-call` is not honored (as elsewhere in the port). The VM keeps its operands on `Lisp::stack` (so they are GC roots) and recurses for calls where C chains frames on its own stack; primitives are called through the subr table by name, not through function cells, as C calls `Fcar` and friends directly |
| `terminal.c` | `terminal.rs` | nothing; `delete_terminal_internal` leaves the coding systems to be dropped with the terminal object |
| `frame.c` | `frame.rs` | in `delete_frame`, the pgtk selection clearing and event swallowing (there are no selections or event queue yet) and `free_glyphs` (there are no glyph matrices, so `glyphs_initialized_p` stays false and, as in C, messages on a window frame are only logged and errors go to standard error); `do_switch_frame` does not resize the old frame's mini-window or redirect focus; of the window-system parameter code (`gui_display_get_arg`, `gui_default_parameter`, `gui_set_frame_parameters`, `frame_float` and the generic `gui_set_*` handlers, dispatched through the terminal's `frame_parm_handlers`), the divider-width and `scroll-bar-width` handlers (they need glyph matrices; `gui_set_font` and `gui_set_font_backend` are ported, minus the image-cache rescaling, as there are no images yet); `adjust_frame_size` is ported but stops with `unreachable!` where it would call a window system's size hook or resize child frames, which don't exist yet |
| `window.c` | `window.rs` | splitting, resizing (`resize_frame_windows` handles only a leaf root window, the only kind the port can make), `window-list`; window configurations (`current-window-configuration`, `set-window-configuration`, `window-configuration-equal-p` and the dead-windows table) follow C, as do `set-window-fringes`, `set-window-scroll-bars`, `set-window-margins`, `window-scroll-bars`, `set-window-start`, `set-window-point` and the mode/header/tab line, divider and scroll bar size DEFUNs (without glyph matrices, line heights always come from `estimate_mode_line_height`; horizontal scroll bars don't exist, as in C without toolkit scroll bars) |
| `minibuf.c` | `minibuf.rs` | `cancel_hourglass` on entry (the hourglass isn't ported, so it would do nothing); interactive reading (`read_minibuf` with `read_minibuf_unwind`, `minibuffer_unwind` and `run_exit_minibuf_hook`), recursive minibuffers, `move_minibuffers_onto_frame` and `zip_minibuffer_stacks` are ported. On exit, `resize_mini_window` only resets the window start while the frame has no glyph matrices (see `xdisp.c`), and the cursor update after the recursive edit does nothing until `update_frame` exists |
| `font.c` | `font.rs` | `font-face-attributes`, `font-shape-gstring`, `font-variation-glyphs`, `font-get-glyphs`, `font-at`, `font_range`, the OTF DEFUNs; `font_put_frame_data` (unused with cairo); the driver layer (`struct font_driver` as `FontDriver`, per-frame driver lists, font caches, listing, sorting, matching, opening, `font_load_for_lface`, `font_open_by_name`, the font log) and its DEFUNs are ported |
| `fontset.c` | `fontset.rs` | `fontset-info` and `font_for_char`; the realized-fontset lookup (`fontset_get_font_group`, `reorder_font_vector`, `fontset_find_font`, `fontset_font`, `face_for_char`) is ported, sorting font groups stably where C calls `qsort`; `map_charset_chars` takes a Rust callback (`CharsetMapFunction::Rust`) where C passes a C function |
| `xfaces.c` | `xfaces.rs` | tty face realization, stipples (there are no bitmaps yet), `face-font`, `x-list-fonts`, `x-family-fonts`, `internal-face-x-get-resource`; `face_at_buffer_position` and GUI face realization (`realize_gui_face`, the default face from the frame font, colors, boxes, underlines, `prepare_face_for_display`, freeing faces and their fontsets, `realize_non_ascii_face`, `face_for_font`) are ported |
| `xdisp.c` | `xdisp.rs` | redisplay and the display iterator; it has the variables, the redisplay flags, `add_to_log`, `message_dolog`, the `message3`/`message_to_stderr` echo path, the message stack (`push_message`, `restore_message`), `invisible-p`, `window_box_height`, and `tab-bar-height`/`tool-bar-height`, which stop with `unreachable!` once a frame has a tab or tool bar window (none can be made yet). `resize_mini_window` is ported up to the display iterator: on a frame without glyph matrices it returns without resizing (nothing has been displayed, so the mini-window never grew), and stops with `unreachable!` once a frame has them |
| `composite.c` | `composite.rs` | shaping on window-system frames |
| `bidi.c` | `bidi.rs` | the paragraph-start region cache (`region-cache.c` isn't ported, so paragraph starts are always found by scanning, as C does when `cache-long-scans` is nil; only the `MAX_PARAGRAPH_SEARCH` cutoff can differ); the cache and its shelving follow C, and the bit fields of `struct bidi_stack` keep their 3-bit truncation. xdisp.c's `compute_display_string_pos`/`_end`, `current-bidi-paragraph-direction` and `bidi-find-overridden-directionality` are ported; the display iterator, which drives `bidi_move_to_visually_next`, is not yet |

Window-system frames don't exist yet, so the face, font and composition code stops with
`unreachable!` on the branches that need them. Realized faces work for the initial frame,
which gets plain realized faces as in C.

`doc.c` is fully ported (`doc.rs`). `dispnew.rs` has the `dispnew.c` variables,
`frame-or-buffer-changed-p`, `ding`, `internal-show-cursor(-p)`, `sleep-for`, `sit_for`
(which never redisplays, as there is no redisplay yet) and `redraw-frame` (which marks the
frame and its windows for redisplay but has no glyph matrices or terminal hooks to clear);
`update_frame` does nothing on a frame without glyph matrices and stops with `unreachable!`
once a frame has them.
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

ERT runs upstream test files, with the generated loaddefs and the byte-compiled preloaded
Lisp from Build and run, plus byte-compiled copies of the files ERT loads (`ert`,
`backtrace`, `cl-print`, `pp`, `debug`, `ewoc`, `find-func`, `help-mode`, `help-fns`,
`cl-extra` and `radix-tree`, compiled in one session like the preloaded files):
`emacs --batch -Q -l ert -l test/src/data-tests.el -f ert-run-tests-batch-and-exit` passes
all 64 tests in a few seconds. `test/src/eval-tests.el` passes 25 of 27; the other two run a
child Emacs and compare its whole output, which starts with loadup's messages because the
port boots like an undumped temacs. `test/src/fns-tests.el` passes 86 of 95 (6 skipped) in
about three minutes; the failures need `object-intervals` on buffers, the error C signals
when a hash function changes its own table, and `make-pipe-process`. Printing a failing test's backtrace takes about four seconds,
mostly spent printing ERT's interpreted test closures. `cl-preloaded.el` checks which type
predicates are defined while it is compiled, so `.elc` files from before 0.1.75, when
`processp` was added, lack the `process` type; rebuild them.

`crates/emacs-unidata` replaces the awk and sed half of `admin/unidata`:
`crates/emacs/build.rs` writes `lisp/international/charscript.el`, `emoji-zwj.el` and
`admin/unidata/unidata.txt`, each only when its sources are newer. The port runs the Lisp
half itself (see Build and run), and `get-char-code-property` reads the generated tables
through `chartab.c`'s uniprop code (compressed blocks, decoders and encoders).

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

With the preloaded Lisp byte-compiled (see Build and run), `emacs` without `--batch` runs
`loadup.el` and startup to the end in about three seconds and exits, as there is no
command loop or frontend yet; `--batch` boots in about six seconds instead of a minute.
Without the `.elc` files an interactive start stops while loading `elisp-mode.el` with
`(wrong-type-argument stringp nil)`: eager macro-expansion of `flymake-log` loads
`flymake.el`, which requires `project.el`, whose `defcustom` calls
`locate-user-emacs-file` before `user-emacs-directory` is set. C Emacs never meets this,
because its preloaded files are always compiled.

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
- `ftcrfont.rs`: the `ftcr` font driver (`ftcrfont.c`, with the parts of `ftfont.c` it
  uses), registered globally at startup. fontdb stands in for fontconfig. Listing filters faces
  by family (generic `monospace`, `sans-serif` and `sans` resolved first, names compared
  ignoring case and blanks), spacing, scalability and the uniquifier characters of the
  registry's `fc_charset_table` entry, checked against each face's cmap. Matching takes the
  best face for the family alone, as `ftfont_spec_pattern` leaves weight, slant and width out
  of the pattern. Faces have no foundry, so a spec naming one lists nothing; `:lang` is not
  checked; a spec with `:otf` lists nothing (there is no OpenType layout support yet); `draw`
  does not clip yet.

`cargo run -p emacs-gui --example shell` opens a frame from the Lisp thread, paints a test
pattern and prints the events it receives.

Lisp sees the iced frontend as the `pgtk` window system for now, so the unmodified
`term/pgtk-win.el` drives it; the plan is to consolidate into a window system of its own
later. `emacs` without `--batch`, `-nw` or `--version` runs the iced daemon on the main
thread and the Lisp machine on its own thread. `emacs_gui::install` does what
`syms_of_pgtkterm`, `syms_of_pgtkfns`, `syms_of_pgtkselect` and `syms_of_pgtkim` do (the
variables, the `pgtk`, `gtk` and `cairo` features, whose version strings give the iced and
tiny-skia versions) and registers the window system's hooks, and `init_display` then sets
`initial-window-system` to `pgtk` as C's `HAVE_PGTK` branch does. Ported so far:
`x-open-connection` and `x-display-list` (`pgtk_term_init` without the GTK plumbing: a pgtk
terminal, its display info and kboard; the display is named after `$WAYLAND_DISPLAY` or
`$DISPLAY`, else `iced`, and only that one display can be opened), `x-get-resource`
(`frame.c`) and `pgtk-set-resource`, which behave as pgtk built without GSettings (no
defaults database). Colors go through `gdkrgba.rs`, a port of GTK 3's `gdk_rgba_parse` and
Pango's `pango_color_parse_with_alpha` (CSS `rgb()`/`rgba()`, `#` hex forms, and the X11 names
of `etc/rgb.txt` matched ignoring case and spaces), so `pgtk_parse_color`,
`pgtk_defined_color`, `x_decode_color` and the `xw-color-*` primitives behave as under GTK;
`gtk_selection_bg_color` and `gtk_selection_fg_color` are undefined, as there is no GTK style
context. Terminals carry C's hook table (`TerminalHooks`); pgtk fills in
`get_string_resource`, `defined_color`, `set_new_font` (`pgtk_new_font`), `default_font_parameter` (`pgtk_default_font_parameter`, without GSettings' system font) and `frame_parm_handlers`, which so far holds the generic `gui_set_*` handlers; the `pgtk_set_*` ones are still missing. The fields of C's display info that
generic code reads through `FRAME_DISPLAY_INFO` (resolution, font counts and the smallest
character size, the font-cache list) are `terminal::DisplayInfo` on the core terminal; the
pgtk-only fields stay in the GUI crate's `PgtkDisplayInfo`. Font drivers are `FontDriver`
tables of function pointers, as C's `struct font_driver`; a font object keeps C's
`struct font` metrics in `FontObject` and its driver's private data behind an `Rc<dyn Any>`,
dropped with the object as C's GC calls `close_font`. `x-create-frame` (`pgtkfns.c`) is ported: it builds a pgtk frame with its
per-frame `PgtkOutput`, registers the `ftcr` driver, picks the default font, applies the
default parameters, realizes faces, sizes the frame with `gui_figure_window_size`, creates the
frame's surface (C's `xg_create_frame_widgets`) and, when visible, asks iced to open the window
(`pgtk_make_frame_visible`); `pgtk_set_window_size` resizes the surface and the window. The
frame's handlers so far are the colors, cursor and mouse color, name and title; menu, tab and
tool bar lines, borders, icons, cursor types, child frames and the window-manager hints are
not ported. A frame becomes visible only when its map event is handled, which needs the
command loop, so `frame-visible-p` is nil for now; making a frame invisible only clears its
flags; a frame whose font can't be opened is unwound instead of going through
`delete_frame`. Deleting a pgtk frame (`pgtk_destroy_window`) frees its faces and output and
closes its iced window; as in GTK builds, a pgtk terminal is kept when its last frame goes, and
`x-close-connection` or `delete-terminal` (`pgtk_delete_terminal`, `pgtk_delete_display`)
removes it. `pgtk-register-dnd-targets` records a frame's drop targets (drops themselves need
the event loop). The display queries that GDK answers without monitors are ported
(`x-display-planes`, `x-display-color-cells`, `x-display-screens`, `x-display-visual-class`,
`x-display-backing-store`, `x-display-save-under`, `x-server-max-request-size`); the
monitor-based ones (`x-display-pixel-width`/`-height`, `x-display-mm-width`/`-height`,
`pgtk-display-monitor-attributes-list`) need a monitor list, which iced does not expose, so
they wait for a platform source (Win32 monitor enumeration in `emacs-w32`). An interactive
`emacs -Q` runs all of `command-line`: it creates the first frame, deletes the initial
terminal frame and its terminal, sets up faces and fonts (non-ASCII characters get faces
through the realized fontsets) and then enters the command loop. Key and focus events from
iced go through pgtk's `read_socket` into the event queue, `read_key_sequence` looks them up
and commands run through `command-execute` and `call-interactively`; timers run while Emacs
waits for input. The minibuffer works too: `read-string` and `M-x` (with `completing-read`)
read typed input and record history. Nothing is drawn yet, so typing can only be observed
from Lisp (a timer that prints the buffer, for instance); an error still ends the session,
because `command-error-default-function` prints to standard error and kills Emacs while the
frame has no glyph matrices.

Next: redisplay (glyph matrices, `adjust_frame_glyphs`, the display iterator and
`echo_area_display`), which also brings map events and frame visibility; with the iterator,
`resize_mini_window` and `update_frame` lose their glyph-less shortcuts.
