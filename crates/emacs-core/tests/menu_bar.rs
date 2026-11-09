mod common;

use common::{eval, on_big_stack};
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;

fn items(lisp: &mut Lisp) -> Vec<String> {
    let v = lisp.menu_bar_items(Value::NIL).unwrap();
    let items = lisp.vector_items(v).to_vec();
    let mut out = Vec::new();
    for chunk in items.chunks(4) {
        if chunk[0].is_nil() {
            break;
        }
        let parts: Vec<String> = chunk.iter().map(|&x| lisp.prin1_to_string(x)).collect();
        out.push(parts.join(" "));
    }
    out
}

#[test]
fn menu_bar_items_come_from_the_active_maps() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        eval(
            &mut lisp,
            "(progn (fset 'keymap-canonicalize (function (lambda (m) m))) (setq m (make-sparse-keymap)) (define-key m [menu-bar file] (cons \"File\" (make-sparse-keymap \"File\"))) (define-key m [menu-bar edit] '(menu-item \"Edit\" ignore :enable t)) (define-key m [menu-bar hidden] '(menu-item \"Hidden\" ignore :visible nil)) (define-key m [menu-bar off] '(menu-item \"Off\" ignore :enable nil)) (use-global-map m))",
        );
        assert_eq!(
            items(&mut lisp),
            vec![
                "edit \"Edit\" (ignore) 0".to_string(),
                "file \"File\" ((keymap \"File\")) 0".to_string()
            ]
        );
        eval(&mut lisp, "(setq menu-bar-final-items '(edit))");
        assert_eq!(
            items(&mut lisp),
            vec![
                "file \"File\" ((keymap \"File\")) 0".to_string(),
                "edit \"Edit\" (ignore) 0".to_string()
            ]
        );
    });
}

#[test]
fn a_local_map_menu_merges_with_the_global_one() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        eval(
            &mut lisp,
            "(progn (fset 'keymap-canonicalize (function (lambda (m) m))) (setq g (make-sparse-keymap)) (define-key g [menu-bar tools] (cons \"Tools\" (make-sparse-keymap \"Global tools\"))) (use-global-map g) (setq l (make-sparse-keymap)) (define-key l [menu-bar tools] (cons \"Tools\" (make-sparse-keymap \"Local tools\"))) (define-key l [menu-bar mode] (cons \"Mode\" (make-sparse-keymap \"Mode\"))) (use-local-map l))",
        );
        assert_eq!(
            items(&mut lisp),
            vec![
                "tools \"Tools\" ((keymap \"Local tools\") (keymap \"Global tools\")) 0"
                    .to_string(),
                "mode \"Mode\" ((keymap \"Mode\")) 0".to_string()
            ]
        );
    });
}

#[test]
fn parse_menu_item_records_key_equivalents_and_buttons() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        eval(
            &mut lisp,
            "(progn (fset 'substitute-command-keys (function (lambda (s) s))) (setq item '(menu-item \"Save\" save-buffer :keys \"C-x C-s\" :button (:toggle . t) :help \"Save it\")))",
        );
        let symbol = lisp.intern("item");
        let item = lisp.find_symbol_value(symbol);
        assert!(lisp.parse_menu_item(item, 0).unwrap());
        let props = lisp.keyboard.menu.item_properties;
        let printed: Vec<String> = lisp
            .vector_items(props)
            .to_vec()
            .into_iter()
            .skip(1)
            .map(|v| lisp.prin1_to_string(v))
            .collect();
        assert_eq!(
            printed,
            [
                "\"Save\"",
                "save-buffer",
                "t",
                "nil",
                "\"Save it\"",
                "t",
                ":toggle",
                "\"  C-x C-s\""
            ]
        );
        eval(
            &mut lisp,
            "(progn (use-global-map (make-sparse-keymap)) (setq off '(menu-item \"Off\" ignore :enable nil)))",
        );
        let symbol = lisp.intern("off");
        let off = lisp.find_symbol_value(symbol);
        assert!(!lisp.parse_menu_item(off, 1).unwrap());
        assert!(lisp.parse_menu_item(off, 0).unwrap());
    });
}
