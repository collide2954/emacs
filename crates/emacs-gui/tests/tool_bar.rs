use std::sync::mpsc;

use emacs_core::dispextern::{
    TOOL_BAR_ITEM_BINDING, TOOL_BAR_ITEM_CAPTION, TOOL_BAR_ITEM_ENABLED_P, TOOL_BAR_ITEM_HELP,
    TOOL_BAR_ITEM_IMAGES, TOOL_BAR_ITEM_KEY, TOOL_BAR_ITEM_LABEL, TOOL_BAR_ITEM_NSLOTS,
    TOOL_BAR_ITEM_SELECTED_P, TOOL_BAR_ITEM_TYPE,
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
        lisp.prin1_to_string(self.slots[item * TOOL_BAR_ITEM_NSLOTS + slot])
    }
}

fn items(lisp: &mut Lisp) -> Items {
    let (vector, count) = lisp.tool_bar_items(Value::NIL).unwrap();
    let slots = lisp.vector_items(vector).to_vec();
    assert!(slots.len() >= count * TOOL_BAR_ITEM_NSLOTS);
    Items { slots, count }
}

#[test]
fn tool_bar_items_parse_the_tool_bar_keymaps() {
    on_big_stack(|| {
        let mut lisp = booted();
        assert_eq!(
            eval(
                &mut lisp,
                "(progn \
                   (use-global-map (make-sparse-keymap)) \
                   (fset 'tb-new (function (lambda () (interactive) nil))) \
                   (fset 'tb-save (function (lambda () (interactive) nil))) \
                   (define-key (current-global-map) \"\\C-cz\" 'tb-new) \
                   (define-key (current-global-map) [tool-bar] \
                     '(keymap \
                       (new menu-item \"Open file...\" tb-new :image (image :type xbm :file \"new.xbm\") :help \"Make a file\") \
                       (sep menu-item \"--\") \
                       (save menu-item \"Save\" tb-save :enable nil :button (:toggle . t) :label \"Keep\") \
                       (hidden menu-item \"Hidden\" tb-save :visible nil) \
                       (sub menu-item \"Sub\" (keymap)) \
                       (very-long-key menu-item \"A caption that is far too long\" tb-save) \
                       (extraordinarily-long menu-item \"Another caption that is too long\" tb-save))) \
                   t)"
            ),
            "t"
        );
        let all = items(&mut lisp);
        assert_eq!(all.count, 5);
        assert_eq!(all.get(&mut lisp, 0, TOOL_BAR_ITEM_KEY), "new");
        assert_eq!(
            all.get(&mut lisp, 0, TOOL_BAR_ITEM_CAPTION),
            "\"Open file...\""
        );
        assert_eq!(all.get(&mut lisp, 0, TOOL_BAR_ITEM_BINDING), "tb-new");
        assert_eq!(all.get(&mut lisp, 0, TOOL_BAR_ITEM_ENABLED_P), "t");
        assert_eq!(
            all.get(&mut lisp, 0, TOOL_BAR_ITEM_IMAGES),
            "(image :type xbm :file \"new.xbm\")"
        );
        assert_eq!(all.get(&mut lisp, 0, TOOL_BAR_ITEM_LABEL), "\"Open File\"");
        assert_eq!(
            all.get(&mut lisp, 0, TOOL_BAR_ITEM_HELP),
            "\"Make a file  (C-c z)\""
        );
        assert_eq!(all.get(&mut lisp, 1, TOOL_BAR_ITEM_KEY), "sep");
        assert_eq!(all.get(&mut lisp, 1, TOOL_BAR_ITEM_TYPE), "t");
        assert_eq!(all.get(&mut lisp, 1, TOOL_BAR_ITEM_BINDING), "nil");
        assert_eq!(all.get(&mut lisp, 2, TOOL_BAR_ITEM_KEY), "save");
        assert_eq!(all.get(&mut lisp, 2, TOOL_BAR_ITEM_ENABLED_P), "nil");
        assert_eq!(all.get(&mut lisp, 2, TOOL_BAR_ITEM_SELECTED_P), "t");
        assert_eq!(all.get(&mut lisp, 2, TOOL_BAR_ITEM_TYPE), ":toggle");
        assert_eq!(all.get(&mut lisp, 2, TOOL_BAR_ITEM_LABEL), "\"Keep\"");
        assert_eq!(all.get(&mut lisp, 2, TOOL_BAR_ITEM_HELP), "nil");
        assert_eq!(all.get(&mut lisp, 3, TOOL_BAR_ITEM_KEY), "very-long-key");
        assert_eq!(
            all.get(&mut lisp, 3, TOOL_BAR_ITEM_LABEL),
            "\"Very Long Key\""
        );
        assert_eq!(
            all.get(&mut lisp, 4, TOOL_BAR_ITEM_KEY),
            "extraordinarily-long"
        );
        assert_eq!(all.get(&mut lisp, 4, TOOL_BAR_ITEM_LABEL), "\"\"");
        eval(
            &mut lisp,
            "(progn \
               (use-local-map (make-sparse-keymap)) \
               (define-key (current-local-map) [tool-bar sep] 'undefined))",
        );
        let fewer = items(&mut lisp);
        assert_eq!(fewer.count, 4);
        assert_eq!(fewer.get(&mut lisp, 1, TOOL_BAR_ITEM_KEY), "save");
    });
}

#[test]
fn local_and_minor_mode_tool_bar_maps_come_before_the_global_map() {
    on_big_stack(|| {
        let mut lisp = booted();
        eval(
            &mut lisp,
            "(progn \
               (use-global-map (make-sparse-keymap)) \
               (define-key (current-global-map) [tool-bar global] '(menu-item \"Global\" ignore)) \
               (setq tb-local (make-sparse-keymap)) \
               (define-key tb-local [tool-bar local] '(menu-item \"Local\" ignore)) \
               (use-local-map tb-local) \
               (setq tb-minor (make-sparse-keymap)) \
               (define-key tb-minor [tool-bar minor] '(menu-item \"Minor\" ignore)) \
               (setq tb-mode t) \
               (setq minor-mode-map-alist (list (cons 'tb-mode tb-minor))))",
        );
        let all = items(&mut lisp);
        let keys: Vec<String> = (0..all.count)
            .map(|i| all.get(&mut lisp, i, TOOL_BAR_ITEM_KEY))
            .collect();
        assert_eq!(keys, ["global", "local", "minor"]);
    });
}
