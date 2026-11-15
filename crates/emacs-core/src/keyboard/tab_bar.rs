use crate::dispextern::{
    TAB_BAR_ITEM_BINDING, TAB_BAR_ITEM_CAPTION, TAB_BAR_ITEM_ENABLED_P, TAB_BAR_ITEM_HELP,
    TAB_BAR_ITEM_KEY, TAB_BAR_ITEM_NSLOTS, TAB_BAR_ITEM_SELECTED_P,
};
use crate::error::LispResult;
use crate::keyboard::menu::menu_separator_name_p;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

impl Lisp {
    pub fn tab_bar_items(&mut self, reuse: Value) -> LispResult<(Value, usize)> {
        let oquit = self.find_symbol_value(sym::INHIBIT_QUIT);
        self.set_plain_value(sym::INHIBIT_QUIT, sym::T);
        let r = self.tab_bar_items_1(reuse);
        self.set_plain_value(sym::INHIBIT_QUIT, oquit);
        r
    }

    fn tab_bar_items_1(&mut self, reuse: Value) -> LispResult<(Value, usize)> {
        self.init_tab_bar_items(reuse);
        let maps = self.bar_keymaps()?;
        let base = self.stack.len();
        self.stack.extend_from_slice(&maps);
        let r = (|| {
            for &map in maps.iter().rev() {
                if map.is_nil() {
                    continue;
                }
                let binding = self.access_keymap(map, sym::TAB_BAR, true, false)?;
                let keymap = self.get_keymap(binding, false, true)?;
                if keymap.is_cons() {
                    self.map_keymap(
                        keymap,
                        &mut |l, key, def| l.process_tab_bar_item(key, def),
                        true,
                    )?;
                }
            }
            Ok(())
        })();
        self.stack.truncate(base);
        r?;
        let n = self.keyboard.menu.ntab_bar_items / TAB_BAR_ITEM_NSLOTS;
        Ok((self.keyboard.menu.tab_bar_items_vector, n))
    }

    fn tab_bar_slot(&self, i: usize) -> Value {
        self.vector_items(self.keyboard.menu.tab_bar_items_vector)[i]
    }

    fn set_tab_bar_slot(&mut self, i: usize, v: Value) {
        let vector = self.keyboard.menu.tab_bar_items_vector;
        self.vector_set(vector, i, v);
    }

    fn process_tab_bar_item(&mut self, key: Value, def: Value) -> LispResult<()> {
        if def == sym::UNDEFINED {
            let n = self.keyboard.menu.ntab_bar_items;
            let mut i = 0;
            while i < n {
                if key == self.tab_bar_slot(i + TAB_BAR_ITEM_KEY) {
                    if n > i + TAB_BAR_ITEM_NSLOTS {
                        for j in i..n - TAB_BAR_ITEM_NSLOTS {
                            let v = self.tab_bar_slot(j + TAB_BAR_ITEM_NSLOTS);
                            self.set_tab_bar_slot(j, v);
                        }
                    }
                    self.keyboard.menu.ntab_bar_items -= TAB_BAR_ITEM_NSLOTS;
                    break;
                }
                i += TAB_BAR_ITEM_NSLOTS;
            }
        } else if self.parse_tab_bar_item(key, def)? {
            self.append_tab_bar_item();
        }
        Ok(())
    }

    fn tab_prop(&self, idx: usize) -> Value {
        self.vector_items(self.keyboard.menu.tab_bar_item_properties)[idx]
    }

    fn set_prop_tab_bar(&mut self, idx: usize, val: Value) {
        let props = self.keyboard.menu.tab_bar_item_properties;
        self.vector_set(props, idx, val);
    }

    fn parse_tab_bar_item(&mut self, key: Value, item: Value) -> LispResult<bool> {
        let mut filter = Value::NIL;
        if !item.is_cons() {
            return Ok(false);
        }
        let mut item = item;
        let head = self.xcar(item);
        if head.is_string() {
            item = self.list(&[head]);
        } else {
            if head != sym::MENU_ITEM {
                return Ok(false);
            }
            item = self.xcdr(item);
            if !item.is_cons() {
                return Ok(false);
            }
        }
        if self.is_vector(self.keyboard.menu.tab_bar_item_properties) {
            for i in 0..TAB_BAR_ITEM_NSLOTS {
                self.set_prop_tab_bar(i, Value::NIL);
            }
        } else {
            self.keyboard.menu.tab_bar_item_properties =
                self.vector(vec![Value::NIL; TAB_BAR_ITEM_NSLOTS]);
        }
        self.set_prop_tab_bar(TAB_BAR_ITEM_KEY, key);
        self.set_prop_tab_bar(TAB_BAR_ITEM_ENABLED_P, sym::T);
        let mut caption = self.xcar(item);
        if !caption.is_string() {
            caption = self.menu_item_eval_property(caption)?;
            if !caption.is_string() {
                return Ok(false);
            }
        }
        self.set_prop_tab_bar(TAB_BAR_ITEM_CAPTION, caption);
        item = self.xcdr(item);
        if !item.is_cons() {
            if menu_separator_name_p(self.string_bytes(caption)) {
                self.set_prop_tab_bar(TAB_BAR_ITEM_ENABLED_P, Value::NIL);
                self.set_prop_tab_bar(TAB_BAR_ITEM_SELECTED_P, Value::NIL);
                self.set_prop_tab_bar(TAB_BAR_ITEM_CAPTION, Value::NIL);
                return Ok(true);
            }
            return Ok(false);
        }
        let binding = self.xcar(item);
        self.set_prop_tab_bar(TAB_BAR_ITEM_BINDING, binding);
        item = self.xcdr(item);
        if item.is_cons() && self.xcar(item).is_cons() {
            item = self.xcdr(item);
        }
        while item.is_cons() {
            let ikey = self.xcar(item);
            item = self.xcdr(item);
            if !item.is_cons() {
                break;
            }
            let value = self.xcar(item);
            if ikey == sym::K_ENABLE {
                let enable_disabled = !self
                    .find_symbol_value(sym::ENABLE_DISABLED_MENUS_AND_BUTTONS)
                    .is_nil();
                self.set_prop_tab_bar(
                    TAB_BAR_ITEM_ENABLED_P,
                    if enable_disabled { sym::T } else { value },
                );
            } else if ikey == sym::K_VISIBLE {
                if self.menu_item_eval_property(value)?.is_nil() {
                    return Ok(false);
                }
            } else if ikey == sym::K_HELP {
                self.set_prop_tab_bar(TAB_BAR_ITEM_HELP, value);
            } else if ikey == sym::K_FILTER {
                filter = value;
            } else if ikey == sym::K_BUTTON && value.is_cons() {
                let (kind, selected) = (self.xcar(value), self.xcdr(value));
                if kind == sym::K_TOGGLE || kind == sym::K_RADIO {
                    self.set_prop_tab_bar(TAB_BAR_ITEM_SELECTED_P, selected);
                }
            }
            item = self.xcdr(item);
        }
        if !filter.is_nil() {
            let binding = self.tab_prop(TAB_BAR_ITEM_BINDING);
            let quoted = self.list(&[sym::QUOTE, binding]);
            let form = self.list(&[filter, quoted]);
            let filtered = self.menu_item_eval_property(form)?;
            self.set_prop_tab_bar(TAB_BAR_ITEM_BINDING, filtered);
        }
        let binding = self.tab_prop(TAB_BAR_ITEM_BINDING);
        if self.get_keymap(binding, false, true)?.is_cons() {
            return Ok(false);
        }
        let enabled = self.tab_prop(TAB_BAR_ITEM_ENABLED_P);
        if enabled != sym::T {
            let enabled = self.menu_item_eval_property(enabled)?;
            self.set_prop_tab_bar(TAB_BAR_ITEM_ENABLED_P, enabled);
        }
        let selected = self.tab_prop(TAB_BAR_ITEM_SELECTED_P);
        if !selected.is_nil() {
            let selected = self.menu_item_eval_property(selected)?;
            self.set_prop_tab_bar(TAB_BAR_ITEM_SELECTED_P, selected);
        }
        Ok(true)
    }

    fn init_tab_bar_items(&mut self, reuse: Value) {
        self.keyboard.menu.tab_bar_items_vector = if self.is_vector(reuse) {
            reuse
        } else {
            self.vector(vec![Value::NIL; 64])
        };
        self.keyboard.menu.ntab_bar_items = 0;
    }

    fn append_tab_bar_item(&mut self) {
        let n = self.keyboard.menu.ntab_bar_items;
        let size = self
            .vector_items(self.keyboard.menu.tab_bar_items_vector)
            .len();
        let incr = n as i64 - (size as i64 - TAB_BAR_ITEM_NSLOTS as i64);
        if incr > 0 {
            let vector = self.keyboard.menu.tab_bar_items_vector;
            self.keyboard.menu.tab_bar_items_vector =
                crate::fns::larger_vector(self, vector, incr as usize);
        }
        for i in 0..TAB_BAR_ITEM_NSLOTS {
            let v = self.tab_prop(i);
            self.set_tab_bar_slot(n + i, v);
        }
        self.keyboard.menu.ntab_bar_items += TAB_BAR_ITEM_NSLOTS;
    }
}
