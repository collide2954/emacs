use crate::dispextern::{
    TOOL_BAR_ITEM_BINDING, TOOL_BAR_ITEM_CAPTION, TOOL_BAR_ITEM_ENABLED_P, TOOL_BAR_ITEM_HELP,
    TOOL_BAR_ITEM_IMAGES, TOOL_BAR_ITEM_KEY, TOOL_BAR_ITEM_LABEL, TOOL_BAR_ITEM_NSLOTS,
    TOOL_BAR_ITEM_RTL_IMAGE, TOOL_BAR_ITEM_SELECTED_P, TOOL_BAR_ITEM_TYPE, TOOL_BAR_ITEM_VERT_ONLY,
};
use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::keyboard::menu::menu_separator_name_p;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const BAD_LABEL: &[u8] = b"!!?GARBLED ITEM?!!";

impl Lisp {
    pub fn tool_bar_items(&mut self, reuse: Value) -> LispResult<(Value, usize)> {
        let oquit = self.find_symbol_value(sym::INHIBIT_QUIT);
        self.set_plain_value(sym::INHIBIT_QUIT, sym::T);
        let r = self.tool_bar_items_1(reuse);
        self.set_plain_value(sym::INHIBIT_QUIT, oquit);
        r
    }

    fn tool_bar_items_1(&mut self, reuse: Value) -> LispResult<(Value, usize)> {
        self.init_tool_bar_items(reuse);
        let mut maps = Vec::new();
        let overriding_terminal = self.kvar(Kvar::OverridingTerminalLocalMap);
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
        let r = (|| {
            for &map in maps.iter().rev() {
                if map.is_nil() {
                    continue;
                }
                let binding = self.access_keymap(map, sym::TOOL_BAR, true, false)?;
                let keymap = self.get_keymap(binding, false, true)?;
                if keymap.is_cons() {
                    self.map_keymap(
                        keymap,
                        &mut |l, key, def| l.process_tool_bar_item(key, def),
                        true,
                    )?;
                }
            }
            Ok(())
        })();
        self.stack.truncate(base);
        r?;
        let n = self.keyboard.menu.ntool_bar_items / TOOL_BAR_ITEM_NSLOTS;
        Ok((self.keyboard.menu.tool_bar_items_vector, n))
    }

    fn tool_bar_slot(&self, i: usize) -> Value {
        self.vector_items(self.keyboard.menu.tool_bar_items_vector)[i]
    }

    fn set_tool_bar_slot(&mut self, i: usize, v: Value) {
        let vector = self.keyboard.menu.tool_bar_items_vector;
        self.vector_set(vector, i, v);
    }

    fn process_tool_bar_item(&mut self, key: Value, def: Value) -> LispResult<()> {
        if def == sym::UNDEFINED {
            let n = self.keyboard.menu.ntool_bar_items;
            let mut i = 0;
            while i < n {
                if key == self.tool_bar_slot(i + TOOL_BAR_ITEM_KEY) {
                    for j in i..n - TOOL_BAR_ITEM_NSLOTS {
                        let v = self.tool_bar_slot(j + TOOL_BAR_ITEM_NSLOTS);
                        self.set_tool_bar_slot(j, v);
                    }
                    self.keyboard.menu.ntool_bar_items -= TOOL_BAR_ITEM_NSLOTS;
                    break;
                }
                i += TOOL_BAR_ITEM_NSLOTS;
            }
        } else if self.parse_tool_bar_item(key, def)? {
            self.append_tool_bar_item();
        }
        Ok(())
    }

    fn prop(&self, idx: usize) -> Value {
        self.vector_items(self.keyboard.menu.tool_bar_item_properties)[idx]
    }

    fn set_prop(&mut self, idx: usize, val: Value) {
        let props = self.keyboard.menu.tool_bar_item_properties;
        self.vector_set(props, idx, val);
    }

    fn parse_tool_bar_item(&mut self, key: Value, item: Value) -> LispResult<bool> {
        let mut filter = Value::NIL;
        let mut have_label = false;
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
        if self.keyboard.menu.tool_bar_item_properties.is_nil() {
            self.keyboard.menu.tool_bar_item_properties =
                self.vector(vec![Value::NIL; TOOL_BAR_ITEM_NSLOTS]);
        } else {
            for i in 0..TOOL_BAR_ITEM_NSLOTS {
                self.set_prop(i, Value::NIL);
            }
        }
        self.set_prop(TOOL_BAR_ITEM_KEY, key);
        self.set_prop(TOOL_BAR_ITEM_ENABLED_P, sym::T);
        let mut caption = self.xcar(item);
        if !caption.is_string() {
            caption = self.menu_item_eval_property(caption)?;
            if !caption.is_string() {
                return Ok(false);
            }
        }
        self.set_prop(TOOL_BAR_ITEM_CAPTION, caption);
        item = self.xcdr(item);
        if !item.is_cons() {
            if menu_separator_name_p(self.string_bytes(caption)) {
                self.set_prop(TOOL_BAR_ITEM_TYPE, sym::T);
                return Ok(true);
            }
            return Ok(false);
        }
        let binding = self.xcar(item);
        self.set_prop(TOOL_BAR_ITEM_BINDING, binding);
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
                self.set_prop(
                    TOOL_BAR_ITEM_ENABLED_P,
                    if enable_disabled { sym::T } else { value },
                );
            } else if ikey == sym::K_VISIBLE {
                if self.menu_item_eval_property(value)?.is_nil() {
                    return Ok(false);
                }
            } else if ikey == sym::K_HELP {
                self.set_prop(TOOL_BAR_ITEM_HELP, value);
            } else if ikey == sym::K_VERT_ONLY {
                self.set_prop(TOOL_BAR_ITEM_VERT_ONLY, value);
            } else if ikey == sym::K_LABEL {
                let label = if value.is_string() {
                    value
                } else {
                    self.make_string(BAD_LABEL)
                };
                self.set_prop(TOOL_BAR_ITEM_LABEL, label);
                have_label = true;
            } else if ikey == sym::K_FILTER {
                filter = value;
            } else if ikey == sym::K_BUTTON && value.is_cons() {
                let (kind, selected) = (self.xcar(value), self.xcdr(value));
                if kind == sym::K_TOGGLE || kind == sym::K_RADIO {
                    self.set_prop(TOOL_BAR_ITEM_SELECTED_P, selected);
                    self.set_prop(TOOL_BAR_ITEM_TYPE, kind);
                }
            } else if ikey == sym::K_IMAGE
                && (value.is_cons()
                    || (self.is_vector(value) && self.vector_items(value).len() == 4))
            {
                self.set_prop(TOOL_BAR_ITEM_IMAGES, value);
            } else if ikey == sym::K_RTL {
                self.set_prop(TOOL_BAR_ITEM_RTL_IMAGE, value);
            } else if ikey == sym::K_WRAP {
                return Ok(false);
            }
            item = self.xcdr(item);
        }
        if !have_label {
            self.make_tool_bar_label()?;
        }
        if !filter.is_nil() {
            let binding = self.prop(TOOL_BAR_ITEM_BINDING);
            let quoted = self.list(&[sym::QUOTE, binding]);
            let form = self.list(&[filter, quoted]);
            let filtered = self.menu_item_eval_property(form)?;
            self.set_prop(TOOL_BAR_ITEM_BINDING, filtered);
        }
        let binding = self.prop(TOOL_BAR_ITEM_BINDING);
        if self.get_keymap(binding, false, true)?.is_cons() {
            return Ok(false);
        }
        let keys = crate::keymap::f_where_is_internal(
            self,
            &[binding, Value::NIL, sym::T, Value::NIL, Value::NIL],
        )?;
        if !keys.is_nil() {
            let mut orig = self.prop(TOOL_BAR_ITEM_HELP);
            if orig.is_nil() {
                orig = self.prop(TOOL_BAR_ITEM_CAPTION);
            }
            let desc = crate::keymap::f_key_description(self, &[keys, Value::NIL])?;
            let beg = self.string("  (");
            let end = self.string(")");
            let help = crate::fns::f_concat(self, &[orig, beg, desc, end])?;
            self.set_prop(TOOL_BAR_ITEM_HELP, help);
        }
        let enabled = self.prop(TOOL_BAR_ITEM_ENABLED_P);
        if enabled != sym::T {
            let enabled = self.menu_item_eval_property(enabled)?;
            self.set_prop(TOOL_BAR_ITEM_ENABLED_P, enabled);
        }
        let selected = self.prop(TOOL_BAR_ITEM_SELECTED_P);
        if !selected.is_nil() {
            let selected = self.menu_item_eval_property(selected)?;
            self.set_prop(TOOL_BAR_ITEM_SELECTED_P, selected);
        }
        Ok(true)
    }

    fn make_tool_bar_label(&mut self) -> LispResult<()> {
        let tkey = self.prop(TOOL_BAR_ITEM_KEY);
        let tcapt = self.prop(TOOL_BAR_ITEM_CAPTION);
        let max_label_size = self.int_var(sym::TOOL_BAR_MAX_LABEL_SIZE);
        let max_lbl_size = (2 * max_label_size.clamp(0, i64::from(i32::MAX) / 2) + 1) as usize;
        let c_str =
            |bytes: &[u8]| -> Vec<u8> { bytes.split(|&c| c == 0).next().unwrap_or(&[]).to_vec() };
        let mut label = if tkey.is_symbol() {
            c_str(self.string_bytes(self.symbol_name(tkey)))
        } else {
            Vec::new()
        };
        let capt = if tcapt.is_string() {
            c_str(self.string_bytes(tcapt))
        } else {
            Vec::new()
        };
        let caption_len = capt.len().min(max_lbl_size);
        if 0 < caption_len && caption_len < max_lbl_size {
            let mut buf = capt;
            while buf.last() == Some(&b'.') {
                buf.pop();
            }
            label = buf;
        }
        let label_len = label.len().min(max_lbl_size);
        if 0 < label_len && label_len < max_lbl_size {
            for c in label.iter_mut() {
                if *c == b'-' {
                    *c = b' ';
                }
            }
        } else {
            label = Vec::new();
        }
        let label = self.make_string(&label);
        let new_lbl = crate::casefiddle::f_upcase_initials(self, &[label])?;
        if self.string_chars(new_lbl) as i64 <= max_label_size {
            self.set_prop(TOOL_BAR_ITEM_LABEL, new_lbl);
        } else {
            let empty = self.unibyte_string(b"");
            self.set_prop(TOOL_BAR_ITEM_LABEL, empty);
        }
        Ok(())
    }

    fn init_tool_bar_items(&mut self, reuse: Value) {
        self.keyboard.menu.tool_bar_items_vector = if self.is_vector(reuse) {
            reuse
        } else {
            self.vector(vec![Value::NIL; 64])
        };
        self.keyboard.menu.ntool_bar_items = 0;
    }

    fn append_tool_bar_item(&mut self) {
        let n = self.keyboard.menu.ntool_bar_items;
        let size = self
            .vector_items(self.keyboard.menu.tool_bar_items_vector)
            .len();
        let incr = n as i64 - (size as i64 - TOOL_BAR_ITEM_NSLOTS as i64);
        if incr > 0 {
            let vector = self.keyboard.menu.tool_bar_items_vector;
            self.keyboard.menu.tool_bar_items_vector =
                crate::fns::larger_vector(self, vector, incr as usize);
        }
        for i in 0..TOOL_BAR_ITEM_NSLOTS {
            let v = self.prop(i);
            self.set_tool_bar_slot(n + i, v);
        }
        self.keyboard.menu.ntool_bar_items += TOOL_BAR_ITEM_NSLOTS;
    }
}
