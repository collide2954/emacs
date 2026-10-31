mod common;

use common::check;

#[test]
fn buffers_exist_at_startup() {
    check(&[
        ("(buffer-name)", "\"*scratch*\""),
        ("(bufferp (current-buffer))", "t"),
        ("(current-buffer)", "#<buffer *scratch*>"),
        ("(buffer-name (get-buffer-create \"foo\"))", "\"foo\""),
        ("(eq (get-buffer \"foo\") (get-buffer-create \"foo\"))", "t"),
        ("(generate-new-buffer-name \"foo\")", "\"foo<2>\""),
        ("(buffer-live-p (get-buffer \"foo\"))", "t"),
        ("(kill-buffer \"foo\")", "t"),
        ("(get-buffer \"foo\")", "nil"),
        (
            "(set-buffer \"nonexistent\")",
            "error: (error \"No buffer named nonexistent\")",
        ),
    ]);
}

#[test]
fn startup_default_directory() {
    let mut cwd = std::env::current_dir()
        .expect("cwd")
        .to_string_lossy()
        .into_owned();
    if !cwd.ends_with('/') && !(cfg!(windows) && cwd.ends_with('\\')) {
        cwd.push('/');
    }
    let expected = format!("{cwd:?}");
    check(&[
        ("default-directory", expected.as_str()),
        ("(multibyte-string-p default-directory)", "nil"),
        ("(default-value 'default-directory)", "nil"),
        (
            "(equal default-directory (save-current-buffer (set-buffer \" *Minibuf-0*\") default-directory))",
            "t",
        ),
    ]);
}

#[test]
fn insertion_and_point() {
    check(&[
        ("(insert \"hello\")", "nil"),
        ("(point)", "6"),
        ("(buffer-string)", "\"hello\""),
        ("(goto-char 3)", "3"),
        ("(insert \"XY\")", "nil"),
        ("(buffer-string)", "\"heXYllo\""),
        ("(point)", "5"),
        ("(insert ?! \" end\")", "nil"),
        ("(buffer-substring 1 4)", "\"heX\""),
        ("(buffer-size)", "12"),
        ("(point-min)", "1"),
        ("(point-max)", "13"),
        ("(char-after 1)", "104"),
        ("(char-before 3)", "101"),
        ("(goto-char 100)", "100"),
        ("(point)", "13"),
        ("(eobp)", "t"),
        ("(bobp)", "nil"),
        ("(following-char)", "0"),
        ("(insert \"é\")", "nil"),
        ("(char-before)", "233"),
        ("(position-bytes (point))", "15"),
    ]);
}

#[test]
fn deletion() {
    check(&[
        ("(insert \"abcdef\")", "nil"),
        ("(delete-region 2 4)", "nil"),
        ("(buffer-string)", "\"adef\""),
        ("(point)", "5"),
        ("(delete-and-extract-region 1 3)", "\"ad\""),
        ("(buffer-string)", "\"ef\""),
        ("(erase-buffer)", "nil"),
        ("(buffer-size)", "0"),
        ("(insert \"xyz\")", "nil"),
        ("(goto-char 2)", "2"),
        (
            "(progn (fset 'undo-auto-amalgamate (function (lambda ()))) nil)",
            "nil",
        ),
        ("(delete-char 1)", "nil"),
        ("(buffer-string)", "\"xz\""),
        ("(delete-char 5)", "error: (end-of-buffer)"),
    ]);
}

#[test]
fn markers() {
    check(&[
        ("(insert \"abcdef\")", "nil"),
        ("(setq m (copy-marker 3))", "#<marker at 3 in *scratch*>"),
        ("(goto-char 1)", "1"),
        ("(insert \"XX\")", "nil"),
        ("(marker-position m)", "5"),
        ("(goto-char 5)", "5"),
        ("(insert \"Y\")", "nil"),
        ("(marker-position m)", "5"),
        ("(set-marker-insertion-type m t)", "t"),
        ("(goto-char 5)", "5"),
        ("(insert \"Z\")", "nil"),
        ("(marker-position m)", "6"),
        ("(delete-region 1 8)", "nil"),
        ("(marker-position m)", "1"),
        ("(set-marker m nil)", "#<marker in no buffer>"),
        ("(marker-buffer m)", "nil"),
        ("(markerp (point-marker))", "t"),
        ("(+ 1 (copy-marker 2))", "3"),
    ]);
}

#[test]
fn narrowing_and_excursions() {
    check(&[
        ("(insert \"0123456789\")", "nil"),
        ("(narrow-to-region 3 6)", "nil"),
        ("(buffer-string)", "\"234\""),
        ("(point)", "6"),
        ("(goto-char 1)", "1"),
        ("(point)", "3"),
        (
            "(save-restriction (widen) (buffer-string))",
            "\"0123456789\"",
        ),
        ("(buffer-string)", "\"234\""),
        ("(widen)", "nil"),
        ("(save-excursion (goto-char 1) (insert \"a\") (point))", "2"),
        ("(point)", "4"),
        ("(buffer-string)", "\"a0123456789\""),
        (
            "(save-current-buffer (set-buffer (get-buffer-create \"other\")) (insert \"o\") (buffer-name))",
            "\"other\"",
        ),
        ("(buffer-name)", "\"*scratch*\""),
        (
            "(with-current-buffer-test)",
            "error: (void-function with-current-buffer-test)",
        ),
    ]);
}

#[test]
fn lines() {
    check(&[
        ("(insert \"one\\ntwo\\nthree\")", "nil"),
        ("(goto-char 6)", "6"),
        ("(line-beginning-position)", "5"),
        ("(line-end-position)", "8"),
        ("(pos-bol 2)", "9"),
        ("(pos-eol 0)", "4"),
        ("(bolp)", "nil"),
        ("(forward-line 1)", "0"),
        ("(point)", "9"),
        ("(forward-line 5)", "4"),
        ("(point)", "14"),
        ("(forward-line -1)", "0"),
        ("(point)", "5"),
        ("(forward-line -5)", "-4"),
        ("(beginning-of-line)", "nil"),
        ("(end-of-line)", "nil"),
        ("(point)", "4"),
    ]);
}

#[test]
fn buffer_local_variables() {
    check(&[
        ("(defvar my-local 'global)", "my-local"),
        ("(make-local-variable 'my-local)", "my-local"),
        ("(setq my-local 'local)", "local"),
        ("(default-value 'my-local)", "global"),
        ("(local-variable-p 'my-local)", "t"),
        (
            "(with-temp-other)",
            "error: (void-function with-temp-other)",
        ),
        (
            "(save-current-buffer (set-buffer (get-buffer-create \"b2\")) my-local)",
            "global",
        ),
        ("(buffer-local-value 'my-local (current-buffer))", "local"),
        ("(kill-local-variable 'my-local)", "my-local"),
        ("my-local", "global"),
        ("(make-variable-buffer-local 'auto-local)", "auto-local"),
        ("(setq auto-local 5)", "5"),
        ("(default-value 'auto-local)", "nil"),
        (
            "(save-current-buffer (set-buffer \"b2\") auto-local)",
            "nil",
        ),
        ("fill-column", "70"),
        ("(setq fill-column 50)", "50"),
        ("(local-variable-p 'fill-column)", "t"),
        ("(default-value 'fill-column)", "70"),
        (
            "(setq fill-column 'bad)",
            "error: (wrong-type-argument integerp bad)",
        ),
        ("major-mode", "fundamental-mode"),
        ("(local-variable-p 'major-mode)", "t"),
        ("(let ((case-fold-search nil)) case-fold-search)", "nil"),
        ("case-fold-search", "t"),
    ]);
}

#[test]
fn let_bindings_of_buffer_locals() {
    check(&[
        (
            "(progn (defvar lb 0) (make-variable-buffer-local 'lb) (set-buffer (get-buffer-create \"lb1\")) (let ((other (get-buffer-create \"lb2\"))) (list (let ((lb 1)) (save-current-buffer (set-buffer other) (set 'lb 123) (list (local-variable-p 'lb) lb (default-value 'lb)))) (save-current-buffer (set-buffer other) (list (local-variable-p 'lb) lb)) (default-value 'lb))))",
            "((t 123 1) (t 123) 0)",
        ),
        (
            "(list (let ((lb 1)) (set 'lb 5) (list (local-variable-p 'lb) lb)) lb)",
            "((nil 5) 0)",
        ),
        (
            "(let ((other (get-buffer-create \"lb3\"))) (list (let ((left-margin 4)) (save-current-buffer (set-buffer other) (setq left-margin 9) (list (local-variable-p 'left-margin) (default-value 'left-margin)))) (buffer-local-value 'left-margin other) (default-value 'left-margin)))",
            "((t 4) 9 0)",
        ),
        (
            "(progn (defvar lu 1) (list (let ((lu 2)) (set (make-local-variable 'lu) 3) lu) lu (default-value 'lu) (local-variable-p 'lu)))",
            "(3 3 1 t)",
        ),
        ("(make-local-variable t)", "error: (setting-constant t)"),
        (
            "(make-local-variable 'window-system)",
            "error: (error \"Symbol window-system may not be buffer-local\")",
        ),
    ]);
}

#[test]
fn per_buffer_predicates() {
    check(&[
        ("(get 'vertical-scroll-bar 'choice)", "(nil t left right)"),
        ("(get 'fraction 'range)", "(0.0 . 1.0)"),
        (
            "(setq scroll-up-aggressively 2.7)",
            "error: (error \"Value should be from 0.0 to 1.0\" 2.7)",
        ),
        ("(setq scroll-up-aggressively 0.5)", "0.5"),
        (
            "(setq scroll-up-aggressively 'abc)",
            "error: (wrong-type-argument numberp abc)",
        ),
        (
            "(setq default-directory 5)",
            "error: (wrong-type-argument stringp 5)",
        ),
        (
            "(setq overwrite-mode 'foo)",
            "error: (error \"One of nil, overwrite-mode-textual or overwrite-mode-binary should be specified\" foo)",
        ),
        ("(setq vertical-scroll-bar 'left)", "left"),
    ]);
}

#[test]
fn kill_all_local_variables_kinds() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"kr\")) (make-local-variable 'kr-foo) (put 'kr-foo 'permanent-local t) (setq kr-foo 'bar) (setq buffer-file-name \"/dummy\") (setq fill-column 20) (kill-all-local-variables) (list (local-variable-p 'kr-foo) buffer-file-name fill-column))",
            "(t \"/dummy\" 70)",
        ),
        (
            "(progn (kill-all-local-variables 'reset) (list (local-variable-p 'kr-foo) buffer-file-name))",
            "(nil nil)",
        ),
        (
            "(progn (set-default 'kp 123) (set (make-local-variable 'kp) t) (put 'kp 'permanent-local t) (setq kp-log nil) (add-variable-watcher 'kp (function (lambda (&rest args) (setq kp-log (cons args kp-log))))) (kill-all-local-variables 'permanent-local) (list (local-variable-p 'kp) kp kp-log))",
            "(nil 123 nil)",
        ),
        (
            "(progn (set (make-local-variable 'kp) t) (kill-all-local-variables t) (list (local-variable-p 'kp) (mapcar (function (lambda (e) (nth 2 e))) kp-log)))",
            "(nil (makunbound set))",
        ),
    ]);
}

#[test]
fn undo_records() {
    check(&[
        ("(set-buffer (get-buffer-create \"u\"))", "#<buffer u>"),
        ("(insert \"ab\")", "nil"),
        ("(insert \"c\")", "nil"),
        ("buffer-undo-list", "((1 . 4) (t . 0))"),
        ("(undo-boundary)", "nil"),
        ("(delete-region 1 2)", "nil"),
        ("(car buffer-undo-list)", "(\"a\" . 1)"),
        ("(buffer-modified-p)", "t"),
        ("(set-buffer-modified-p nil)", "nil"),
        ("(buffer-modified-p)", "nil"),
    ]);
}

#[test]
fn text_properties() {
    check(&[
        ("(insert \"abcdef\")", "nil"),
        ("(put-text-property 2 4 'face 'bold)", "nil"),
        ("(get-text-property 3 'face)", "bold"),
        ("(get-text-property 4 'face)", "nil"),
        ("(text-properties-at 2)", "(face bold)"),
        ("(next-single-property-change 1 'face)", "2"),
        ("(next-single-property-change 2 'face)", "4"),
        ("(buffer-substring 1 5)", "#(\"abcd\" 1 3 (face bold))"),
        ("(goto-char 3)", "3"),
        ("(insert \"Z\")", "nil"),
        ("(get-text-property 3 'face)", "nil"),
        ("(get-text-property 4 'face)", "bold"),
        ("(insert (propertize \"Q\" 'k 1))", "nil"),
        ("(get-text-property 4 'k)", "1"),
        ("(remove-text-properties 1 (point-max) '(face nil))", "t"),
        ("(next-single-property-change 1 'face)", "nil"),
        ("(propertize \"x\" 'a 1 'b 2)", "#(\"x\" 0 1 (a 1 b 2))"),
        (
            "(let ((s (copy-sequence \"abc\"))) (add-text-properties 1 2 '(p t) s) s)",
            "#(\"abc\" 1 2 (p t))",
        ),
        ("(insert (propertize \"ro\" 'read-only t))", "nil"),
        ("(goto-char (- (point) 1))", "6"),
        ("(insert \"x\")", "error: (text-read-only)"),
    ]);
}

#[test]
fn overlays() {
    check(&[
        ("(insert \"0123456789\")", "nil"),
        (
            "(setq ov (make-overlay 3 6))",
            "#<overlay from 3 to 6 in *scratch*>",
        ),
        ("(overlay-put ov 'face 'italic)", "italic"),
        ("(get-char-property 4 'face)", "italic"),
        ("(get-char-property 7 'face)", "nil"),
        ("(goto-char 1)", "1"),
        ("(insert \"ab\")", "nil"),
        ("(list (overlay-start ov) (overlay-end ov))", "(5 8)"),
        ("(length (overlays-at 6))", "1"),
        ("(length (overlays-in 1 3))", "0"),
        ("(delete-overlay ov)", "nil"),
        ("(overlay-buffer ov)", "nil"),
    ]);
}

#[test]
fn change_hooks() {
    check(&[
        ("(setq log nil)", "nil"),
        ("(add-hook-test)", "error: (void-function add-hook-test)"),
        (
            "(setq after-change-functions (list #'(lambda (b e l) (setq log (cons (list b e l) log)))))",
            "(#[(b e l) ((setq log (cons (list b e l) log))) nil])",
        ),
        ("(insert \"abc\")", "nil"),
        ("(delete-region 1 2)", "nil"),
        ("log", "((1 1 1) (1 4 0))"),
        ("(setq after-change-functions nil)", "nil"),
        ("(setq buffer-read-only t)", "t"),
        (
            "(insert \"x\")",
            "error: (buffer-read-only #<buffer *scratch*>)",
        ),
        ("(let ((inhibit-read-only t)) (insert \"x\"))", "nil"),
    ]);
}

#[test]
fn format() {
    check(&[
        ("(format \"%s and %S\" \"a\" \"b\")", "\"a and \\\"b\\\"\""),
        (
            "(format \"%d %o %x %X %c\" 42 8 255 255 ?A)",
            "\"42 10 ff FF A\"",
        ),
        ("(format \"%5d|%-5d|%05d\" 1 2 3)", "\"    1|2    |00003\""),
        (
            "(format \"%.2f %e %g\" 3.14159 1234.5 0.0001)",
            "\"3.14 1.234500e+03 0.0001\"",
        ),
        ("(format \"%g\" 1e10)", "\"1e+10\""),
        ("(format \"%%\")", "\"%\""),
        ("(format \"%2$s %1$s\" 'a 'b)", "\"b a\""),
        ("(format \"%.3s\" \"abcdef\")", "\"abc\""),
        ("(format \"%d\" 1.9)", "\"1\""),
        ("(format \"%x\" -255)", "\"-ff\""),
        ("(format \"%#x %#o\" 255 8)", "\"0xff 010\""),
        ("(format \"%+d % d\" 5 5)", "\"+5  5\""),
        ("(format \"%d\" (expt 2 70))", "\"1180591620717411303424\""),
        (
            "(format \"%s\")",
            "error: (error \"Not enough arguments for format string\")",
        ),
        (
            "(format \"%d\" \"x\")",
            "error: (error \"Format specifier doesn't match argument type\")",
        ),
        (
            "(format \"%q\" 1)",
            "error: (error \"Invalid format operation %q\")",
        ),
        ("(format-message \"`foo'\")", "\"‘foo’\""),
        ("(let ((s \"plain\")) (eq s (format s)))", "t"),
        ("(format \"%s\" 'sym)", "\"sym\""),
        (
            "(format (propertize \"%s!\" 'face 'bold) \"hi\")",
            "#(\"hi!\" 0 3 (face bold))",
        ),
        (
            "(format \"%s\" (propertize \"in\" 'p 1))",
            "#(\"in\" 0 2 (p 1))",
        ),
        (
            "(format \"<%s>\" (propertize \"in\" 'p 1))",
            "#(\"<in>\" 1 3 (p 1))",
        ),
    ]);
}

#[test]
fn files() {
    check(&[
        ("(file-name-directory \"/a/b/c.el\")", "\"/a/b/\""),
        ("(file-name-nondirectory \"/a/b/c.el\")", "\"c.el\""),
        ("(file-name-as-directory \"/a/b\")", "\"/a/b/\""),
        ("(directory-file-name \"/a/b/\")", "\"/a/b\""),
        ("(file-name-concat \"a\" \"b/\" \"c\")", "\"a/b/c\""),
        ("(expand-file-name \"c\" \"/a/b/\")", "\"/a/b/c\""),
        ("(expand-file-name \"../c\" \"/a/b/\")", "\"/a/c\""),
        ("(expand-file-name \"./x/./y\" \"/a/\")", "\"/a/x/y\""),
        ("(file-name-absolute-p \"/x\")", "t"),
        ("(file-name-absolute-p \"x\")", "nil"),
        ("(substitute-in-file-name \"/a//b\")", "\"/b\""),
        ("(substitute-in-file-name \"$$x\")", "\"$x\""),
    ]);
}

#[test]
fn mode_line_update() {
    check(&[
        ("(force-mode-line-update)", "nil"),
        ("(force-mode-line-update t)", "t"),
    ]);
}
