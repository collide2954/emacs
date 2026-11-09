use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const ITEM_PROPERTY_ITEM: usize = 0;
pub const ITEM_PROPERTY_NAME: usize = 1;
pub const ITEM_PROPERTY_DEF: usize = 2;
pub const ITEM_PROPERTY_ENABLE: usize = 3;
pub const ITEM_PROPERTY_MAP: usize = 4;
pub const ITEM_PROPERTY_HELP: usize = 5;
pub const ITEM_PROPERTY_SELECTED: usize = 6;
pub const ITEM_PROPERTY_TYPE: usize = 7;
pub const ITEM_PROPERTY_KEYEQ: usize = 8;
pub const ITEM_PROPERTY_MAX: usize = 8;

pub struct MenuState {
    pub menu_bar_items_vector: Value,
    pub menu_bar_items_index: usize,
    pub menu_bar_one_keymap_changed_items: Value,
    pub item_properties: Value,
}

impl Default for MenuState {
    fn default() -> Self {
        MenuState {
            menu_bar_items_vector: Value::NIL,
            menu_bar_items_index: 0,
            menu_bar_one_keymap_changed_items: Value::NIL,
            item_properties: Value::NIL,
        }
    }
}

impl MenuState {
    pub fn roots(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.menu_bar_items_vector,
            self.menu_bar_one_keymap_changed_items,
            self.item_properties,
        ]);
    }
}

impl Lisp {
    fn item_property(&self, i: usize) -> Value {
        self.vector_items(self.keyboard.menu.item_properties)[i]
    }

    fn set_item_property(&mut self, i: usize, v: Value) {
        let props = self.keyboard.menu.item_properties;
        self.vector_set(props, i, v);
    }

    fn menu_bar_slot(&self, i: usize) -> Value {
        self.vector_items(self.keyboard.menu.menu_bar_items_vector)[i]
    }

    fn set_menu_bar_slot(&mut self, i: usize, v: Value) {
        let vector = self.keyboard.menu.menu_bar_items_vector;
        self.vector_set(vector, i, v);
    }

    fn grow_menu_bar_items(&mut self) {
        let vector = self.keyboard.menu.menu_bar_items_vector;
        let mut items = self.vector_items(vector).to_vec();
        let new_len = (items.len() * 2).max(items.len() + 4);
        items.resize(new_len, Value::NIL);
        self.keyboard.menu.menu_bar_items_vector = self.vector(items);
    }

    fn shift_menu_bar_items_down(&mut self, i: usize, end: usize) {
        for j in i..end - 4 {
            let v = self.menu_bar_slot(j + 4);
            self.set_menu_bar_slot(j, v);
        }
    }

    pub fn help_echo_substitute_command_keys(&mut self, help: Value) -> LispResult {
        if help.is_string() && self.string_chars(help) > 0 {
            let inhibit = crate::textprop::f_get_text_property(
                self,
                &[Value::fixnum(0), sym::HELP_ECHO_INHIBIT_SUBSTITUTION, help],
            )?;
            if !inhibit.is_nil() {
                return Ok(help);
            }
        }
        self.funcall(sym::SUBSTITUTE_COMMAND_KEYS, &[help])
    }

    pub fn menu_bar_items(&mut self, old: Value) -> LispResult {
        let oquit = self.find_symbol_value(sym::INHIBIT_QUIT);
        self.set_plain_value(sym::INHIBIT_QUIT, sym::T);
        let r = self.menu_bar_items_1(old);
        self.set_plain_value(sym::INHIBIT_QUIT, oquit);
        r
    }

    fn menu_bar_items_1(&mut self, old: Value) -> LispResult {
        self.keyboard.menu.menu_bar_items_vector = if old.is_nil() {
            self.vector(vec![Value::NIL; 24])
        } else {
            old
        };
        self.keyboard.menu.menu_bar_items_index = 0;
        let mut maps = Vec::new();
        let overriding_terminal = self.kvar(crate::keyboard::Kvar::OverridingTerminalLocalMap);
        let flag = self.find_symbol_value(sym::OVERRIDING_LOCAL_MAP_MENU_FLAG);
        let overriding = self.find_symbol_value(sym::OVERRIDING_LOCAL_MAP);
        if !flag.is_nil() && !overriding.is_nil() {
            if !overriding_terminal.is_nil() {
                maps.push(overriding_terminal);
            }
            if !overriding.is_nil() {
                maps.push(overriding);
            }
        } else {
            let minor: Vec<Value> = self
                .current_minor_maps()?
                .into_iter()
                .map(|(_, m)| m)
                .collect();
            if !overriding_terminal.is_nil() && !flag.is_nil() {
                maps.push(overriding_terminal);
            }
            let (pt, cb) = (self.cur().pt, self.current_buffer);
            let tem = self.get_local_map(pt, cb, sym::KEYMAP)?;
            if !tem.is_nil() {
                maps.push(tem);
            }
            maps.extend(minor);
            let local = self.get_local_map(pt, cb, sym::LOCAL_MAP)?;
            maps.push(local);
        }
        maps.push(self.current_global_map);
        let base = self.stack.len();
        self.stack.extend_from_slice(&maps);
        for &map in maps.iter().rev() {
            if map.is_nil() {
                continue;
            }
            let binding = self.access_keymap(map, sym::MENU_BAR, true, false)?;
            let def = self.get_keymap(binding, false, true)?;
            if def.is_cons() {
                self.keyboard.menu.menu_bar_one_keymap_changed_items = Value::NIL;
                self.map_keymap_canonical(def, &mut |l, key, item| l.menu_bar_item(key, item))?;
            }
        }
        self.stack.truncate(base);
        let mut tail = self.find_symbol_value(sym::MENU_BAR_FINAL_ITEMS);
        while tail.is_cons() {
            let end = self.keyboard.menu.menu_bar_items_index;
            let target = self.xcar(tail);
            let mut i = 0;
            while i < end {
                if target == self.menu_bar_slot(i) {
                    let saved: Vec<Value> = (0..4).map(|k| self.menu_bar_slot(i + k)).collect();
                    if end > i + 4 {
                        self.shift_menu_bar_items_down(i, end);
                    }
                    for (k, v) in saved.into_iter().enumerate() {
                        self.set_menu_bar_slot(end - 4 + k, v);
                    }
                    break;
                }
                i += 4;
            }
            tail = self.xcdr(tail);
        }
        let i = self.keyboard.menu.menu_bar_items_index;
        if i + 4
            > self
                .vector_items(self.keyboard.menu.menu_bar_items_vector)
                .len()
        {
            self.grow_menu_bar_items();
        }
        for k in 0..4 {
            self.set_menu_bar_slot(i + k, Value::NIL);
        }
        self.keyboard.menu.menu_bar_items_index = i + 4;
        Ok(self.keyboard.menu.menu_bar_items_vector)
    }

    fn menu_bar_item(&mut self, key: Value, item: Value) -> LispResult<()> {
        if item == sym::UNDEFINED {
            let mut i = 0;
            while i < self.keyboard.menu.menu_bar_items_index {
                if key == self.menu_bar_slot(i) {
                    let index = self.keyboard.menu.menu_bar_items_index;
                    if index > i + 4 {
                        self.shift_menu_bar_items_down(i, index);
                    }
                    self.keyboard.menu.menu_bar_items_index -= 4;
                }
                i += 4;
            }
        }
        let changed = self.keyboard.menu.menu_bar_one_keymap_changed_items;
        if !self.memq(key, changed).is_nil() || item.is_nil() {
            return Ok(());
        }
        self.keyboard.menu.menu_bar_one_keymap_changed_items = self.cons(key, changed);
        if !self.parse_menu_item(item, 1)? {
            return Ok(());
        }
        let mut item = self.item_property(ITEM_PROPERTY_DEF);
        let mut i = 0;
        while i < self.keyboard.menu.menu_bar_items_index {
            if key == self.menu_bar_slot(i) {
                break;
            }
            i += 4;
        }
        if i == self.keyboard.menu.menu_bar_items_index {
            if i + 4
                > self
                    .vector_items(self.keyboard.menu.menu_bar_items_vector)
                    .len()
            {
                self.grow_menu_bar_items();
            }
            let name = self.item_property(ITEM_PROPERTY_NAME);
            let maps = self.list(&[item]);
            self.set_menu_bar_slot(i, key);
            self.set_menu_bar_slot(i + 1, name);
            self.set_menu_bar_slot(i + 2, maps);
            self.set_menu_bar_slot(i + 3, Value::fixnum(0));
            self.keyboard.menu.menu_bar_items_index = i + 4;
        } else {
            let old = self.menu_bar_slot(i + 2);
            let both_keymaps = self.keymapp(item) && self.keymapp(self.xcar(old));
            item = self.cons(item, if both_keymaps { old } else { Value::NIL });
            self.set_menu_bar_slot(i + 2, item);
        }
        Ok(())
    }

    pub fn parse_menu_item(&mut self, item: Value, inmenubar: i32) -> LispResult<bool> {
        let mut filter = Value::NIL;
        let mut keyhint = Value::NIL;
        if !item.is_cons() {
            return Ok(false);
        }
        if self.keyboard.menu.item_properties.is_nil() {
            self.keyboard.menu.item_properties =
                self.vector(vec![Value::NIL; ITEM_PROPERTY_MAX + 1]);
        }
        for i in ITEM_PROPERTY_DEF..=ITEM_PROPERTY_MAX {
            self.set_item_property(i, Value::NIL);
        }
        self.set_item_property(ITEM_PROPERTY_ENABLE, sym::T);
        self.set_item_property(ITEM_PROPERTY_ITEM, item);
        let item_string = self.xcar(item);
        let mut item = self.xcdr(item);
        let enable_disabled = !self
            .find_symbol_value(sym::ENABLE_DISABLED_MENUS_AND_BUTTONS)
            .is_nil();
        if item_string.is_string() {
            self.set_item_property(ITEM_PROPERTY_NAME, item_string);
            if item.is_cons() && self.xcar(item).is_string() {
                let help = self.xcar(item);
                let help = self.help_echo_substitute_command_keys(help)?;
                self.set_item_property(ITEM_PROPERTY_HELP, help);
                item = self.xcdr(item);
            }
            if item.is_cons() && self.xcar(item).is_cons() {
                let head = self.xcar(self.xcar(item));
                if head.is_nil() || self.is_vector(head) {
                    item = self.xcdr(item);
                }
            }
            self.set_item_property(ITEM_PROPERTY_DEF, item);
            if item.is_symbol() {
                let tem = self.get(item, sym::MENU_ENABLE);
                if enable_disabled {
                    self.set_item_property(ITEM_PROPERTY_ENABLE, sym::T);
                } else if !tem.is_nil() {
                    self.set_item_property(ITEM_PROPERTY_ENABLE, tem);
                }
            }
        } else if item_string == sym::MENU_ITEM && item.is_cons() {
            let name = self.xcar(item);
            self.set_item_property(ITEM_PROPERTY_NAME, name);
            let start = self.xcdr(item);
            if start.is_cons() {
                let def = self.xcar(start);
                self.set_item_property(ITEM_PROPERTY_DEF, def);
                item = self.xcdr(start);
                if item.is_cons() && self.xcar(item).is_cons() {
                    item = self.xcdr(item);
                }
                let mut tail = crate::lisp::ForEachTail::unrooted(item);
                while tail.tail.is_cons() {
                    let tem = self.xcar(tail.tail);
                    let rest = self.xcdr(tail.tail);
                    if !rest.is_cons() {
                        break;
                    }
                    let value = self.xcar(rest);
                    if tem == sym::K_ENABLE {
                        self.set_item_property(
                            ITEM_PROPERTY_ENABLE,
                            if enable_disabled { sym::T } else { value },
                        );
                    } else if tem == sym::K_VISIBLE {
                        if self.menu_item_eval_property(value)?.is_nil() {
                            return Ok(false);
                        }
                    } else if tem == sym::K_HELP {
                        let help = if value.is_string() {
                            self.help_echo_substitute_command_keys(value)?
                        } else {
                            value
                        };
                        self.set_item_property(ITEM_PROPERTY_HELP, help);
                    } else if tem == sym::K_FILTER {
                        filter = rest;
                    } else if tem == sym::K_KEY_SEQUENCE {
                        if value.is_symbol() || value.is_string() || self.is_vector(value) {
                            keyhint = rest;
                        }
                    } else if tem == sym::K_KEYS {
                        if self.functionp(value) {
                            let keys = self.funcall(value, &[])?;
                            self.set_item_property(ITEM_PROPERTY_KEYEQ, keys);
                        } else if value.is_cons() || value.is_string() {
                            self.set_item_property(ITEM_PROPERTY_KEYEQ, value);
                        }
                    } else if tem == sym::K_BUTTON && value.is_cons() {
                        let kind = self.xcar(value);
                        if kind == sym::K_TOGGLE || kind == sym::K_RADIO {
                            let selected = self.xcdr(value);
                            self.set_item_property(ITEM_PROPERTY_SELECTED, selected);
                            self.set_item_property(ITEM_PROPERTY_TYPE, kind);
                        }
                    }
                    tail.tail = rest;
                    tail.step_safe(self);
                }
            } else if inmenubar != 0 || !start.is_nil() {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
        let mut item_string = self.item_property(ITEM_PROPERTY_NAME);
        if !item_string.is_string() {
            item_string = self.menu_item_eval_property(item_string)?;
            if !item_string.is_string() {
                return Ok(false);
            }
            self.set_item_property(ITEM_PROPERTY_NAME, item_string);
        }
        let mut def = self.item_property(ITEM_PROPERTY_DEF);
        if !filter.is_nil() {
            let quoted = self.list(&[sym::QUOTE, def]);
            let form = self.list(&[self.xcar(filter), quoted]);
            def = self.menu_item_eval_property(form)?;
            self.set_item_property(ITEM_PROPERTY_DEF, def);
        }
        let tem = self.item_property(ITEM_PROPERTY_ENABLE);
        if tem != sym::T {
            let tem = self.menu_item_eval_property(tem)?;
            if inmenubar != 0 && tem.is_nil() {
                return Ok(false);
            }
            self.set_item_property(ITEM_PROPERTY_ENABLE, tem);
        }
        if def.is_nil() {
            return Ok(inmenubar == 0);
        }
        let def = self.item_property(ITEM_PROPERTY_DEF);
        let tem = self.get_keymap(def, false, true)?;
        if tem.is_cons() {
            self.set_item_property(ITEM_PROPERTY_MAP, tem);
            self.set_item_property(ITEM_PROPERTY_DEF, tem);
            return Ok(true);
        }
        if inmenubar > 0 {
            return Ok(true);
        }
        let mut keyeq = self.item_property(ITEM_PROPERTY_KEYEQ);
        let space_space = self.string("  ");
        if keyeq.is_string() && !keyhint.is_cons() {
            let keys = self.funcall(sym::SUBSTITUTE_COMMAND_KEYS, &[keyeq])?;
            keyeq = crate::fns::f_concat(self, &[space_space, keys])?;
        } else {
            let mut prefix = keyeq;
            let mut keys = Value::NIL;
            let def = if prefix.is_cons() {
                let d = self.xcar(prefix);
                prefix = self.xcdr(prefix);
                d
            } else {
                self.item_property(ITEM_PROPERTY_DEF)
            };
            if keyhint.is_cons() && !self.xcar(keyhint).is_nil() {
                keys = self.xcar(keyhint);
                let tem = crate::keymap::f_key_binding(
                    self,
                    &[keys, Value::NIL, Value::NIL, Value::NIL],
                )?;
                if tem.is_nil()
                    || (tem != def
                        && !(def.is_symbol() && tem == self.heap.symbol_ref(def).function))
                {
                    keys = Value::NIL;
                }
            }
            if keys.is_nil() {
                keys = crate::keymap::f_where_is_internal(
                    self,
                    &[def, Value::NIL, sym::T, Value::NIL, Value::NIL],
                )?;
            }
            keyeq = if !keys.is_nil() {
                let mut tem = crate::keymap::f_key_description(self, &[keys, Value::NIL])?;
                if prefix.is_cons() {
                    let (head, tail) = (self.xcar(prefix), self.xcdr(prefix));
                    if head.is_string() {
                        tem = crate::fns::f_concat(self, &[head, tem])?;
                    }
                    if tail.is_string() {
                        tem = crate::fns::f_concat(self, &[tem, tail])?;
                    }
                }
                crate::fns::f_concat(self, &[space_space, tem])?
            } else {
                Value::NIL
            };
        }
        self.set_item_property(ITEM_PROPERTY_KEYEQ, keyeq);
        let tem = self.item_property(ITEM_PROPERTY_SELECTED);
        if !tem.is_nil() {
            let v = self.menu_item_eval_property(tem)?;
            self.set_item_property(ITEM_PROPERTY_SELECTED, v);
        }
        Ok(true)
    }
}
