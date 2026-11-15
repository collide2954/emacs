use std::sync::mpsc;

use emacs_core::dispextern::{
    TAB_BAR_ITEM_BINDING, TAB_BAR_ITEM_CAPTION, TAB_BAR_ITEM_ENABLED_P, TAB_BAR_ITEM_HELP,
    TAB_BAR_ITEM_KEY, TAB_BAR_ITEM_NSLOTS, TAB_BAR_ITEM_SELECTED_P,
};
use emacs_core::emacs::Startup;
use emacs_core::lisp::Lisp;
use emacs_core::value::Value;
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
