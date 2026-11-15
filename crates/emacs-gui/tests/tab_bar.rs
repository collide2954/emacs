use std::sync::mpsc;

use emacs_core::dispextern::{
    GlyphU, TAB_BAR_ITEM_BINDING, TAB_BAR_ITEM_CAPTION, TAB_BAR_ITEM_ENABLED_P, TAB_BAR_ITEM_HELP,
    TAB_BAR_ITEM_KEY, TAB_BAR_ITEM_NSLOTS, TAB_BAR_ITEM_SELECTED_P, TEXT_AREA,
};
use emacs_core::dispnew::matrix::WhichMatrix;
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
use emacs_core::xfaces::TAB_BAR_FACE_ID;
use emacs_gui::display::Display;
use iced::futures::channel::mpsc as fmpsc;

fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 << 20)
            .spawn_scoped(scope, f)
            .expect("spawn test thread")
            .join()
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    })
}

fn eval(lisp: &mut Lisp, source: &str) -> String {
    match lisp.eval_string(source) {
        Ok(value) => lisp.prin1_to_string(value),
        Err(flow) => {
            let condition = lisp.flow_condition(&flow);
            format!("error: {}", lisp.prin1_to_string(condition))
        }
    }
}

fn booted() -> Lisp {
    let mut lisp = Lisp::boot(Startup::program(false));
    emacs_gui::install(
        &mut lisp,
        Display::new(fmpsc::unbounded().0, mpsc::channel().1),
    );
    lisp.init_display();
    lisp
}

struct Items {
    slots: Vec<Value>,
    count: usize,
}

impl Items {
    fn get(&self, lisp: &mut Lisp, item: usize, slot: usize) -> String {
        lisp.prin1_to_string(self.slots[item * TAB_BAR_ITEM_NSLOTS + slot])
    }
}

fn items(lisp: &mut Lisp) -> Items {
    let (vector, count) = lisp.tab_bar_items(Value::NIL).unwrap();
    let slots = lisp.vector_items(vector).to_vec();
    assert!(slots.len() >= count * TAB_BAR_ITEM_NSLOTS);
    Items { slots, count }
}

#[test]
fn tab_bar_items_parse_the_tab_bar_keymaps() {
    on_big_stack(|| {
        let mut lisp = booted();
        assert_eq!(
            eval(
                &mut lisp,
                "(progn \
                   (use-global-map (make-sparse-keymap)) \
                   (fset 'tb-filter (function (lambda (b) 'forward-char))) \
                   (define-key (current-global-map) [tab-bar] \
                     '(keymap \
                       (current-tab menu-item \"*scratch*\" ignore :help \"Current tab\") \
                       (sep menu-item \"--\") \
                       (tab-2 menu-item \"other\" ignore :enable nil :button (:toggle . t)) \
                       (hidden menu-item \"h\" ignore :visible nil) \
                       (sub menu-item \"s\" (keymap)) \
                       (filtered menu-item \"f\" ignore :filter tb-filter))) \
                   t)"
            ),
            "t"
        );
        let all = items(&mut lisp);
        assert_eq!(all.count, 4);
        assert_eq!(all.get(&mut lisp, 0, TAB_BAR_ITEM_KEY), "current-tab");
        assert_eq!(all.get(&mut lisp, 0, TAB_BAR_ITEM_CAPTION), "\"*scratch*\"");
        assert_eq!(all.get(&mut lisp, 0, TAB_BAR_ITEM_BINDING), "ignore");
        assert_eq!(all.get(&mut lisp, 0, TAB_BAR_ITEM_ENABLED_P), "t");
        assert_eq!(all.get(&mut lisp, 0, TAB_BAR_ITEM_SELECTED_P), "nil");
        assert_eq!(all.get(&mut lisp, 0, TAB_BAR_ITEM_HELP), "\"Current tab\"");
        assert_eq!(all.get(&mut lisp, 1, TAB_BAR_ITEM_KEY), "sep");
        assert_eq!(all.get(&mut lisp, 1, TAB_BAR_ITEM_ENABLED_P), "nil");
        assert_eq!(all.get(&mut lisp, 1, TAB_BAR_ITEM_CAPTION), "nil");
        assert_eq!(all.get(&mut lisp, 2, TAB_BAR_ITEM_KEY), "tab-2");
        assert_eq!(all.get(&mut lisp, 2, TAB_BAR_ITEM_ENABLED_P), "nil");
        assert_eq!(all.get(&mut lisp, 2, TAB_BAR_ITEM_SELECTED_P), "t");
        assert_eq!(all.get(&mut lisp, 3, TAB_BAR_ITEM_KEY), "filtered");
        assert_eq!(all.get(&mut lisp, 3, TAB_BAR_ITEM_BINDING), "forward-char");
        eval(
            &mut lisp,
            "(progn \
               (use-local-map (make-sparse-keymap)) \
               (define-key (current-local-map) [tab-bar sep] 'undefined))",
        );
        let fewer = items(&mut lisp);
        assert_eq!(fewer.count, 3);
        assert_eq!(fewer.get(&mut lisp, 1, TAB_BAR_ITEM_KEY), "tab-2");
    });
}

fn gui_frame(lisp: &mut Lisp) -> Value {
    assert_eq!(
        eval(
            lisp,
            "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)) (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"g\") (width . 30) (height . 8) (tool-bar-lines . 0)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    f
}

fn tab_bar_row(lisp: &Lisp, f: Value, vpos: usize) -> (String, Vec<usize>) {
    let w = lisp.frame(f).tab_bar_window;
    let row = &lisp.window_matrix(w, WhichMatrix::Current).unwrap().rows[vpos];
    let glyphs = row.area_glyphs(TEXT_AREA);
    let text = glyphs
        .iter()
        .filter_map(|g| match g.u {
            GlyphU::Ch(c) => char::from_u32(c),
            _ => None,
        })
        .collect();
    (text, glyphs.iter().map(|g| g.face_id as usize).collect())
}

#[test]
fn a_tab_bar_shows_the_tab_captions_and_grows_to_fit_them() {
    on_big_stack(|| {
        let mut lisp = booted();
        eval(
            &mut lisp,
            "(progn (use-global-map (make-sparse-keymap)) \
               (define-key (current-global-map) [tab-bar] \
                 '(keymap (one menu-item \"One\" ignore) (two menu-item \" Two\" ignore))))",
        );
        let f = gui_frame(&mut lisp);
        let line_height = lisp.frame(f).geometry.line_height;
        emacs_gui::pgtkfns::pgtk_change_tab_bar_height(&mut lisp, f, line_height).unwrap();
        assert_eq!(lisp.frame(f).geometry.tab_bar_lines, 1);
        lisp.redisplay().unwrap();
        assert_eq!(lisp.frame(f).n_tab_bar_items, 2);
        let (text, faces) = tab_bar_row(&lisp, f, 0);
        assert!(text.starts_with("One Two"), "{text:?}");
        assert_eq!(faces[0], TAB_BAR_FACE_ID);
        assert_eq!(eval(&mut lisp, "(tab-bar-height f)"), "1");
        assert_eq!(
            eval(&mut lisp, "(tab-bar-height f t)"),
            line_height.to_string()
        );
        eval(
            &mut lisp,
            "(progn (define-key (current-global-map) [tab-bar] \
                      '(keymap (a menu-item \"Caption number one \" ignore) \
                               (b menu-item \"Caption number two \" ignore) \
                               (c menu-item \"Caption number three\" ignore))) \
                    (force-mode-line-update t))",
        );
        assert_eq!(eval(&mut lisp, "(tab-bar-height f)"), "2");
        lisp.redisplay().unwrap();
        lisp.redisplay().unwrap();
        assert_eq!(lisp.frame(f).geometry.tab_bar_lines, 2);
        let (first, _) = tab_bar_row(&lisp, f, 0);
        let (second, _) = tab_bar_row(&lisp, f, 1);
        assert!(first.starts_with("Caption number one"), "{first:?}");
        assert!(second.contains("three"), "{second:?}");
    });
}
