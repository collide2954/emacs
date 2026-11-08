mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;

fn mark_unchanged(lisp: &mut Lisp) {
    let t = &mut lisp.cur_mut().text;
    t.unchanged_modified = t.modiff;
    t.overlay_unchanged_modified = t.overlay_modiff;
    let gpt = t.gpt() as i64;
    t.beg_unchanged = gpt - 1;
    t.end_unchanged = (t.chars + 1) as i64 - gpt;
}

fn unchanged(lisp: &Lisp) -> (i64, i64, usize) {
    let t = &lisp.cur().text;
    (t.beg_unchanged, t.end_unchanged, t.gpt())
}

fn buffer_with_text(lisp: &mut Lisp) {
    assert_eq!(
        eval(
            lisp,
            "(progn (set-buffer (get-buffer-create \"u\")) (insert \"abcdefghij\") (buffer-size))"
        ),
        "10"
    );
}

#[test]
fn a_new_buffer_starts_with_nothing_known_unchanged() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        eval(&mut lisp, "(set-buffer (get-buffer-create \"n\"))");
        let t = &lisp.cur().text;
        assert_eq!((t.beg_unchanged, t.end_unchanged, t.gpt()), (0, 0, 1));
        assert_eq!((t.unchanged_modified, t.overlay_unchanged_modified), (1, 1));
    });
}

#[test]
fn insertions_and_deletions_shrink_the_unchanged_ends() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        buffer_with_text(&mut lisp);
        assert_eq!(lisp.cur().text.gpt(), 11);
        mark_unchanged(&mut lisp);
        assert_eq!(unchanged(&lisp), (10, 0, 11));
        eval(&mut lisp, "(progn (goto-char 4) (insert \"X\"))");
        assert_eq!(unchanged(&lisp), (3, 0, 5));
        mark_unchanged(&mut lisp);
        assert_eq!(unchanged(&lisp), (4, 7, 5));
        eval(&mut lisp, "(progn (goto-char 8) (insert \"Y\"))");
        assert_eq!(unchanged(&lisp), (7, 4, 9));
        assert_eq!(eval(&mut lisp, "(delete-region 2 3)"), "nil");
        assert_eq!(unchanged(&lisp), (1, 4, 2));
        assert_eq!(eval(&mut lisp, "(buffer-string)"), "\"acXdefYghij\"");
        mark_unchanged(&mut lisp);
        assert_eq!(eval(&mut lisp, "(delete-region 9 7)"), "nil");
        assert_eq!(unchanged(&lisp), (6, 3, 7));
        assert_eq!(eval(&mut lisp, "(buffer-string)"), "\"acXdefhij\"");
    });
}

#[test]
fn property_and_overlay_changes_shrink_the_unchanged_ends() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        buffer_with_text(&mut lisp);
        mark_unchanged(&mut lisp);
        eval(&mut lisp, "(put-text-property 5 7 'face 'bold)");
        assert_eq!(unchanged(&lisp), (3, 4, 11));
        mark_unchanged(&mut lisp);
        let modiff = lisp.cur().text.overlay_modiff;
        eval(&mut lisp, "(setq o (make-overlay 2 4))");
        assert_eq!(lisp.cur().text.overlay_modiff, modiff);
        assert_eq!(unchanged(&lisp), (10, 0, 11));
        eval(&mut lisp, "(overlay-put o 'face 'bold)");
        assert_eq!(unchanged(&lisp), (1, 7, 11));
        assert_eq!(lisp.cur().text.overlay_modiff, modiff + 1);
        mark_unchanged(&mut lisp);
        eval(&mut lisp, "(overlay-put o 'face 'bold)");
        assert_eq!(unchanged(&lisp), (10, 0, 11));
        eval(&mut lisp, "(move-overlay o 6 8)");
        assert_eq!(unchanged(&lisp), (1, 3, 11));
        mark_unchanged(&mut lisp);
        eval(&mut lisp, "(move-overlay o 6 9)");
        assert_eq!(unchanged(&lisp), (7, 2, 11));
        mark_unchanged(&mut lisp);
        eval(&mut lisp, "(delete-overlay o)");
        assert_eq!(unchanged(&lisp), (5, 2, 11));
        mark_unchanged(&mut lisp);
        let modiff = lisp.cur().text.overlay_modiff;
        eval(&mut lisp, "(progn (goto-char 3) (insert \"Z\"))");
        assert_eq!(lisp.cur().text.overlay_modiff, modiff);
    });
}
