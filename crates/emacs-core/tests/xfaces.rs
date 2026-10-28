mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn realized_faces() {
    on_big_stack(|| {
        let mut lisp = Lisp::new_with(false);
        for (source, expected) in [
            (
                "(internal-get-lisp-face-attribute 'default :foreground (selected-frame))",
                "\"unspecified-fg\"",
            ),
            (
                "(internal-get-lisp-face-attribute 'default :background (selected-frame))",
                "\"unspecified-bg\"",
            ),
            (
                "(list (internal-get-lisp-face-attribute 'default :family (selected-frame)) (internal-get-lisp-face-attribute 'default :height (selected-frame)) (internal-get-lisp-face-attribute 'default :weight (selected-frame)))",
                "(\"default\" 1 normal)",
            ),
            (
                "(vectorp (internal-lisp-face-p 'mode-line-active (selected-frame)))",
                "t",
            ),
            ("(display-supports-face-attributes-p '(:underline t))", "t"),
            (
                "(display-supports-face-attributes-p '(:underline nil))",
                "nil",
            ),
            (
                "(display-supports-face-attributes-p '(:weight bold))",
                "nil",
            ),
            (
                "(display-supports-face-attributes-p '(:underline (:style dots)))",
                "nil",
            ),
            (
                "(display-supports-face-attributes-p '(:underline (:style wave)))",
                "t",
            ),
            (
                "(progn (internal-make-lisp-face 'xr) (internal-set-lisp-face-attribute 'xr :foreground 'reset (selected-frame)) (aref (face-attributes-as-vector '(xr)) 9))",
                "\"unspecified-fg\"",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
    });
    check(&[(
        "(display-supports-face-attributes-p '(:underline t))",
        "nil",
    )]);
}

#[test]
fn lisp_faces() {
    check(&[
        ("(length (internal-make-lisp-face 'xf-a))", "20"),
        ("(aref (internal-make-lisp-face 'xf-a) 0)", "face"),
        ("(get 'xf-a 'face)", "0"),
        (
            "(progn (internal-make-lisp-face 'xf-b) (get 'xf-b 'face))",
            "1",
        ),
        ("(vectorp (internal-lisp-face-p 'xf-a))", "t"),
        ("(internal-lisp-face-p 'xf-none)", "nil"),
        ("(internal-lisp-face-empty-p 'xf-a t)", "t"),
        (
            "(internal-set-lisp-face-attribute 'xf-a :foreground \"red\" t)",
            "xf-a",
        ),
        (
            "(internal-get-lisp-face-attribute 'xf-a :foreground t)",
            "\"red\"",
        ),
        ("(internal-lisp-face-empty-p 'xf-a t)", "nil"),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-a :weight 'unspecified t) (aref (internal-lisp-face-p 'xf-a) 5))",
            ":ignore-defface",
        ),
        (
            "(internal-get-lisp-face-attribute 'xf-a :weight t)",
            "unspecified",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-a :weight 'zork t)",
            "error: (error \"Invalid face weight\" zork)",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-a :zork 1 t)",
            "error: (error \"Invalid face attribute name\" :zork)",
        ),
        (
            "(internal-get-lisp-face-attribute 'xf-none :weight t)",
            "error: (error \"Invalid face\" xf-none)",
        ),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-a :italic t t) (internal-get-lisp-face-attribute 'xf-a :slant t))",
            "italic",
        ),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-a :bold nil t) (internal-get-lisp-face-attribute 'xf-a :weight t))",
            "normal",
        ),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-a :box t t) (internal-get-lisp-face-attribute 'xf-a :box t))",
            "1",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-a :box '(:line-width 0) t)",
            "error: (error \"Invalid face box\" :line-width 0)",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-a :underline '(:style zigzag) t)",
            "error: (error \"Invalid face underline\" :style zigzag)",
        ),
        (
            "(progn (internal-make-lisp-face 'default) (internal-set-lisp-face-attribute 'default :height 1.5 t))",
            "error: (error \"Default face height not absolute and positive\" 1.5)",
        ),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-a :height 1.5 t) (internal-get-lisp-face-attribute 'xf-a :height t))",
            "1.5",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-a :fontset \"x\" t)",
            "error: (error \"Window system is not in use or not initialized\")",
        ),
    ]);
}

#[test]
fn inheritance_and_frames() {
    check(&[
        (
            "(progn (internal-make-lisp-face 'xf-p) (internal-make-lisp-face 'xf-c) nil)",
            "nil",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-c :inherit 'xf-p t)",
            "xf-c",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-p :inherit 'xf-c t)",
            "error: (error \"Face inheritance results in inheritance cycle\" xf-c)",
        ),
        (
            "(internal-set-lisp-face-attribute 'xf-p :inherit '(1) t)",
            "error: (error \"Invalid face inheritance\" 1)",
        ),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-p :foreground \"blue\" t) (internal-lisp-face-equal-p 'xf-c 'xf-p t))",
            "nil",
        ),
        ("(internal-lisp-face-equal-p 'xf-c 'xf-p t t)", "t"),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-c :background \"red\" (selected-frame)) (internal-get-lisp-face-attribute 'xf-c :background))",
            "\"red\"",
        ),
        ("(hash-table-count (frame--face-hash-table))", "1"),
        (
            "(progn (internal-merge-in-global-face 'xf-c (selected-frame)) (internal-get-lisp-face-attribute 'xf-c :inherit))",
            "xf-p",
        ),
        (
            "(progn (internal-copy-lisp-face 'xf-p 'xf-q t nil) (internal-get-lisp-face-attribute 'xf-q :foreground t))",
            "\"blue\"",
        ),
        (
            "(progn (internal-set-lisp-face-attribute 'xf-p :foreground nil (selected-frame)) (internal-get-lisp-face-attribute 'xf-p :foreground))",
            "unspecified",
        ),
        (
            "(save-current-buffer (set-buffer \"*Messages*\") (buffer-string))",
            "\"Warning: setting attribute ‘:foreground’ of face ‘xf-p’: nil value is invalid, use ‘unspecified’ instead.\n\"",
        ),
    ]);
}

#[test]
fn face_merging() {
    check(&[
        (
            "(aref (face-attributes-as-vector '(:foreground \"red\" :weight bold)) 5)",
            "bold",
        ),
        (
            "(aref (face-attributes-as-vector '((:foreground \"red\") (:foreground \"blue\"))) 9)",
            "\"red\"",
        ),
        (
            "(aref (face-attributes-as-vector '(foreground-color . \"green\")) 9)",
            "\"green\"",
        ),
        ("(aref (face-attributes-as-vector '(:height 2.0)) 4)", "2.0"),
        (
            "(progn (internal-make-lisp-face 'xf-m) (internal-set-lisp-face-attribute 'xf-m :slant 'italic (selected-frame)) (aref (face-attributes-as-vector '(xf-m (:weight bold))) 6))",
            "italic",
        ),
        ("(face-attribute-relative-p :height 1.5)", "t"),
        ("(face-attribute-relative-p :height 10)", "nil"),
        ("(face-attribute-relative-p :weight 'unspecified)", "t"),
        ("(merge-face-attribute :height 2.0 10)", "20"),
        ("(merge-face-attribute :weight 'unspecified 'bold)", "bold"),
        (
            "(internal-lisp-face-attribute-values :underline)",
            "(t nil)",
        ),
        (
            "(display-supports-face-attributes-p '(:weight bold))",
            "nil",
        ),
    ]);
}

#[test]
fn font_selection_and_alists() {
    check(&[
        (
            "(internal-set-font-selection-order '(:width :height :weight :slant))",
            "nil",
        ),
        (
            "(internal-set-font-selection-order '(:width :height))",
            "error: (error \"Invalid font sort order\" :width :height)",
        ),
        (
            "(internal-set-font-selection-order '(:width :height :weight :foo))",
            "error: (error \"Invalid font sort order\" :width :height :weight :foo)",
        ),
        (
            "(internal-set-alternative-font-family-alist '((\"Mono\" \"Courier\")))",
            "((Mono Courier))",
        ),
        (
            "(internal-set-alternative-font-registry-alist '((\"ISO8859-1\" \"ISO8859-15\")))",
            "((\"iso8859-1\" \"iso8859-15\"))",
        ),
        (
            "(fontset-list)",
            "(\"-*-*-*-*-*-*-*-*-*-*-*-*-fontset-default\")",
        ),
        (
            "(query-fontset \"fontset-default\")",
            "error: (error \"Window system is not in use or not initialized\")",
        ),
    ]);
}

#[test]
fn colors() {
    check(&[
        (
            "(color-values-from-color-spec \"#fff\")",
            "(65535 65535 65535)",
        ),
        (
            "(color-values-from-color-spec \"#102030\")",
            "(4112 8224 12336)",
        ),
        (
            "(color-values-from-color-spec \"rgb:f/80/000\")",
            "(65535 32896 0)",
        ),
        (
            "(color-values-from-color-spec \"rgbi:0/0.5/1\")",
            "(0 32768 65535)",
        ),
        ("(color-values-from-color-spec \"rgbi:0/1.5/1\")", "nil"),
        ("(color-values-from-color-spec \"#12\")", "nil"),
        ("(color-values-from-color-spec \"red\")", "nil"),
        ("(color-distance '(0 0 0) '(0 0 0))", "0"),
        ("(color-distance '(65535 65535 65535) '(0 0 0))", "589805"),
        (
            "(color-distance \"white\" '(0 0 0))",
            "error: (error \"Invalid color\" \"white\")",
        ),
        (
            "(color-distance '(1 2 3) '(4 5 6) nil (function (lambda (a b) (list a b))))",
            "((1 2 3) (4 5 6))",
        ),
        (
            "(color-distance 3 '(0 0 0))",
            "error: (error \"Invalid color\" 3)",
        ),
        ("(color-gray-p \"white\")", "nil"),
        ("(color-supported-p \"white\")", "nil"),
        ("(bitmap-spec-p \"file.xbm\")", "t"),
        ("(bitmap-spec-p '(8 2 \"ab\"))", "t"),
        ("(bitmap-spec-p '(9 2 \"ab\"))", "nil"),
        ("(bitmap-spec-p '(0 2 \"ab\"))", "nil"),
        ("(tty-suppress-bold-inverse-default-colors t)", "t"),
    ]);
}
