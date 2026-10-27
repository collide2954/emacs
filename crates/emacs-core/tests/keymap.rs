mod common;

use common::check;

#[test]
fn keymap_construction() {
    check(&[
        ("(keymapp (make-sparse-keymap))", "t"),
        ("(keymapp '(foo))", "nil"),
        ("(make-sparse-keymap \"Menu\")", "(keymap \"Menu\")"),
        ("(char-table-p (car (cdr (make-keymap))))", "t"),
        ("(keymap-prompt (make-sparse-keymap \"Menu\"))", "\"Menu\""),
        (
            "(let ((m (make-sparse-keymap))) (define-key m \"a\" 'foo) m)",
            "(keymap (97 . foo))",
        ),
    ]);
}

#[test]
fn prefix_and_meta_bindings() {
    check(&[
        ("(setq m (make-sparse-keymap))", "(keymap)"),
        ("(define-key m \"\\C-xl\" 'count-lines)", "count-lines"),
        ("(lookup-key m \"\\C-xl\")", "count-lines"),
        ("(lookup-key m \"\\C-x\")", "(keymap (108 . count-lines))"),
        ("(lookup-key m \"\\C-xlz\")", "2"),
        ("(lookup-key m \"q\")", "nil"),
        ("(define-key m \"\\M-f\" 'forward-word)", "forward-word"),
        ("(assq 27 m)", "(27 keymap (102 . forward-word))"),
        ("(lookup-key m [?\\M-f])", "forward-word"),
        ("(lookup-key m \"\\ef\")", "forward-word"),
        (
            "(let ((k (make-sparse-keymap))) (define-key k \"a\" 'x) (define-key k \"ab\" 'y))",
            "error: (error \"Key sequence a b starts with non-prefix key a\")",
        ),
        (
            "(define-key (make-sparse-keymap) [C-RET] 'x)",
            r#"error: (error "To bind the key C-RET, use [?\\C-\\r], not [C-RET]")"#,
        ),
    ]);
}

#[test]
fn defaults_removal_and_ranges() {
    check(&[
        (
            "(let ((m (make-sparse-keymap))) (define-key m [t] 'dflt) (list (lookup-key m \"x\") (lookup-key m \"x\" t)))",
            "(nil dflt)",
        ),
        (
            "(let ((m (make-sparse-keymap))) (define-key m \"a\" 'x) (define-key m \"a\" nil t) m)",
            "(keymap)",
        ),
        (
            "(let ((m (make-sparse-keymap))) (define-key m [(97 . 99)] 'abc) (list (lookup-key m \"b\") (char-table-p (car (cdr m)))))",
            "(abc t)",
        ),
        (
            "(let ((m (make-keymap))) (define-key m \"a\" 'x) (lookup-key m \"a\"))",
            "x",
        ),
    ]);
}

#[test]
fn inheritance_and_copying() {
    check(&[
        (
            "(let ((p (make-sparse-keymap)) (m (make-sparse-keymap))) (define-key p \"a\" 'pa) (set-keymap-parent m p) (list (keymap-parent m) (lookup-key m \"a\")))",
            "((keymap (97 . pa)) pa)",
        ),
        (
            "(let ((m (make-sparse-keymap))) (set-keymap-parent m m))",
            "error: (error \"Cyclic keymap inheritance\")",
        ),
        (
            "(let* ((m (make-sparse-keymap)) (c (progn (define-key m \"\\C-xa\" 'x) (copy-keymap m)))) (define-key m \"\\C-xa\" 'z) (list (lookup-key c \"\\C-xa\") (lookup-key m \"\\C-xa\")))",
            "(x z)",
        ),
        (
            "(let ((m (make-sparse-keymap)) r) (define-key m \"a\" 'x) (define-key m \"b\" 'y) (map-keymap (function (lambda (k v) (setq r (cons (cons k v) r)))) m) r)",
            "((97 . x) (98 . y))",
        ),
    ]);
}

#[test]
fn key_descriptions() {
    check(&[
        ("(key-description [?\\C-x ?l])", "\"C-x l\""),
        ("(key-description \"\\M-x\")", "\"M-x\""),
        ("(key-description [27 ?x])", "\"M-x\""),
        ("(key-description [27])", "\"ESC\""),
        ("(key-description \"l\" \"\\C-x\")", "\"C-x l\""),
        ("(key-description [])", "\"\""),
        ("(single-key-description 'f1)", "\"<f1>\""),
        ("(single-key-description 'C-M-down)", "\"C-M-<down>\""),
        ("(single-key-description 'f1 t)", "\"f1\""),
        ("(single-key-description ?\\C-a)", "\"C-a\""),
        ("(single-key-description 127)", "\"DEL\""),
        ("(single-key-description 32)", "\"SPC\""),
        ("(single-key-description ?\\t)", "\"TAB\""),
        ("(single-key-description ?\\r)", "\"RET\""),
        ("(single-key-description ?\\M-\\t)", "\"C-M-i\""),
        ("(single-key-description '(97 . 99))", "\"a..c\""),
        ("(text-char-description ?\\C-c)", "\"^C\""),
        ("(text-char-description ?a)", "\"a\""),
        ("(text-char-description 127)", "\"^?\""),
    ]);
}

#[test]
fn event_symbols() {
    check(&[
        ("(event-convert-list '(control meta ?a))", "134217729"),
        ("(event-convert-list '(control f1))", "C-f1"),
        ("(event-convert-list '(shift ?a))", "65"),
        (
            "(internal-event-symbol-parse-modifiers 'C-M-down-mouse-1)",
            "(mouse-1 meta control down)",
        ),
        (
            "(internal-event-symbol-parse-modifiers 'mouse-1)",
            "(mouse-1 click)",
        ),
        ("(get 'C-f1 'event-symbol-element-mask)", "nil"),
    ]);
}

#[test]
fn active_maps_and_lookup() {
    check(&[
        (
            "(let ((g (make-sparse-keymap))) (use-global-map g) (define-key g \"q\" 'quit-x) (list (eq (current-global-map) g) (key-binding \"q\")))",
            "(t quit-x)",
        ),
        ("(defvar my-mode t)", "my-mode"),
        (
            "(let ((mm (make-sparse-keymap))) (define-key mm \"z\" 'zz) (setq minor-mode-map-alist (list (cons 'my-mode mm))) (list (key-binding \"z\") (minor-mode-key-binding \"z\")))",
            "(zz ((my-mode . zz)))",
        ),
        (
            "(let ((g (make-sparse-keymap))) (define-key g [remap kill-line] 'my-kill) (command-remapping 'kill-line nil (list g)))",
            "my-kill",
        ),
        (
            "(let ((m (make-sparse-keymap))) (use-local-map m) (eq (current-local-map) m))",
            "t",
        ),
    ]);
}

#[test]
fn where_is_and_accessible_maps() {
    check(&[
        ("(setq m (make-sparse-keymap))", "(keymap)"),
        ("(define-key m \"\\C-xa\" 'foo)", "foo"),
        ("(where-is-internal 'foo (list m))", "([24 97])"),
        ("(where-is-internal 'foo (list m) t)", "[24 97]"),
        (
            "(accessible-keymaps m)",
            "(([] keymap (24 keymap (97 . foo))) ([24] keymap (97 . foo)))",
        ),
        (
            "(accessible-keymaps m \"\\C-x\")",
            "(([24] keymap (97 . foo)))",
        ),
    ]);
}
