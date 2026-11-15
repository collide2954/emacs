use std::sync::mpsc;

use emacs_core::dispextern::{
    TOOL_BAR_ITEM_BINDING, TOOL_BAR_ITEM_CAPTION, TOOL_BAR_ITEM_ENABLED_P, TOOL_BAR_ITEM_HELP,
    TOOL_BAR_ITEM_IMAGES, TOOL_BAR_ITEM_KEY, TOOL_BAR_ITEM_LABEL, TOOL_BAR_ITEM_NSLOTS,
    TOOL_BAR_ITEM_SELECTED_P, TOOL_BAR_ITEM_TYPE,
};
use emacs_core::emacs::Startup;
use emacs_core::keyboard::CTRL_MODIFIER;
use emacs_core::lisp::Lisp;
use emacs_core::termhooks::{EventKind, InputEvent};
use emacs_core::value::Value;
use emacs_gui::display::Display;
use emacs_gui::protocol::{Notice, ToolBar, ToolBarImage, ToolBarPosition, ToolItem};
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

const BAR: &str = "(progn \
   (use-global-map (make-sparse-keymap)) \
   (define-key (current-global-map) [tool-bar] \
     '(keymap \
       (red menu-item \"Red\" ignore :image (image :type pbm :data \"P3 1 1 255 255 0 0\")) \
       (sep menu-item \"--\") \
       (off menu-item \"Off\" ignore :enable nil :vert-only t \
         :image (image :type pbm :data \"P3 1 1 255 0 255 255\")) \
       (vec menu-item \"Vec\" ignore \
         :image [(image :type pbm :data \"P3 1 1 255 0 255 0\") \
                 (image :type pbm :data \"P3 1 1 255 0 0 255\") \
                 (image :type pbm :data \"P3 1 1 255 255 255 0\") \
                 (image :type pbm :data \"P3 1 1 255 0 255 255\")]) \
       (bad menu-item \"Bad\" ignore :image (image :type nosuch)))))";

fn gui_frame(lisp: &mut Lisp) -> Value {
    assert_eq!(
        eval(
            lisp,
            "(progn (set-char-table-range char-script-table '(0 . #x24F) 'latin) (set-char-table-extra-slot char-script-table 0 '(latin)) (fset 'window--pixel-to-total (function (lambda (&rest _) nil))) (fset 'record-window-buffer (function (lambda (w) nil))) (setq x-resource-name \"emacs\") (x-open-connection nil) (setq f (x-create-frame '((name . \"g\") (width . 30) (height . 8)))) (select-frame f) (framep f))"
        ),
        "pgtk"
    );
    let symbol = lisp.intern("f");
    let f = lisp.find_symbol_value(symbol);
    lisp.frame_mut(f).flags.visible = true;
    f
}

fn shown_tool_bar(lisp: &Lisp, f: Value) -> Option<ToolBar> {
    let id = lisp.frame(f).id as u64;
    emacs_gui::with_display(|d| d.tool_bar(id).cloned()).expect("display installed")
}

fn pixel(r: u8, g: u8, b: u8) -> Option<ToolBarImage> {
    Some(ToolBarImage {
        width: 1,
        height: 1,
        rgba: vec![r, g, b, 255],
    })
}

#[test]
fn tool_bar_lines_show_the_tool_bar_items_on_an_external_tool_bar() {
    on_big_stack(|| {
        let mut lisp = booted();
        eval(&mut lisp, BAR);
        let f = gui_frame(&mut lisp);
        assert!(lisp.frame(f).flags.external_tool_bar);
        lisp.redisplay().unwrap();
        assert_eq!(lisp.frame(f).n_tool_bar_items, 5);
        let bar = shown_tool_bar(&lisp, f).expect("tool bar shown");
        assert!(!bar.horiz);
        assert_eq!(bar.position, Some(ToolBarPosition::Top));
        assert_eq!(
            bar.items,
            [
                ToolItem::Button {
                    index: 0,
                    image: pixel(255, 0, 0),
                    label: None,
                    enabled: true,
                },
                ToolItem::Separator,
                ToolItem::Button {
                    index: 2,
                    image: pixel(0, 255, 255),
                    label: None,
                    enabled: false,
                },
            ]
        );
        eval(
            &mut lisp,
            "(progn (setq tool-bar-style 'both-horiz) (force-mode-line-update t))",
        );
        lisp.redisplay().unwrap();
        let bar = shown_tool_bar(&lisp, f).expect("tool bar shown");
        assert!(bar.horiz);
        assert!(!bar.text_image);
        assert_eq!(
            bar.items[0],
            ToolItem::Button {
                index: 0,
                image: pixel(255, 0, 0),
                label: Some("Red".into()),
                enabled: true,
            }
        );
        assert_eq!(
            bar.items[2],
            ToolItem::Button {
                index: 2,
                image: pixel(0, 255, 255),
                label: None,
                enabled: false,
            }
        );
        eval(
            &mut lisp,
            "(progn (setq tool-bar-style 'text) (force-mode-line-update t))",
        );
        lisp.redisplay().unwrap();
        let bar = shown_tool_bar(&lisp, f).expect("tool bar shown");
        assert_eq!(
            bar.items[2],
            ToolItem::Button {
                index: 2,
                image: None,
                label: Some("Off".into()),
                enabled: false,
            }
        );
        eval(
            &mut lisp,
            "(modify-frame-parameters f '((tool-bar-lines . 0)))",
        );
        assert!(!lisp.frame(f).flags.external_tool_bar);
        assert_eq!(shown_tool_bar(&lisp, f), None);
    });
}

#[test]
fn tool_bar_get_system_style_falls_back_to_image() {
    on_big_stack(|| {
        let mut lisp = booted();
        assert_eq!(eval(&mut lisp, "(tool-bar-get-system-style)"), "image");
        assert_eq!(
            eval(
                &mut lisp,
                "(let ((tool-bar-style 'text-image-horiz)) (tool-bar-get-system-style))"
            ),
            "text-image-horiz"
        );
        assert_eq!(
            eval(
                &mut lisp,
                "(let ((tool-bar-style 'bogus)) (tool-bar-get-system-style))"
            ),
            "image"
        );
    });
}

#[test]
fn implied_resizes_are_inhibited_only_once_the_tool_bar_was_resized() {
    on_big_stack(|| {
        let mut lisp = booted();
        let f = gui_frame(&mut lisp);
        let id = lisp.frame(f).id as u64;
        let surface_width =
            || emacs_gui::with_display(|d| d.surface(id).map(|s| s.width())).flatten();
        eval(
            &mut lisp,
            "(progn (frame-after-make-frame f t) (setq frame-inhibit-implied-resize t))",
        );
        assert!(!lisp.frame(f).flags.tool_bar_resized);
        let before = surface_width();
        eval(
            &mut lisp,
            "(modify-frame-parameters f '((internal-border-width . 10)))",
        );
        assert_ne!(surface_width(), before);
        lisp.redisplay().unwrap();
        assert!(lisp.frame(f).flags.tool_bar_resized);
        let before = surface_width();
        eval(
            &mut lisp,
            "(modify-frame-parameters f '((internal-border-width . 20)))",
        );
        assert_eq!(surface_width(), before);
    });
}

#[test]
fn tool_bar_notices_become_tool_bar_and_help_events() {
    on_big_stack(|| {
        let (tx, notices) = mpsc::channel();
        let mut lisp = Lisp::boot(Startup::program(false));
        emacs_gui::install(&mut lisp, Display::new(fmpsc::unbounded().0, notices));
        lisp.init_display();
        eval(&mut lisp, BAR);
        let f = gui_frame(&mut lisp);
        lisp.redisplay().unwrap();
        let id = lisp.frame(f).id as u64;
        tx.send(Notice::Input(InputEvent {
            kind: EventKind::ToolBar,
            code: 2,
            modifiers: CTRL_MODIFIER,
            frame: id,
            ..InputEvent::default()
        }))
        .unwrap();
        tx.send(Notice::ToolBarHelp {
            frame: id,
            index: Some(0),
        })
        .unwrap();
        tx.send(Notice::ToolBarHelp {
            frame: id,
            index: None,
        })
        .unwrap();
        let frame = lisp.prin1_to_string(f);
        let mut events = Vec::new();
        for _ in 0..3 {
            let (event, _, _, _) = lisp.kbd_buffer_get_event(None, None).unwrap();
            events.push(lisp.prin1_to_string(event));
        }
        assert_eq!(
            events,
            [
                format!("(C-off ({frame} tool-bar))"),
                format!("(help-echo {frame} \"Red\" nil nil 0)"),
                format!("(help-echo {frame} nil nil nil 0)"),
            ]
        );
    });
}
