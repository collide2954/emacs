mod common;

use common::{eval, on_big_stack};
use emacs_core::bidi::{BIDI_EOB, BidiDir, BidiIt};
use emacs_core::lisp::Lisp;

const TABLES: &str = "(progn (put 'char-code-property-table 'char-table-extra-slots 5) (setq bidi-tab (make-char-table 'char-code-property-table)) (set-char-table-range bidi-tab t 23) (set-char-table-range bidi-tab '(97 . 122) 1) (set-char-table-range bidi-tab '(#x5d0 . #x5ea) 2) (set-char-table-range bidi-tab '(48 . 57) 3) (aset bidi-tab 32 22) (aset bidi-tab 10 6) (aset bidi-tab #x202e 11) (aset bidi-tab #x202c 12) (setq mirror-tab (make-char-table 'char-code-property-table)) (aset mirror-tab 40 41) (aset mirror-tab 41 40) (setq bracket-tab (make-char-table 'char-code-property-table)) (set-char-table-range bracket-tab t 1) (aset bracket-tab 40 2) (aset bracket-tab 41 3) (setq char-code-property-alist (list (cons 'bidi-class bidi-tab) (cons 'mirroring mirror-tab) (cons 'bracket-type bracket-tab))) nil)";

fn visual_order(lisp: &mut Lisp, text: &str) -> Vec<(i64, i8)> {
    let insert = format!("(progn (erase-buffer) (insert \"{text}\") nil)");
    assert_eq!(eval(lisp, &insert), "nil");
    let mut it = BidiIt::default();
    lisp.bidi_init_it(1, 1, false, &mut it).expect("init");
    lisp.bidi_paragraph_init(BidiDir::NeutralDir, &mut it, false)
        .expect("paragraph");
    let zv = lisp.cur().zv as i64;
    let mut out = Vec::new();
    loop {
        lisp.bidi_move_to_visually_next(&mut it).expect("move");
        if it.ch == BIDI_EOB || it.charpos >= zv {
            break;
        }
        out.push((it.charpos, it.resolved_level));
    }
    out
}

#[test]
fn reorders_mixed_paragraphs() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        assert_eq!(eval(&mut lisp, TABLES), "nil");
        assert_eq!(
            visual_order(&mut lisp, "ab \u{5d3}\u{5d4}\u{5d5} 12"),
            vec![
                (1, 0),
                (2, 0),
                (3, 0),
                (8, 2),
                (9, 2),
                (7, 1),
                (6, 1),
                (5, 1),
                (4, 1)
            ]
        );
        assert_eq!(
            visual_order(&mut lisp, "\u{5d3}\u{5d4}\u{5d5} ab"),
            vec![(1, 1), (2, 1), (3, 1), (4, 1), (6, 2), (5, 2)]
        );
        assert_eq!(
            visual_order(&mut lisp, "a\u{202e}bc\u{202c}d"),
            vec![(1, 0), (5, 1), (4, 1), (3, 1), (2, 1), (6, 0)]
        );
    });
}
