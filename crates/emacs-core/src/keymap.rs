use crate::character::{CHAR_META, CHAR_MODIFIER_MASK, MAX_5_BYTE_CHAR, MAX_CHAR};
use crate::error::LispResult;
use crate::keyboard::{
    ALT_MODIFIER, CTRL_MODIFIER, HYPER_MODIFIER, META_MODIFIER, SHIFT_MODIFIER, SUPER_MODIFIER,
};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

const KEY_MASK: i64 = CHAR_META | (CHAR_META - 1);

pub const SUBRS: &[Subr] = &[
    Subr::fixed("keymapp", 1, 1, f_keymapp),
    Subr::fixed("keymap-parent", 1, 1, f_keymap_parent),
    Subr::fixed("keymap-prompt", 1, 1, f_keymap_prompt),
    Subr::fixed("set-keymap-parent", 2, 2, f_set_keymap_parent),
    Subr::fixed("make-keymap", 0, 1, f_make_keymap),
    Subr::fixed("make-sparse-keymap", 0, 1, f_make_sparse_keymap),
    Subr::fixed("map-keymap-internal", 2, 2, f_map_keymap_internal),
    Subr::fixed("map-keymap", 2, 3, f_map_keymap),
    Subr::fixed("copy-keymap", 1, 1, f_copy_keymap),
    Subr::fixed("command-remapping", 1, 3, f_command_remapping),
    Subr::fixed("key-binding", 1, 4, f_key_binding),
    Subr::fixed("minor-mode-key-binding", 1, 2, f_minor_mode_key_binding),
    Subr::fixed("define-key", 3, 4, f_define_key),
    Subr::fixed("lookup-key", 2, 3, f_lookup_key),
    Subr::fixed("use-global-map", 1, 1, f_use_global_map),
    Subr::fixed("use-local-map", 1, 1, f_use_local_map),
    Subr::fixed("current-local-map", 0, 0, f_current_local_map),
    Subr::fixed("current-global-map", 0, 0, f_current_global_map),
    Subr::fixed("current-minor-mode-maps", 0, 0, f_current_minor_mode_maps),
    Subr::fixed("current-active-maps", 0, 2, f_current_active_maps),
    Subr::fixed("accessible-keymaps", 1, 2, f_accessible_keymaps),
    Subr::fixed("key-description", 1, 2, f_key_description),
    Subr::fixed("keymap--get-keyelt", 2, 2, f_keymap_get_keyelt),
    Subr::fixed("help--describe-vector", 7, 7, f_help_describe_vector),
    Subr::fixed("describe-vector", 1, 2, f_describe_vector),
    Subr::fixed("single-key-description", 1, 2, f_single_key_description),
    Subr::fixed("text-char-description", 1, 1, f_text_char_description),
    Subr::fixed("where-is-internal", 1, 5, f_where_is_internal),
    Subr::fixed("describe-buffer-bindings", 1, 3, f_describe_buffer_bindings),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.put(sym::KEYMAP, sym::CHAR_TABLE_EXTRA_SLOTS, Value::fixnum(0));
    let minibuffer_map = lisp.list(&[sym::KEYMAP]);
    lisp.defvar(sym::MINIBUFFER_LOCAL_MAP, minibuffer_map);
    lisp.defvar(sym::MINOR_MODE_MAP_ALIST, Value::NIL);
    lisp.defvar(sym::MINOR_MODE_OVERRIDING_MAP_ALIST, Value::NIL);
    lisp.defvar(sym::EMULATION_MODE_MAP_ALISTS, Value::NIL);
    lisp.defvar(sym::WHERE_IS_PREFERRED_MODIFIER, Value::NIL);
    lisp.defvar(sym::DESCRIBE_BINDINGS_CHECK_SHADOWING_IN_RANGES, Value::NIL);
}

struct Roots(usize);

impl Lisp {
    fn roots_mark(&self) -> Roots {
        Roots(self.stack.len())
    }

    fn roots_release<T>(&mut self, mark: Roots, value: T) -> T {
        self.stack.truncate(mark.0);
        value
    }

    fn protect(&mut self, v: Value) -> Value {
        self.stack.push(v);
        v
    }

    fn get_keymap_noload(&self, object: Value) -> Value {
        if object.is_nil() {
            return Value::NIL;
        }
        if object.is_cons() && self.xcar(object) == sym::KEYMAP {
            return object;
        }
        let tem = self.indirect_function(object);
        if tem.is_cons() {
            if self.xcar(tem) == sym::KEYMAP {
                return tem;
            }
            if self.xcar(tem) == sym::AUTOLOAD
                && object.is_symbol()
                && self.nth(4, tem) == sym::KEYMAP
            {
                return object;
            }
        }
        Value::NIL
    }

    pub fn keymapp(&self, object: Value) -> bool {
        !self.get_keymap_noload(object).is_nil()
    }

    pub fn get_keymap(&mut self, object: Value, error: bool, autoload: bool) -> LispResult {
        loop {
            if !object.is_nil() {
                if object.is_cons() && self.xcar(object) == sym::KEYMAP {
                    return Ok(object);
                }
                let tem = self.indirect_function(object);
                if tem.is_cons() {
                    if self.xcar(tem) == sym::KEYMAP {
                        return Ok(tem);
                    }
                    if (autoload || !error)
                        && self.xcar(tem) == sym::AUTOLOAD
                        && object.is_symbol()
                        && self.nth(4, tem) == sym::KEYMAP
                    {
                        if autoload {
                            let mark = self.roots_mark();
                            self.protect(object);
                            let r = self.autoload_do_load(tem, object, Value::NIL);
                            self.roots_release(mark, r)?;
                            continue;
                        }
                        return Ok(object);
                    }
                }
            }
            if error {
                return Err(self.wrong_type(sym::KEYMAPP, object));
            }
            return Ok(Value::NIL);
        }
    }

    pub fn make_sparse_keymap(&mut self, prompt: Value) -> Value {
        if prompt.is_nil() {
            self.list(&[sym::KEYMAP])
        } else {
            self.list(&[sym::KEYMAP, prompt])
        }
    }

    fn keymap_parent(&mut self, keymap: Value, autoload: bool) -> LispResult {
        let keymap = self.get_keymap(keymap, true, autoload)?;
        let mut list = self.xcdr(keymap);
        while list.is_cons() {
            if self.keymapp(list) {
                return Ok(list);
            }
            list = self.xcdr(list);
        }
        self.get_keymap(list, false, autoload)
    }

    fn keymap_memberp(&mut self, map: Value, mut maps: Value) -> LispResult<bool> {
        if map.is_nil() {
            return Ok(false);
        }
        while self.keymapp(maps) && map != maps {
            maps = self.keymap_parent(maps, false)?;
        }
        Ok(map == maps)
    }

    pub fn set_keymap_parent(&mut self, keymap: Value, parent: Value) -> LispResult {
        let mark = self.roots_mark();
        self.protect(parent);
        let r = self.set_keymap_parent_1(keymap, parent);
        self.roots_release(mark, r)
    }

    fn set_keymap_parent_1(&mut self, keymap: Value, mut parent: Value) -> LispResult {
        let keymap = self.get_keymap(keymap, true, true)?;
        if !parent.is_nil() {
            parent = self.get_keymap(parent, true, false)?;
            if self.keymap_memberp(keymap, parent)? {
                return Err(self.error("Cyclic keymap inheritance"));
            }
        }
        let mut prev = keymap;
        loop {
            let list = self.xcdr(prev);
            if !list.is_cons() || self.keymapp(list) {
                self.heap.set_cdr(prev, parent);
                return Ok(parent);
            }
            prev = list;
        }
    }

    fn meta_prefix_char(&mut self) -> Value {
        let v = self.find_symbol_value(sym::META_PREFIX_CHAR);
        if let Some(n) = v.as_fixnum()
            && n & CHAR_META != 0
        {
            let fixed = Value::fixnum(27);
            self.set_plain_value(sym::META_PREFIX_CHAR, fixed);
            return fixed;
        }
        v
    }

    fn canonical_index(&mut self, idx: Value) -> LispResult {
        let idx = self.event_head(idx);
        if idx.is_symbol() {
            self.reorder_modifiers(idx)
        } else if let Some(n) = idx.as_fixnum() {
            Ok(Value::fixnum(n & KEY_MASK))
        } else {
            Ok(idx)
        }
    }

    pub fn access_keymap(
        &mut self,
        map: Value,
        idx: Value,
        t_ok: bool,
        noinherit: bool,
    ) -> LispResult {
        let val = self.access_keymap_1(map, idx, t_ok, noinherit)?;
        Ok(if val == sym::UNBOUND { Value::NIL } else { val })
    }

    fn access_keymap_1(
        &mut self,
        map: Value,
        idx: Value,
        t_ok: bool,
        noinherit: bool,
    ) -> LispResult {
        let mark = self.roots_mark();
        self.protect(map);
        self.protect(idx);
        let r = self.access_keymap_2(map, idx, t_ok, noinherit);
        self.roots_release(mark, r)
    }

    fn access_keymap_2(
        &mut self,
        mut map: Value,
        idx: Value,
        mut t_ok: bool,
        noinherit: bool,
    ) -> LispResult {
        let mut idx = self.canonical_index(idx)?;
        self.protect(idx);
        if let Some(n) = idx.as_fixnum()
            && n & META_MODIFIER != 0
        {
            let meta_prefix = self.meta_prefix_char();
            let event_meta_binding = self.access_keymap_1(map, meta_prefix, t_ok, noinherit)?;
            self.protect(event_meta_binding);
            let event_meta_map = self.get_keymap(event_meta_binding, false, true)?;
            if event_meta_map.is_cons() {
                map = self.protect(event_meta_map);
                idx = Value::fixnum(n & !META_MODIFIER);
            } else if t_ok {
                idx = sym::T;
            } else {
                return Ok(if event_meta_binding.is_nil() {
                    Value::NIL
                } else {
                    sym::UNBOUND
                });
            }
        }
        let mut t_binding = sym::UNBOUND;
        let mut retval = sym::UNBOUND;
        let mut retval_tail = Value::NIL;
        let mut tail = if map.is_cons() && self.xcar(map) == sym::KEYMAP {
            self.xcdr(map)
        } else {
            map
        };
        loop {
            if !tail.is_cons() {
                tail = self.get_keymap(tail, false, true)?;
                if !tail.is_cons() {
                    break;
                }
                self.protect(tail);
            }
            let mut val = sym::UNBOUND;
            let binding = self.xcar(tail);
            let submap = self.get_keymap(binding, false, true)?;
            if binding == sym::KEYMAP {
                if noinherit || retval.is_nil() {
                    break;
                } else if retval != sym::UNBOUND {
                    let parent = self.access_keymap_1(tail, idx, t_ok, false)?;
                    let parent_entry = self.get_keymap(parent, false, true)?;
                    if self.keymapp(parent_entry) {
                        if retval_tail.is_cons() {
                            self.heap.set_cdr(retval_tail, parent_entry);
                        } else {
                            retval_tail = self.cons(retval, parent_entry);
                            retval = self.cons(sym::KEYMAP, retval_tail);
                            self.protect(retval);
                        }
                    }
                    break;
                }
            } else if submap.is_cons() {
                self.protect(submap);
                val = self.access_keymap_1(submap, idx, t_ok, noinherit)?;
            } else if binding.is_cons() {
                let key = self.xcar(binding);
                if key == idx {
                    val = self.xcdr(binding);
                } else if t_ok && key == sym::T {
                    t_binding = self.protect(self.xcdr(binding));
                    t_ok = false;
                }
            } else if self.is_vector(binding) {
                if let Some(n) = idx.as_fixnum()
                    && n >= 0
                    && (n as usize) < self.vector_items(binding).len()
                {
                    val = self.vector_items(binding)[n as usize];
                }
            } else if self.is_char_table(binding)
                && let Some(n) = idx.as_fixnum()
                && n >= 0
                && n & CHAR_MODIFIER_MASK == 0
            {
                val = self.aref_value(binding, idx)?;
                if val.is_nil() {
                    val = sym::UNBOUND;
                }
            }
            if val != sym::UNBOUND {
                if val == sym::T {
                    val = Value::NIL;
                }
                self.protect(val);
                val = self.get_keyelt(val, true)?;
                self.protect(val);
                if !self.keymapp(val) {
                    if retval.is_nil() || retval == sym::UNBOUND {
                        retval = val;
                    }
                    if !val.is_nil() {
                        break;
                    }
                } else if retval.is_nil() || retval == sym::UNBOUND {
                    retval = val;
                } else if retval_tail.is_cons() {
                    let cell = self.list(&[val]);
                    self.heap.set_cdr(retval_tail, cell);
                    retval_tail = cell;
                } else {
                    retval_tail = self.list(&[val]);
                    let inner = self.cons(retval, retval_tail);
                    retval = self.cons(sym::KEYMAP, inner);
                    self.protect(retval);
                }
            }
            tail = self.xcdr(tail);
        }
        if retval == sym::UNBOUND {
            self.get_keyelt(t_binding, true)
        } else {
            Ok(retval)
        }
    }

    pub fn get_keyelt(&mut self, mut object: Value, autoload: bool) -> LispResult {
        loop {
            if !object.is_cons() {
                return Ok(object);
            }
            let head = self.xcar(object);
            if head == sym::MENU_ITEM {
                let rest = self.xcdr(object);
                if !rest.is_cons() {
                    return Ok(object);
                }
                object = self.xcdr(rest);
                let mut tem = object;
                if object.is_cons() {
                    object = self.xcar(object);
                }
                while tem.is_cons() && self.xcdr(tem).is_cons() {
                    if self.xcar(tem) == K_FILTER && autoload {
                        let filter = self.xcar(self.xcdr(tem));
                        let quoted = self.list(&[sym::QUOTE, object]);
                        let form = self.list(&[filter, quoted]);
                        object = self.menu_item_eval_property(form)?;
                        break;
                    }
                    tem = self.xcdr(tem);
                }
            } else if head.is_string() {
                object = self.xcdr(object);
            } else {
                return Ok(object);
            }
        }
    }

    pub fn menu_item_eval_property(&mut self, sexpr: Value) -> LispResult {
        let count = self.specpdl.len();
        self.stack.push(sexpr);
        let bound = self.specbind(sym::INHIBIT_REDISPLAY, sym::T);
        let r = match bound {
            Ok(()) => self.condition_case_internal(
                sym::ERROR,
                |l| l.eval(sexpr),
                |l, error| {
                    let head = l.car_safe(error);
                    if head == sym::QUIT {
                        let data = l.cdr_safe(error);
                        Err(l.signal(head, data))
                    } else {
                        Ok(Value::NIL)
                    }
                },
            ),
            Err(e) => Err(e),
        };
        self.stack.pop();
        self.unbind_to_value(count, r)
    }

    fn store_in_keymap(
        &mut self,
        keymap: Value,
        idx: Value,
        def: Value,
        remove: bool,
    ) -> LispResult {
        let mark = self.roots_mark();
        self.protect(keymap);
        self.protect(idx);
        self.protect(def);
        let r = self.store_in_keymap_1(keymap, idx, def, remove);
        self.roots_release(mark, r)
    }

    fn store_in_keymap_1(
        &mut self,
        keymap: Value,
        mut idx: Value,
        mut def: Value,
        remove: bool,
    ) -> LispResult {
        if idx == sym::KEYMAP {
            return Err(self.error("`keymap' is reserved for embedded parent maps"));
        }
        if def.is_cons() {
            let head = self.xcar(def);
            if head == sym::MENU_ITEM || head.is_string() {
                def = self.cons(head, self.xcdr(def));
                self.protect(def);
            }
        }
        if !keymap.is_cons() || self.xcar(keymap) != sym::KEYMAP {
            return Err(self.error("Attempt to define a key in a non-keymap"));
        }
        let range = idx.is_cons() && self.is_character(self.xcar(idx));
        if range {
            self.check_character(self.xcdr(idx))?;
        } else {
            idx = self.canonical_index(idx)?;
            self.protect(idx);
        }
        let mut insertion_point = keymap;
        let mut tail = self.xcdr(keymap);
        while tail.is_cons() {
            let elt = self.xcar(tail);
            if self.is_vector(elt) {
                let size = self.vector_items(elt).len() as i64;
                if let Some(n) = idx.as_fixnum()
                    && n >= 0
                    && n < size
                {
                    self.aset_value(elt, idx, def)?;
                    return Ok(def);
                } else if range {
                    let from = self.xcar(idx).as_fixnum().unwrap_or(0);
                    let full_to = self.xcdr(idx).as_fixnum().unwrap_or(0);
                    let to = full_to.min(size - 1);
                    for i in from..=to {
                        self.aset_value(elt, Value::fixnum(i), def)?;
                    }
                    if to == full_to {
                        return Ok(def);
                    }
                }
                insertion_point = tail;
            } else if self.is_char_table(elt) {
                let sdef = if remove {
                    Value::NIL
                } else if def.is_nil() {
                    sym::T
                } else {
                    def
                };
                if let Some(n) = idx.as_fixnum()
                    && n >= 0
                    && n & CHAR_MODIFIER_MASK == 0
                {
                    self.aset_value(elt, idx, sdef)?;
                    return Ok(def);
                } else if range {
                    let from = self.check_character(self.xcar(idx))?;
                    let to = self.check_character(self.xcdr(idx))?;
                    self.char_table_set_range(elt, from, to, sdef);
                    return Ok(def);
                }
                insertion_point = tail;
            } else if elt.is_cons() {
                let key = self.xcar(elt);
                if key == sym::KEYMAP {
                    tail = elt;
                    insertion_point = elt;
                } else if key == idx {
                    if remove {
                        crate::fns::f_delq(self, &[elt, insertion_point])?;
                    } else {
                        self.heap.set_cdr(elt, def);
                    }
                    return Ok(def);
                } else if range && self.is_character(key) {
                    let from = self.xcar(idx).as_fixnum().unwrap_or(0);
                    let to = self.xcdr(idx).as_fixnum().unwrap_or(0);
                    let k = key.as_fixnum().unwrap_or(0);
                    if from <= k && to >= k {
                        if remove {
                            insertion_point = crate::fns::f_delq(self, &[elt, insertion_point])?;
                        } else {
                            self.heap.set_cdr(elt, def);
                        }
                        if from == to {
                            return Ok(def);
                        }
                    }
                }
            } else if elt == sym::KEYMAP {
                break;
            }
            tail = self.xcdr(tail);
        }
        if !remove {
            let elt = if range {
                let table = self.make_char_table(sym::KEYMAP, Value::NIL)?;
                let from = self.check_character(self.xcar(idx))?;
                let to = self.check_character(self.xcdr(idx))?;
                let value = if def.is_nil() { sym::T } else { def };
                self.char_table_set_range(table, from, to, value);
                table
            } else {
                self.cons(idx, def)
            };
            let rest = self.xcdr(insertion_point);
            let cell = self.cons(elt, rest);
            self.heap.set_cdr(insertion_point, cell);
        }
        Ok(def)
    }

    fn is_character(&self, v: Value) -> bool {
        matches!(v.as_fixnum(), Some(n) if (0..=MAX_CHAR as i64).contains(&n))
    }

    fn copy_keymap_item(&mut self, elt: Value, depth: usize) -> LispResult {
        if !elt.is_cons() {
            return Ok(elt);
        }
        let mark = self.roots_mark();
        self.protect(elt);
        let r = self.copy_keymap_item_1(elt, depth);
        self.roots_release(mark, r)
    }

    fn copy_keymap_item_1(&mut self, mut elt: Value, depth: usize) -> LispResult {
        let mut res = elt;
        let mut tem = elt;
        if self.xcar(tem) == sym::MENU_ITEM {
            elt = self.cons(self.xcar(tem), self.xcdr(tem));
            res = self.protect(elt);
            tem = self.xcdr(elt);
            if tem.is_cons() {
                let cell = self.cons(self.xcar(tem), self.xcdr(tem));
                self.heap.set_cdr(elt, cell);
                elt = cell;
                tem = self.xcdr(elt);
            }
            if tem.is_cons() {
                let cell = self.cons(self.xcar(tem), self.xcdr(tem));
                self.heap.set_cdr(elt, cell);
                elt = cell;
                tem = self.xcar(elt);
                if tem.is_cons() && self.xcar(tem) == sym::KEYMAP {
                    let copy = self.copy_keymap_1(tem, depth)?;
                    self.heap.set_car(elt, copy);
                }
            }
        } else if self.xcar(tem).is_string() {
            elt = self.cons(self.xcar(tem), self.xcdr(tem));
            res = self.protect(elt);
            tem = self.xcdr(elt);
            if tem.is_cons() && self.xcar(tem).is_string() {
                let cell = self.cons(self.xcar(tem), self.xcdr(tem));
                self.heap.set_cdr(elt, cell);
                elt = cell;
                tem = self.xcdr(elt);
            }
            if tem.is_cons() && self.xcar(tem) == sym::KEYMAP {
                let copy = self.copy_keymap_1(tem, depth)?;
                self.heap.set_cdr(elt, copy);
            }
        } else if self.xcar(tem) == sym::KEYMAP {
            res = self.copy_keymap_1(elt, depth)?;
        }
        Ok(res)
    }

    fn copy_keymap_1(&mut self, keymap: Value, depth: usize) -> LispResult {
        if depth > 100 {
            return Err(self.error("Possible infinite recursion when copying keymap"));
        }
        let mark = self.roots_mark();
        let r = self.copy_keymap_2(keymap, depth);
        self.roots_release(mark, r)
    }

    fn copy_keymap_2(&mut self, keymap: Value, depth: usize) -> LispResult {
        let keymap = self.get_keymap(keymap, true, false)?;
        self.protect(keymap);
        let copy = self.list(&[sym::KEYMAP]);
        self.protect(copy);
        let mut tail = copy;
        let mut rest = self.xcdr(keymap);
        while rest.is_cons() && self.xcar(rest) != sym::KEYMAP {
            let mut elt = self.xcar(rest);
            if self.is_char_table(elt) {
                elt = crate::fns::f_copy_sequence(self, &[elt])?;
                self.protect(elt);
                let ranges = self.map_char_table_collect(elt);
                for (from, to, val) in ranges {
                    let item = self.copy_keymap_item(val, depth + 1)?;
                    self.char_table_set_range(elt, from, to, item);
                }
            } else if self.is_vector(elt) {
                elt = crate::fns::f_copy_sequence(self, &[elt])?;
                self.protect(elt);
                let len = self.vector_items(elt).len();
                for i in 0..len {
                    let item = self.vector_items(elt)[i];
                    let item = self.copy_keymap_item(item, depth + 1)?;
                    self.aset_value(elt, Value::fixnum(i as i64), item)?;
                }
            } else if elt.is_cons() {
                if self.xcar(elt) == sym::KEYMAP {
                    elt = self.copy_keymap_1(elt, depth + 1)?;
                } else {
                    let item = self.copy_keymap_item(self.xcdr(elt), depth + 1)?;
                    elt = self.cons(self.xcar(elt), item);
                }
            }
            let cell = self.list(&[elt]);
            self.heap.set_cdr(tail, cell);
            tail = cell;
            rest = self.xcdr(rest);
        }
        self.heap.set_cdr(tail, rest);
        Ok(copy)
    }

    fn map_keymap_internal(
        &mut self,
        map: Value,
        fun: &mut dyn FnMut(&mut Lisp, Value, Value) -> LispResult<()>,
    ) -> LispResult {
        let mark = self.roots_mark();
        self.protect(map);
        let r = self.map_keymap_internal_1(map, fun);
        self.roots_release(mark, r)
    }

    fn map_keymap_internal_1(
        &mut self,
        map: Value,
        fun: &mut dyn FnMut(&mut Lisp, Value, Value) -> LispResult<()>,
    ) -> LispResult {
        let mut tail = if map.is_cons() && self.xcar(map) == sym::KEYMAP {
            self.xcdr(map)
        } else {
            map
        };
        while tail.is_cons() && self.xcar(tail) != sym::KEYMAP {
            let binding = self.xcar(tail);
            if self.keymapp(binding) {
                break;
            } else if binding.is_cons() {
                let val = self.xcdr(binding);
                let val = if val == sym::T { Value::NIL } else { val };
                fun(self, self.xcar(binding), val)?;
            } else if self.is_vector(binding) {
                let len = self.vector_items(binding).len();
                for c in 0..len {
                    let val = self
                        .vector_items(binding)
                        .get(c)
                        .copied()
                        .unwrap_or(Value::NIL);
                    let val = if val == sym::T { Value::NIL } else { val };
                    fun(self, Value::fixnum(c as i64), val)?;
                }
            } else if self.is_char_table(binding) {
                let ranges = self.map_char_table_collect(binding);
                let base = self.stack.len();
                self.stack.extend(ranges.iter().map(|r| r.2));
                for (from, to, val) in ranges {
                    let key = if from == to {
                        Value::fixnum(from as i64)
                    } else {
                        self.cons(Value::fixnum(from as i64), Value::fixnum(to as i64))
                    };
                    let val = if val == sym::T { Value::NIL } else { val };
                    fun(self, key, val)?;
                }
                self.stack.truncate(base);
            }
            tail = self.xcdr(tail);
        }
        Ok(tail)
    }

    pub fn map_keymap(
        &mut self,
        map: Value,
        fun: &mut dyn FnMut(&mut Lisp, Value, Value) -> LispResult<()>,
        autoload: bool,
    ) -> LispResult<()> {
        let mark = self.roots_mark();
        let r = self.map_keymap_1(map, fun, autoload);
        self.roots_release(mark, r)
    }

    fn map_keymap_1(
        &mut self,
        map: Value,
        fun: &mut dyn FnMut(&mut Lisp, Value, Value) -> LispResult<()>,
        autoload: bool,
    ) -> LispResult<()> {
        let mut map = self.get_keymap(map, true, autoload)?;
        while map.is_cons() {
            self.protect(map);
            let head = self.xcar(map);
            if self.keymapp(head) {
                self.map_keymap(head, fun, autoload)?;
                map = self.xcdr(map);
            } else {
                map = self.map_keymap_internal(map, fun)?;
            }
            if !map.is_cons() {
                map = self.get_keymap(map, false, autoload)?;
            }
        }
        Ok(())
    }

    fn check_vector_or_string(&mut self, key: Value) -> LispResult<usize> {
        if self.is_vector(key) {
            Ok(self.vector_items(key).len())
        } else if key.is_string() {
            Ok(self.string_chars(key))
        } else {
            Err(self.wrong_type(sym::ARRAYP, key))
        }
    }

    fn possibly_translate_key_sequence(&mut self, key: Value, length: &mut usize) -> LispResult {
        if self.is_vector(key)
            && self.vector_items(key).len() == 1
            && self.vector_items(key)[0].is_string()
        {
            let spec = self.vector_items(key)[0];
            let valid_p = self.heap.symbol_ref(sym::KEY_VALID_P).function;
            if valid_p.is_nil() {
                let msg =
                    self.string("`key-valid-p' is not defined, so this syntax can't be used: %s");
                return Err(self.signal_list(sym::ERROR, &[msg, key]));
            }
            if self.funcall(sym::KEY_VALID_P, &[spec])?.is_nil() {
                return Ok(key);
            }
            let parsed = self.funcall(sym::KEY_PARSE, &[spec])?;
            *length = self.check_vector_or_string(parsed)?;
            if *length == 0 {
                let msg = self.string("Invalid `key-parse' syntax: %S");
                return Err(self.signal_list(sym::ERROR, &[msg, parsed]));
            }
            return Ok(parsed);
        }
        Ok(key)
    }

    fn invalid_event_message(&mut self, c: Value) -> LispResult<()> {
        let text = format!(
            "Key sequence contains invalid event {}",
            self.princ_to_string(c)
        );
        let s = self.string(&text);
        self.message_string(s)
    }

    pub fn define_key(
        &mut self,
        keymap: Value,
        key: Value,
        def: Value,
        remove: bool,
    ) -> LispResult {
        let mark = self.roots_mark();
        self.protect(keymap);
        self.protect(key);
        self.protect(def);
        let r = self.define_key_1(keymap, key, def, remove);
        self.roots_release(mark, r)
    }

    fn define_key_1(
        &mut self,
        keymap: Value,
        key: Value,
        mut def: Value,
        remove: bool,
    ) -> LispResult {
        let mut metized = false;
        let mut keymap = self.get_keymap(keymap, true, true)?;
        self.protect(keymap);
        let mut length = self.check_vector_or_string(key)?;
        if length == 0 {
            return Ok(Value::NIL);
        }
        let multibyte = key.is_string() && self.string_multibyte(key);
        let meta_bit = if self.is_vector(key) || multibyte {
            META_MODIFIER
        } else {
            0x80
        };
        if self.is_vector(def)
            && !self.vector_items(def).is_empty()
            && self.vector_items(def)[0].is_cons()
        {
            let items = self.vector_items(def).to_vec();
            let tmp = self.vector(vec![Value::NIL; items.len()]);
            self.protect(tmp);
            for (i, defi) in items.into_iter().enumerate().rev() {
                let defi = if defi.is_cons() && self.lucid_event_type_list_p(defi) {
                    self.event_convert_list(defi)?
                } else {
                    defi
                };
                self.aset_value(tmp, Value::fixnum(i as i64), defi)?;
            }
            def = tmp;
        }
        let key = self.possibly_translate_key_sequence(key, &mut length)?;
        self.protect(key);
        let mut idx = 0usize;
        loop {
            let mut c = self.aref_value(key, Value::fixnum(idx as i64))?;
            if c.is_cons() {
                if self.lucid_event_type_list_p(c) {
                    c = self.event_convert_list(c)?;
                } else if self.is_character(self.xcar(c)) {
                    self.check_character(self.xcdr(c))?;
                }
            }
            if c.is_symbol() {
                self.silly_event_symbol_error(c)?;
            }
            let meta_prefix = self.meta_prefix_char();
            if let Some(n) = c.as_fixnum()
                && n & meta_bit != 0
                && !metized
            {
                c = meta_prefix;
                metized = true;
            } else {
                if let Some(n) = c.as_fixnum() {
                    c = Value::fixnum(n & !meta_bit);
                }
                metized = false;
                idx += 1;
            }
            if !c.is_fixnum()
                && !c.is_symbol()
                && (!c.is_cons() || (self.xcar(c).is_fixnum() && idx != length))
            {
                self.invalid_event_message(c)?;
            }
            if idx == length {
                return self.store_in_keymap(keymap, c, def, remove);
            }
            let mut cmd = self.access_keymap(keymap, c, false, true)?;
            if cmd.is_nil() {
                cmd = self.define_as_prefix(keymap, c)?;
            }
            self.protect(cmd);
            keymap = self.get_keymap(cmd, false, true)?;
            if !keymap.is_cons() {
                let trailing_esc = if c == meta_prefix && metized {
                    if idx == 0 { "ESC" } else { " ESC" }
                } else {
                    ""
                };
                let whole = self.key_description(key, Value::NIL)?;
                let whole = self.string_to_rust(whole);
                let prefix = crate::fns::f_substring(
                    self,
                    &[key, Value::fixnum(0), Value::fixnum(idx as i64)],
                )?;
                let prefix = self.key_description(prefix, Value::NIL)?;
                let prefix = self.string_to_rust(prefix);
                return Err(self.error(&format!(
                    "Key sequence {whole} starts with non-prefix key {prefix}{trailing_esc}"
                )));
            }
            self.protect(keymap);
        }
    }

    fn define_as_prefix(&mut self, keymap: Value, c: Value) -> LispResult {
        let cmd = self.make_sparse_keymap(Value::NIL);
        self.store_in_keymap(keymap, c, cmd, false)?;
        Ok(cmd)
    }

    fn silly_event_symbol_error(&mut self, c: Value) -> LispResult<()> {
        let parsed = self.parse_modifiers(c)?;
        let modifiers = self.xcar(self.xcdr(parsed)).as_fixnum().unwrap_or(0);
        let base = self.xcar(parsed);
        let name = self.symbol_name_string(base);
        let replacement = match name.as_str() {
            "DEL" => "\\d",
            "TAB" => "\\t",
            "RET" => "\\r",
            "ESC" => "\\e",
            "SPC" => " ",
            _ => return Ok(()),
        };
        let mut new_mods = String::new();
        for (bit, text) in [
            (ALT_MODIFIER, "\\A-"),
            (CTRL_MODIFIER, "\\C-"),
            (HYPER_MODIFIER, "\\H-"),
            (META_MODIFIER, "\\M-"),
            (SHIFT_MODIFIER, "\\S-"),
            (SUPER_MODIFIER, "\\s-"),
        ] {
            if modifiers & bit != 0 {
                new_mods.push_str(text);
            }
        }
        let c = self.reorder_modifiers(c)?;
        let cname = self.symbol_name_string(c);
        Err(self.error(&format!(
            "To bind the key {cname}, use [?{new_mods}{replacement}], not [{cname}]"
        )))
    }

    fn lookup_key_1(&mut self, keymap: Value, key: Value, accept_default: Value) -> LispResult {
        let mark = self.roots_mark();
        self.protect(keymap);
        self.protect(key);
        let r = self.lookup_key_2(keymap, key, accept_default);
        self.roots_release(mark, r)
    }

    fn lookup_key_2(&mut self, mut keymap: Value, key: Value, accept_default: Value) -> LispResult {
        let t_ok = !accept_default.is_nil();
        if !keymap.is_cons() && !keymap.is_nil() {
            keymap = self.get_keymap(keymap, true, true)?;
            self.protect(keymap);
        }
        let mut length = self.check_vector_or_string(key)?;
        if length == 0 {
            return Ok(keymap);
        }
        let key = self.possibly_translate_key_sequence(key, &mut length)?;
        self.protect(key);
        let unibyte = key.is_string() && !self.string_multibyte(key);
        let mut idx = 0usize;
        loop {
            let mut c = self.aref_value(key, Value::fixnum(idx as i64))?;
            idx += 1;
            if c.is_cons() && self.lucid_event_type_list_p(c) {
                c = self.event_convert_list(c)?;
            }
            if unibyte
                && let Some(n) = c.as_fixnum()
                && n & 0x80 != 0
            {
                c = Value::fixnum((n | META_MODIFIER) & !0x80);
            }
            if !c.is_fixnum() && !c.is_symbol() && !c.is_cons() && !c.is_string() {
                self.invalid_event_message(c)?;
            }
            let cmd = self.access_keymap(keymap, c, t_ok, false)?;
            if idx == length {
                return Ok(cmd);
            }
            self.protect(cmd);
            keymap = self.get_keymap(cmd, false, true)?;
            if !keymap.is_cons() {
                return Ok(Value::fixnum(idx as i64));
            }
            self.protect(keymap);
        }
    }

    pub fn lookup_key(&mut self, keymap: Value, key: Value, accept_default: Value) -> LispResult {
        let found = self.lookup_key_1(keymap, key, accept_default)?;
        if !found.is_nil() && !self.is_number(found) {
            return Ok(found);
        }
        if !self.is_vector(key)
            || self.vector_items(key).is_empty()
            || self.vector_items(key)[0] != sym::MENU_BAR
        {
            return Ok(found);
        }
        let mark = self.roots_mark();
        self.protect(keymap);
        self.protect(key);
        let r = self.lookup_menu_key(keymap, key, accept_default);
        self.roots_release(mark, r)
    }

    fn lookup_menu_key(&mut self, keymap: Value, key: Value, accept_default: Value) -> LispResult {
        let items = self.vector_items(key).to_vec();
        let mut new_items = Vec::with_capacity(items.len());
        for item in &items {
            if item.is_symbol() {
                let name = self.symbol_name_string(*item);
                let lowered: String = name
                    .chars()
                    .map(|ch| {
                        let mut lower = ch.to_lowercase();
                        match (lower.next(), lower.next()) {
                            (Some(l), None) => l,
                            _ => ch,
                        }
                    })
                    .collect();
                new_items.push(self.intern(&lowered));
            } else {
                new_items.push(*item);
            }
        }
        let new_key = self.vector(new_items.clone());
        self.protect(new_key);
        let found = self.lookup_key_1(keymap, new_key, accept_default)?;
        if !found.is_nil() && !self.is_number(found) {
            return Ok(found);
        }
        for (i, item) in new_items.iter().enumerate() {
            if !item.is_symbol() {
                continue;
            }
            let name = self.symbol_name_string(*item);
            if !name.contains(' ') {
                continue;
            }
            let dashed = self.intern(&name.replace(' ', "-"));
            self.aset_value(new_key, Value::fixnum(i as i64), dashed)?;
        }
        self.lookup_key_1(keymap, new_key, accept_default)
    }

    pub fn current_minor_maps(&mut self) -> LispResult<Vec<(Value, Value)>> {
        let mut out = Vec::new();
        let lists = [
            self.find_symbol_value(sym::MINOR_MODE_OVERRIDING_MAP_ALIST),
            self.find_symbol_value(sym::MINOR_MODE_MAP_ALIST),
        ];
        let mut emulation = self.find_symbol_value(sym::EMULATION_MODE_MAP_ALISTS);
        let mut list_number: i32 = 0;
        while list_number < 2 {
            let mut alist;
            if emulation.is_cons() {
                alist = self.xcar(emulation);
                emulation = self.xcdr(emulation);
                if alist.is_symbol() {
                    alist = self.find_symbol_value(alist);
                }
                list_number = -1;
            } else {
                alist = lists[list_number as usize];
            }
            while alist.is_cons() {
                let assoc = self.xcar(alist);
                alist = self.xcdr(alist);
                if !assoc.is_cons() {
                    continue;
                }
                let var = self.xcar(assoc);
                if !var.is_symbol() {
                    continue;
                }
                let val = self.find_symbol_value(var);
                if val == sym::UNBOUND || val.is_nil() {
                    continue;
                }
                if list_number == 1 && !self.assq(var, lists[0]).is_nil() {
                    continue;
                }
                let temp = self.indirect_function(self.xcdr(assoc));
                if !temp.is_nil() {
                    out.push((var, temp));
                }
            }
            list_number += 1;
        }
        Ok(out)
    }

    fn click_position(&mut self, position: Value) -> LispResult<usize> {
        let pos = if let Some(n) = position.as_fixnum() {
            n
        } else if self.is_marker(position) {
            self.marker_position_checked(position)? as i64
        } else {
            self.cur().pt as i64
        };
        let (begv, zv) = (self.cur().begv as i64, self.cur().zv as i64);
        if !(begv <= pos && pos <= zv) {
            let buffer = self.current_buffer;
            return Err(self.args_out_of_range(buffer, position));
        }
        Ok(pos as usize)
    }

    pub fn get_local_map(&mut self, position: usize, buffer: Value, kind: Value) -> LispResult {
        let (begv, zv, z) = {
            let b = self.buffer(buffer);
            (b.begv, b.zv, b.z())
        };
        let position = position.clamp(begv, zv);
        let saved = {
            let b = self.buffer(buffer);
            (b.begv, b.begv_byte, b.zv, b.zv_byte)
        };
        let z_byte = self.buffer(buffer).z_byte();
        {
            let b = self.buffer_mut(buffer);
            b.begv = 1;
            b.begv_byte = 1;
            b.zv = z;
            b.zv_byte = z_byte;
        }
        let lispy_position = Value::fixnum(position as i64);
        let mut prop = self.get_char_property(lispy_position, kind, buffer);
        if let Ok(v) = prop
            && v.is_nil()
        {
            prop = self.get_pos_property(lispy_position, kind, buffer);
        }
        {
            let b = self.buffer_mut(buffer);
            (b.begv, b.begv_byte, b.zv, b.zv_byte) = saved;
        }
        let prop = self.get_keymap(prop?, false, false)?;
        if prop.is_cons() {
            return Ok(prop);
        }
        if kind == sym::KEYMAP {
            Ok(Value::NIL)
        } else {
            Ok(self.buffer(buffer).keymap)
        }
    }

    pub fn current_active_maps(&mut self, olp: Value, position: Value) -> LispResult {
        let mark = self.roots_mark();
        self.protect(position);
        let r = self.current_active_maps_1(olp, position);
        self.roots_release(mark, r)
    }

    fn current_active_maps_1(&mut self, olp: Value, position: Value) -> LispResult {
        let global = self.current_global_map;
        let mut keymaps = self.list(&[global]);
        self.protect(keymaps);
        let otlp = self.find_symbol_value(sym::OVERRIDING_TERMINAL_LOCAL_MAP);
        let olm = self.find_symbol_value(sym::OVERRIDING_LOCAL_MAP);
        if !olp.is_nil() && otlp.is_nil() && !olm.is_nil() {
            keymaps = self.cons(olm, keymaps);
            self.protect(keymaps);
        }
        if self.xcdr(keymaps).is_nil() {
            let pt = self.click_position(position)?;
            let buffer = self.current_buffer;
            let mut local_map = self.get_local_map(pt, buffer, sym::LOCAL_MAP)?;
            self.protect(local_map);
            let mut keymap = self.get_local_map(pt, buffer, sym::KEYMAP)?;
            self.protect(keymap);
            if position.is_cons() {
                let string = self.nth(4, position);
                local_map = Value::NIL;
                keymap = Value::NIL;
                if string.is_cons() && self.xcar(string).is_string() {
                    let pos = self.xcdr(string);
                    let s = self.xcar(string);
                    if let Some(p) = pos.as_fixnum()
                        && p >= 0
                        && (p as usize) < self.string_chars(s)
                    {
                        local_map = self.string_property_lookup(s, p as usize, sym::LOCAL_MAP);
                        keymap = self.string_property_lookup(s, p as usize, sym::KEYMAP);
                        self.protect(local_map);
                        self.protect(keymap);
                    }
                }
                let buffer_posn = if self.heap.symbol_ref(sym::POSN_POINT).function.is_nil() {
                    Value::NIL
                } else {
                    self.funcall(sym::POSN_POINT, &[position])?
                };
                if let Some(bp) = buffer_posn.as_fixnum()
                    && bp >= 1
                    && bp as usize <= self.cur().z()
                {
                    if local_map.is_nil() {
                        local_map = self.get_local_map(bp as usize, buffer, sym::LOCAL_MAP)?;
                        self.protect(local_map);
                    }
                    if keymap.is_nil() {
                        keymap = self.get_local_map(bp as usize, buffer, sym::KEYMAP)?;
                        self.protect(keymap);
                    }
                }
                if local_map.is_nil() {
                    local_map = self.cur().keymap;
                }
            }
            if !local_map.is_nil() {
                keymaps = self.cons(local_map, keymaps);
                self.protect(keymaps);
            }
            let maps = self.current_minor_maps()?;
            for (_, map) in maps.into_iter().rev() {
                if !map.is_nil() {
                    keymaps = self.cons(map, keymaps);
                    self.protect(keymaps);
                }
            }
            if !keymap.is_nil() {
                keymaps = self.cons(keymap, keymaps);
                self.protect(keymaps);
            }
            if !olp.is_nil() && !otlp.is_nil() {
                keymaps = self.cons(otlp, keymaps);
            }
        }
        Ok(keymaps)
    }

    fn string_property_lookup(&mut self, string: Value, pos: usize, prop: Value) -> Value {
        let plist = self.string_property_at(string, pos, prop);
        if !plist.is_nil() {
            return plist;
        }
        Value::NIL
    }

    pub fn command_remapping(
        &mut self,
        command: Value,
        position: Value,
        keymaps: Value,
    ) -> LispResult {
        if !command.is_symbol() {
            return Ok(Value::NIL);
        }
        let vector = self.vector(vec![sym::REMAP, command]);
        let mark = self.roots_mark();
        self.protect(vector);
        let r = if keymaps.is_nil() {
            self.key_binding(vector, Value::NIL, sym::T, position)
        } else {
            self.lookup_key(keymaps, vector, Value::NIL)
        };
        let command = self.roots_release(mark, r)?;
        Ok(if command.is_fixnum() {
            Value::NIL
        } else {
            command
        })
    }

    pub fn key_binding(
        &mut self,
        key: Value,
        accept_default: Value,
        no_remap: Value,
        mut position: Value,
    ) -> LispResult {
        if position.is_nil() && self.is_vector(key) {
            let items = self.vector_items(key);
            if items.is_empty() {
                return Ok(Value::NIL);
            }
            let event = if items[0].is_symbol() && items.len() > 1 {
                items[1]
            } else {
                items[0]
            };
            if event.is_cons() && self.xcdr(event).is_cons() {
                let head = self.event_head(event);
                let kind = self.get(head, sym::EVENT_KIND);
                if kind == sym::MOUSE_CLICK {
                    position = self.car_safe(self.xcdr(event));
                }
            }
        }
        let mark = self.roots_mark();
        self.protect(key);
        self.protect(position);
        let r = self.key_binding_1(key, accept_default, no_remap, position);
        self.roots_release(mark, r)
    }

    fn key_binding_1(
        &mut self,
        key: Value,
        accept_default: Value,
        no_remap: Value,
        position: Value,
    ) -> LispResult {
        let maps = self.current_active_maps(sym::T, position)?;
        self.protect(maps);
        let mut value = self.lookup_key(maps, key, accept_default)?;
        if value.is_nil() || value.is_fixnum() {
            return Ok(Value::NIL);
        }
        if no_remap.is_nil() && value.is_symbol() {
            self.protect(value);
            let remapped = self.command_remapping(value, position, Value::NIL)?;
            if !remapped.is_nil() {
                value = remapped;
            }
        }
        Ok(value)
    }

    pub fn key_description(&mut self, keys: Value, prefix: Value) -> LispResult {
        let mut parts: Vec<Value> = Vec::new();
        let sep = self.string(" ");
        let mark = self.roots_mark();
        self.protect(sep);
        let mut add_meta = false;
        let meta_prefix = self.meta_prefix_char();
        for list in [prefix, keys] {
            if !(list.is_nil() || list.is_string() || self.is_vector(list) || list.is_cons()) {
                self.stack.truncate(mark.0);
                return Err(self.wrong_type(sym::ARRAYP, list));
            }
            let elements: Vec<Value> = if list.is_string() {
                let multibyte = self.string_multibyte(list);
                self.string_char_vec(list)
                    .into_iter()
                    .map(|c| {
                        let c = c as i64;
                        let c = if !multibyte && c & 0o200 != 0 {
                            c ^ (0o200 | META_MODIFIER)
                        } else {
                            c
                        };
                        Value::fixnum(c)
                    })
                    .collect()
            } else if list.is_nil() {
                Vec::new()
            } else {
                match self.sequence_elements(list) {
                    Ok(v) => v,
                    Err(e) => {
                        self.stack.truncate(mark.0);
                        return Err(e);
                    }
                }
            };
            for mut key in elements {
                if add_meta {
                    let is_meta_key = match key.as_fixnum() {
                        Some(n) => key == meta_prefix || n & META_MODIFIER != 0,
                        None => true,
                    };
                    if is_meta_key {
                        let d = self.single_key_description(meta_prefix, Value::NIL)?;
                        parts.push(self.protect(d));
                        parts.push(sep);
                        if key == meta_prefix {
                            continue;
                        }
                    } else {
                        key = Value::fixnum(key.as_fixnum().unwrap_or(0) | META_MODIFIER);
                    }
                    add_meta = false;
                } else if key == meta_prefix {
                    add_meta = true;
                    continue;
                }
                let d = match self.single_key_description(key, Value::NIL) {
                    Ok(d) => d,
                    Err(e) => {
                        self.stack.truncate(mark.0);
                        return Err(e);
                    }
                };
                parts.push(self.protect(d));
                parts.push(sep);
            }
        }
        let result = if add_meta {
            let d = self.single_key_description(meta_prefix, Value::NIL)?;
            parts.push(self.protect(d));
            self.concat_to_string(&parts)
        } else if parts.is_empty() {
            Ok(self.empty_unibyte)
        } else {
            parts.pop();
            self.concat_to_string(&parts)
        };
        self.roots_release(mark, result)
    }

    pub fn single_key_description(&mut self, key: Value, no_angles: Value) -> LispResult {
        let mut key = key;
        if key.is_cons() && self.lucid_event_type_list_p(key) {
            key = self.event_convert_list(key)?;
        }
        if key.is_cons() && self.xcar(key).is_fixnum() && self.xcdr(key).is_fixnum() {
            let a = self.single_key_description(self.xcar(key), no_angles)?;
            let mark = self.roots_mark();
            self.protect(a);
            let b = self.single_key_description(self.xcdr(key), no_angles);
            let r = match b {
                Ok(b) => {
                    self.protect(b);
                    let dots = self.string("..");
                    self.concat_to_string(&[a, dots, b])
                }
                Err(e) => Err(e),
            };
            return self.roots_release(mark, r);
        }
        let key = self.event_head(key);
        if let Some(n) = key.as_fixnum() {
            let chars = push_key_description(n);
            return Ok(self.string_from_chars(&chars, true));
        }
        if key.is_symbol() {
            if !no_angles.is_nil() {
                return Ok(self.symbol_name(key));
            }
            let name = self.symbol_name(key);
            let bytes = self.string_bytes(name).to_vec();
            let len = bytes.len();
            let mut i = 0;
            while i + 3 < len && bytes[i + 1] == b'-' && b"CMSsHA".contains(&bytes[i]) {
                i += 2;
            }
            let mut buffer = Vec::with_capacity(len + 2);
            buffer.extend_from_slice(&bytes[..i]);
            buffer.push(b'<');
            buffer.extend_from_slice(&bytes[i..]);
            buffer.push(b'>');
            let multibyte = self.string_multibyte(name);
            return Ok(self.make_string_from_bytes(buffer, multibyte));
        }
        if key.is_string() {
            return crate::fns::f_copy_sequence(self, &[key]);
        }
        Err(self.error("KEY must be an integer, cons, symbol, or string"))
    }

    fn preferred_sequence_p(&mut self, seq: Value, preferred: i64) -> LispResult<i32> {
        let mut result = 1;
        for elt in self.sequence_elements(seq)? {
            match elt.as_fixnum() {
                None => return Ok(0),
                Some(n) => {
                    let modifiers = n & (CHAR_MODIFIER_MASK & !CHAR_META);
                    if modifiers == preferred {
                        result = 2;
                    } else if modifiers != 0 {
                        return Ok(0);
                    }
                }
            }
        }
        Ok(result)
    }

    fn shadow_lookup(
        &mut self,
        keymap: Value,
        key: Value,
        accept_default: Value,
        remap: bool,
    ) -> LispResult {
        let value = self.lookup_key(keymap, key, accept_default)?;
        if matches!(value.as_fixnum(), Some(n) if n >= 0) {
            Ok(Value::NIL)
        } else if !value.is_nil() && remap && value.is_symbol() {
            self.stack.push(value);
            let remapping = self.command_remapping(value, Value::NIL, keymap);
            self.stack.pop();
            let remapping = remapping?;
            Ok(if remapping.is_nil() { value } else { remapping })
        } else {
            Ok(value)
        }
    }

    fn append_key(&mut self, seq: Value, key: Value) -> LispResult {
        let mut items = self.sequence_elements(seq)?;
        items.push(key);
        Ok(self.vector(items))
    }

    fn mouse_events_member(&mut self, event: Value) -> bool {
        if [
            sym::MENU_BAR,
            sym::TAB_BAR,
            sym::TOOL_BAR,
            sym::TAB_LINE,
            sym::HEADER_LINE,
            sym::MODE_LINE,
        ]
        .contains(&event)
        {
            return true;
        }
        if !event.is_symbol() {
            return false;
        }
        let name = self.symbol_name_string(event);
        matches!(
            name.as_str(),
            "mouse-1" | "mouse-2" | "mouse-3" | "mouse-4" | "mouse-5"
        )
    }

    fn where_is_internal_seqs(
        &mut self,
        definition: Value,
        keymaps: Value,
        noindirect: bool,
        nomenus: bool,
    ) -> LispResult {
        let mark = self.roots_mark();
        self.protect(definition);
        self.protect(keymaps);
        let r = self.where_is_internal_seqs_1(definition, keymaps, noindirect, nomenus);
        self.roots_release(mark, r)
    }

    fn where_is_internal_seqs_1(
        &mut self,
        definition: Value,
        keymaps: Value,
        noindirect: bool,
        nomenus: bool,
    ) -> LispResult {
        let mut maps = Value::NIL;
        let mut found = keymaps;
        while found.is_cons() {
            let km = self.get_keymap(self.xcar(found), true, false)?;
            let acc = self.accessible_keymaps(km, Value::NIL)?;
            maps = self.nconc2(maps, acc);
            self.protect(maps);
            found = self.xcdr(found);
        }
        let mut sequences = Value::NIL;
        let meta_prefix = self.meta_prefix_char();
        while maps.is_cons() {
            let entry = self.xcar(maps);
            maps = self.xcdr(maps);
            let this = self.car_safe(entry);
            let map = self.cdr_safe(entry);
            let this_len = self.sequence_elements(this)?.len();
            let last = this_len as i64 - 1;
            let last_is_meta =
                last >= 0 && self.aref_value(this, Value::fixnum(last))? == meta_prefix;
            if nomenus && last >= 0 {
                let first = self.aref_value(this, Value::fixnum(0))?;
                if first.is_symbol() {
                    let parsed = self.parse_modifiers(first)?;
                    let base = self.xcar(parsed);
                    if self.mouse_events_member(base) {
                        continue;
                    }
                }
            }
            if map.is_cons() {
                let mut collected = Vec::new();
                self.map_keymap(
                    map,
                    &mut |lisp, key, binding| {
                        let binding = if noindirect {
                            binding
                        } else {
                            lisp.get_keyelt(binding, false)?
                        };
                        let matches = binding == definition
                            || (definition.is_cons() && lisp.equal(binding, definition)?);
                        if !matches {
                            return Ok(());
                        }
                        let sequence = if key.is_fixnum() && last_is_meta {
                            let seq = crate::fns::f_copy_sequence(lisp, &[this])?;
                            let n = key.as_fixnum().unwrap_or(0) | META_MODIFIER;
                            lisp.aset_value(seq, Value::fixnum(last), Value::fixnum(n))?;
                            seq
                        } else {
                            let key = if key.is_cons() {
                                lisp.cons(lisp.xcar(key), lisp.xcdr(key))
                            } else {
                                key
                            };
                            lisp.append_key(this, key)?
                        };
                        lisp.stack.push(sequence);
                        collected.push(sequence);
                        Ok(())
                    },
                    false,
                )?;
                for seq in collected {
                    sequences = self.cons(seq, sequences);
                    self.protect(sequences);
                }
            }
        }
        Ok(sequences)
    }

    pub fn where_is_internal(
        &mut self,
        definition: Value,
        keymap: Value,
        firstonly: Value,
        noindirect: Value,
        no_remap: Value,
    ) -> LispResult {
        let mark = self.roots_mark();
        self.protect(definition);
        self.protect(keymap);
        let r = self.where_is_internal_1(definition, keymap, firstonly, noindirect, no_remap);
        self.roots_release(mark, r)
    }

    fn where_is_internal_1(
        &mut self,
        mut definition: Value,
        keymap: Value,
        firstonly: Value,
        noindirect: Value,
        no_remap: Value,
    ) -> LispResult {
        let nomenus = !firstonly.is_nil() && firstonly != sym::NON_ASCII;
        let preferred_sym = self.find_symbol_value(sym::WHERE_IS_PREFERRED_MODIFIER);
        let preferred = self.parse_solitary_modifier(preferred_sym);
        let keymaps = if keymap.is_cons() && self.keymapp(self.xcar(keymap)) {
            keymap
        } else if !keymap.is_nil() {
            let global = self.current_global_map;
            self.list(&[keymap, global])
        } else {
            self.current_active_maps(Value::NIL, Value::NIL)?
        };
        self.protect(keymaps);
        let tem = self.command_remapping(definition, Value::NIL, keymaps)?;
        if no_remap.is_nil() && !tem.is_nil() {
            definition = self.protect(tem);
        }
        if definition.is_symbol() && !firstonly.is_nil() {
            let mut tem = self.get(definition, sym::K_ADVERTISED_BINDING);
            if !tem.is_nil() {
                self.protect(tem);
                while tem.is_cons() {
                    let candidate = self.xcar(tem);
                    if self.shadow_lookup(keymaps, candidate, Value::NIL, false)? == definition {
                        return Ok(candidate);
                    }
                    tem = self.xcdr(tem);
                }
                if !tem.is_nil()
                    && self.shadow_lookup(keymaps, tem, Value::NIL, false)? == definition
                {
                    return Ok(tem);
                }
            }
        }
        let seqs =
            self.where_is_internal_seqs(definition, keymaps, !noindirect.is_nil(), nomenus)?;
        let mut sequences = self.list_to_vec(seqs)?;
        sequences.reverse();
        let base = self.stack.len();
        self.stack.extend_from_slice(&sequences);
        let mut queue: std::collections::VecDeque<Value> = sequences.into();
        let mut remapped_sequences: Vec<Value> = Vec::new();
        let mut remapped = false;
        let mut found: Vec<Value> = Vec::new();
        loop {
            let sequence = match queue.pop_front() {
                Some(s) => s,
                None => {
                    if remapped {
                        break;
                    }
                    remapped = true;
                    queue = std::mem::take(&mut remapped_sequences).into();
                    match queue.pop_front() {
                        Some(s) => s,
                        None => break,
                    }
                }
            };
            let shadow = self.shadow_lookup(keymaps, sequence, Value::NIL, remapped)?;
            if !self.equal(shadow, definition)? {
                continue;
            }
            if no_remap.is_nil()
                && !remapped
                && self.is_vector(sequence)
                && self.vector_items(sequence).len() == 2
                && self.vector_items(sequence)[0] == sym::REMAP
                && self.vector_items(sequence)[1].is_symbol()
            {
                let function = self.vector_items(sequence)[1];
                let seqs =
                    self.where_is_internal_seqs(function, keymaps, !noindirect.is_nil(), nomenus)?;
                let mut seqs = self.list_to_vec(seqs)?;
                self.stack.extend_from_slice(&seqs);
                seqs.reverse();
                seqs.extend(remapped_sequences);
                remapped_sequences = seqs;
                continue;
            }
            if !sequence.is_nil() {
                let len = self.sequence_elements(sequence)?.len();
                if len > 0 {
                    let last = self.aref_value(sequence, Value::fixnum(len as i64 - 1))?;
                    if last.is_string() {
                        let any = self.string("(any string)");
                        self.aset_value(sequence, Value::fixnum(len as i64 - 1), any)?;
                    }
                }
            }
            let mut seen = false;
            for f in &found {
                if self.equal(*f, sequence)? {
                    seen = true;
                    break;
                }
            }
            let non_key = self.is_vector(sequence)
                && self.vector_items(sequence).len() == 1
                && self.vector_items(sequence)[0].is_symbol()
                && !self
                    .get(self.vector_items(sequence)[0], sym::NON_KEY_EVENT)
                    .is_nil();
            if !seen && !non_key {
                found.push(sequence);
            }
            if firstonly == sym::NON_ASCII
                || (!firstonly.is_nil() && self.preferred_sequence_p(sequence, preferred)? == 2)
            {
                self.stack.truncate(base);
                return Ok(sequence);
            }
        }
        self.stack.truncate(base);
        if firstonly.is_nil() {
            return Ok(self.list(&found));
        }
        if preferred == 0 {
            return Ok(found.first().copied().unwrap_or(Value::NIL));
        }
        for &binding in &found {
            if self.preferred_sequence_p(binding, preferred)? != 0 {
                return Ok(binding);
            }
        }
        Ok(found.first().copied().unwrap_or(Value::NIL))
    }

    pub fn accessible_keymaps(&mut self, keymap: Value, prefix: Value) -> LispResult {
        let mark = self.roots_mark();
        self.protect(keymap);
        self.protect(prefix);
        let r = self.accessible_keymaps_1(keymap, prefix);
        self.roots_release(mark, r)
    }

    fn accessible_keymaps_1(&mut self, keymap: Value, mut prefix: Value) -> LispResult {
        let prefixlen = self.length(prefix)?.as_fixnum().unwrap_or(0);
        let maps = if !prefix.is_nil() {
            let tem = self.lookup_key(keymap, prefix, sym::T)?;
            let tem = self.get_keymap(tem, false, false)?;
            if tem.is_nil() {
                return Ok(Value::NIL);
            }
            if prefix.is_string() {
                let multibyte = self.string_multibyte(prefix);
                let items: Vec<Value> = self
                    .string_char_vec(prefix)
                    .into_iter()
                    .map(|c| {
                        let c = c as i64;
                        let c = if !multibyte && c & 0o200 != 0 {
                            c ^ (0o200 | META_MODIFIER)
                        } else {
                            c
                        };
                        Value::fixnum(c)
                    })
                    .collect();
                prefix = self.vector(items);
                self.protect(prefix);
            }
            let entry = self.cons(prefix, tem);
            self.list(&[entry])
        } else {
            let km = self.get_keymap(keymap, true, false)?;
            let empty = self.vector(Vec::new());
            let entry = self.cons(empty, km);
            self.list(&[entry])
        };
        self.protect(maps);
        let meta_prefix = self.meta_prefix_char();
        let mut tail = maps;
        while tail.is_cons() {
            let entry = self.xcar(tail);
            let thismap = self.cdr(entry)?;
            let thisseq = self.car(entry)?;
            let thislen = self.length(thisseq)?.as_fixnum().unwrap_or(0);
            let last = thislen - 1;
            let is_metized = last >= 0
                && last >= prefixlen
                && self.aref_value(thisseq, Value::fixnum(last))? == meta_prefix;
            if thismap.is_cons() {
                let current_tail = tail;
                self.map_keymap(
                    thismap,
                    &mut |lisp, key, cmd| {
                        lisp.accessible_keymaps_item(
                            key,
                            cmd,
                            maps,
                            current_tail,
                            thisseq,
                            is_metized && key.is_fixnum(),
                        )
                    },
                    false,
                )?;
            }
            tail = self.xcdr(tail);
        }
        Ok(maps)
    }

    fn accessible_keymaps_item(
        &mut self,
        key: Value,
        cmd: Value,
        mut maps: Value,
        tail: Value,
        thisseq: Value,
        is_metized: bool,
    ) -> LispResult<()> {
        let elt = self.get_keyelt(cmd, false)?;
        let cmd = self.get_keymap(elt, false, false)?;
        if cmd.is_nil() {
            return Ok(());
        }
        let thislen = self.length(thisseq)?.as_fixnum().unwrap_or(0);
        loop {
            let tem = self.rassq(cmd, maps);
            if tem.is_nil() {
                break;
            }
            let prefix = self.xcar(tem);
            let lim = self.length(prefix)?.as_fixnum().unwrap_or(0);
            if lim <= thislen {
                let mut i = 0;
                while i < lim
                    && self.aref_value(prefix, Value::fixnum(i))?
                        == self.aref_value(thisseq, Value::fixnum(i))?
                {
                    i += 1;
                }
                if i >= lim {
                    return Ok(());
                }
            }
            let rest = self.memq(tem, maps);
            maps = self.xcdr(rest);
        }
        if is_metized {
            let tem = crate::fns::f_copy_sequence(self, &[thisseq])?;
            let n = key.as_fixnum().unwrap_or(0) | META_MODIFIER;
            self.aset_value(tem, Value::fixnum(thislen - 1), Value::fixnum(n))?;
            let entry = self.cons(tem, cmd);
            let rest = self.xcdr(tail);
            let cell = self.cons(entry, rest);
            self.heap.set_cdr(tail, cell);
        } else {
            let tem = self.append_key(thisseq, key)?;
            let entry = self.cons(tem, cmd);
            let cell = self.list(&[entry]);
            self.nconc2(tail, cell);
        }
        Ok(())
    }

    fn rassq(&self, key: Value, mut list: Value) -> Value {
        while list.is_cons() {
            let elt = self.xcar(list);
            if elt.is_cons() && self.xcdr(elt) == key {
                return elt;
            }
            list = self.xcdr(list);
        }
        Value::NIL
    }

    fn describe_key_maybe_fontify(
        &mut self,
        keys: Value,
        prefix: Value,
        keymap_p: bool,
    ) -> LispResult {
        let desc = self.key_description(keys, prefix)?;
        if keymap_p {
            self.stack.push(desc);
            let props = self.list(&[sym::FONT_LOCK_FACE, sym::HELP_KEY_BINDING]);
            let end = Value::fixnum(self.string_chars(desc) as i64);
            let r = self.add_text_properties(Value::fixnum(0), end, props, desc, false);
            self.stack.pop();
            r?;
        }
        Ok(desc)
    }

    fn insert_value(&mut self, s: Value) -> LispResult<()> {
        self.insert_string(s, false, false)
    }

    #[allow(clippy::too_many_arguments)]
    fn describe_vector(
        &mut self,
        vector: Value,
        prefix: Value,
        args: Value,
        princ_style: bool,
        partial: bool,
        shadow: Value,
        entire_map: Value,
        keymap_p: bool,
        mention_shadow: bool,
    ) -> LispResult<()> {
        let mut prefix = prefix;
        let mut elt_prefix = Value::NIL;
        if !keymap_p {
            if !prefix.is_nil() && self.length(prefix)?.as_fixnum().unwrap_or(0) > 0 {
                let desc = self.key_description(prefix, Value::NIL)?;
                self.protect(desc);
                let space = self.string(" ");
                elt_prefix = self.concat_to_string(&[desc, space])?;
                self.protect(elt_prefix);
            }
            prefix = Value::NIL;
        }
        let kludge = self.vector(vec![Value::NIL]);
        self.protect(kludge);
        let suppress = if partial {
            sym::SUPPRESS_KEYMAP
        } else {
            Value::NIL
        };
        let is_table = self.is_char_table(vector);
        let (mut stop, to) = if is_table {
            (MAX_5_BYTE_CHAR as i64 + 1, MAX_CHAR as i64 + 1)
        } else {
            let n = self.vector_items(vector).len() as i64;
            (n, n)
        };
        let check_ranges = self.find_symbol_value(sym::DESCRIBE_BINDINGS_CHECK_SHADOWING_IN_RANGES);
        let mut first = true;
        let mut i: i64 = 0;
        loop {
            if i == stop {
                if i == to {
                    break;
                }
                stop = to;
            }
            let starting_i = i;
            let mut range_beg = i;
            let val = if is_table {
                let mut from = range_beg as u32;
                let mut end = (stop - 1) as u32;
                let v =
                    self.char_table_ref_and_range(vector, range_beg as u32, &mut from, &mut end);
                range_beg = from as i64;
                i = end as i64;
                v
            } else {
                self.vector_items(vector)[i as usize]
            };
            let definition = self.get_keyelt(val, false)?;
            if definition.is_nil() {
                i += 1;
                continue;
            }
            self.protect(definition);
            if definition.is_symbol() && partial && !self.get(definition, suppress).is_nil() {
                i += 1;
                continue;
            }
            self.aset_value(kludge, Value::fixnum(0), Value::fixnum(starting_i))?;
            let mut this_shadowed = false;
            let mut shadowed_by = Value::NIL;
            if !shadow.is_nil() {
                shadowed_by = self.shadow_lookup(shadow, kludge, sym::T, false)?;
                self.protect(shadowed_by);
                if !shadowed_by.is_nil() && shadowed_by != definition {
                    if mention_shadow {
                        this_shadowed = true;
                    } else {
                        i += 1;
                        continue;
                    }
                }
            }
            if !entire_map.is_nil() {
                let tem = self.lookup_key(entire_map, kludge, sym::T)?;
                if tem != definition {
                    i += 1;
                    continue;
                }
            }
            if first {
                self.insert_str("\n")?;
                first = false;
            }
            if !elt_prefix.is_nil() {
                self.insert_value(elt_prefix)?;
            }
            let desc = self.describe_key_maybe_fontify(kludge, prefix, keymap_p)?;
            self.insert_value(desc)?;
            if !is_table {
                while i + 1 < stop {
                    let next = self.vector_items(vector)[(i + 1) as usize];
                    let tem2 = self.get_keyelt(next, false)?;
                    if tem2.is_nil() || !self.equal(tem2, definition)? {
                        break;
                    }
                    i += 1;
                }
            }
            if !check_ranges.is_nil()
                && is_table
                && i != starting_i
                && (check_ranges != sym::IGNORE_SELF_INSERT
                    || definition != sym::SELF_INSERT_COMMAND)
            {
                let key = self.vector(vec![Value::NIL]);
                self.protect(key);
                for j in range_beg + 1..=i {
                    self.aset_value(key, Value::fixnum(0), Value::fixnum(j))?;
                    let tem = self.shadow_lookup(shadow, key, sym::T, false)?;
                    if !self.equal(tem, shadowed_by)? {
                        i = j - 1;
                        break;
                    }
                }
            }
            if i != starting_i {
                self.insert_str(" .. ")?;
                self.aset_value(kludge, Value::fixnum(0), Value::fixnum(i))?;
                if !elt_prefix.is_nil() {
                    self.insert_value(elt_prefix)?;
                }
                let desc = self.describe_key_maybe_fontify(kludge, prefix, keymap_p)?;
                self.insert_value(desc)?;
            }
            self.describe_elt(definition, args, princ_style)?;
            if this_shadowed {
                let pt = self.cur().pt;
                self.goto_char_internal(pt - 1);
                if shadowed_by.is_symbol() {
                    let name = self.symbol_name_string(shadowed_by);
                    self.insert_str(&format!("  (currently shadowed by `{name}')"))?;
                } else {
                    self.insert_str("  (currently shadowed)")?;
                }
                let pt = self.cur().pt;
                self.goto_char_internal(pt + 1);
            }
            i += 1;
        }
        if is_table {
            let default = self.char_table_default(vector);
            if !default.is_nil() {
                if !elt_prefix.is_nil() {
                    self.insert_value(elt_prefix)?;
                }
                self.insert_str("default")?;
                self.describe_elt(default, args, princ_style)?;
            }
        }
        Ok(())
    }

    fn describe_elt(&mut self, elt: Value, fun: Value, princ_style: bool) -> LispResult<()> {
        if princ_style {
            let indent_to = self.intern("indent-to");
            self.funcall(indent_to, &[Value::fixnum(16), Value::fixnum(1)])?;
            self.funcall(fun, &[elt])?;
            let terpri = self.intern("terpri");
            self.funcall(terpri, &[Value::NIL, Value::NIL])?;
        } else {
            self.funcall(fun, &[elt])?;
        }
        Ok(())
    }

    fn describe_map_tree(&mut self, args: &[Value]) -> LispResult<()> {
        self.funcall(sym::HELP__DESCRIBE_MAP_TREE, args)?;
        Ok(())
    }

    fn describe_buffer_bindings(
        &mut self,
        buffer: Value,
        prefix: Value,
        menus: Value,
    ) -> LispResult<()> {
        let nomenu = if menus.is_nil() { sym::T } else { Value::NIL };
        let mut alternate_heading = Some(
            "Keyboard translations:\n\nYou type        Translation\n--------        -----------\n",
        );
        self.check_buffer(buffer)?;
        let mut shadow = Value::NIL;
        let outbuf = self.current_buffer;
        let translate = self.find_symbol_value(sym::KEYBOARD_TRANSLATE_TABLE);
        if translate.is_string() && !prefix.is_nil() {
            let table = self.string_char_vec(translate);
            for (c, &t) in table.iter().enumerate() {
                if t as usize != c {
                    if let Some(h) = alternate_heading.take() {
                        self.insert_str(h)?;
                    }
                    let a = push_key_description(t as i64);
                    self.insert_char_codes(&a, false, false)?;
                    let indent_to = self.intern("indent-to");
                    self.funcall(indent_to, &[Value::fixnum(16), Value::fixnum(1)])?;
                    let b = push_key_description(c as i64);
                    self.insert_char_codes(&b, false, false)?;
                    self.insert_str("\n")?;
                }
            }
            self.insert_str("\n")?;
        }
        let ktm = self.bound_value(sym::KEY_TRANSLATION_MAP);
        if !ktm.is_nil() {
            let msg = self.unibyte_string(b"Key translations");
            self.describe_map_tree(&[
                ktm,
                Value::NIL,
                Value::NIL,
                prefix,
                msg,
                nomenu,
                sym::T,
                Value::NIL,
                Value::NIL,
                buffer,
            ])?;
        }
        let mut start1 = self.bound_value(sym::OVERRIDING_TERMINAL_LOCAL_MAP);
        if !start1.is_nil() {
            let msg = self.unibyte_string(b"\x0c\nOverriding Bindings");
            self.describe_map_tree(&[
                start1,
                sym::T,
                shadow,
                prefix,
                msg,
                nomenu,
                Value::NIL,
                Value::NIL,
                Value::NIL,
                buffer,
            ])?;
            shadow = self.cons(start1, shadow);
            self.protect(shadow);
            start1 = Value::NIL;
        } else {
            let olm = self.bound_value(sym::OVERRIDING_LOCAL_MAP);
            if !olm.is_nil() {
                start1 = olm;
            }
        }
        if !start1.is_nil() {
            let msg = self.unibyte_string(b"\x0c\nOverriding Bindings");
            self.describe_map_tree(&[
                start1,
                sym::T,
                shadow,
                prefix,
                msg,
                nomenu,
                Value::NIL,
                Value::NIL,
                Value::NIL,
                buffer,
            ])?;
            shadow = self.cons(start1, shadow);
            self.protect(shadow);
        } else {
            self.set_buffer_checked(buffer)?;
            let maps = self.current_minor_maps();
            self.set_buffer_checked(outbuf)?;
            let maps = maps?;
            for (_, m) in &maps {
                self.protect(*m);
            }
            let bpt = self.buffer(buffer).pt;
            let start = self.get_local_map(bpt, buffer, sym::KEYMAP)?;
            if !start.is_nil() {
                self.protect(start);
                let msg = self.unibyte_string(b"\x0c\n`keymap' Property Bindings");
                self.describe_map_tree(&[
                    start,
                    sym::T,
                    shadow,
                    prefix,
                    msg,
                    nomenu,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                    buffer,
                ])?;
                shadow = self.cons(start, shadow);
                self.protect(shadow);
            }
            for (mode, map) in maps {
                let name = self.symbol_name(mode);
                let mut title = b"\x0c\n`".to_vec();
                title.extend_from_slice(self.string_bytes(name));
                title.extend_from_slice(b"' Minor Mode Bindings");
                let msg = self.unibyte_string(&title);
                self.describe_map_tree(&[
                    map,
                    sym::T,
                    shadow,
                    prefix,
                    msg,
                    nomenu,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                    buffer,
                ])?;
                shadow = self.cons(map, shadow);
                self.protect(shadow);
            }
            let bpt = self.buffer(buffer).pt;
            let start = self.get_local_map(bpt, buffer, sym::LOCAL_MAP)?;
            if !start.is_nil() {
                self.protect(start);
                let msg = if start == self.buffer(buffer).keymap {
                    let fmt = self.unibyte_string(b"\x0c\n`%s' Major Mode Bindings");
                    let mode = self.buffer_local_value_or_default(sym::MAJOR_MODE, buffer);
                    self.format_string(&[fmt, mode], false)?
                } else {
                    self.unibyte_string(b"\x0c\n`local-map' Property Bindings")
                };
                self.describe_map_tree(&[
                    start,
                    sym::T,
                    shadow,
                    prefix,
                    msg,
                    nomenu,
                    Value::NIL,
                    Value::NIL,
                    Value::NIL,
                    buffer,
                ])?;
                shadow = self.cons(start, shadow);
                self.protect(shadow);
            }
        }
        let msg = self.unibyte_string(b"\x0c\nGlobal Bindings");
        let global = self.current_global_map;
        self.describe_map_tree(&[
            global,
            sym::T,
            shadow,
            prefix,
            msg,
            nomenu,
            Value::NIL,
            sym::T,
            Value::NIL,
            buffer,
        ])?;
        let lfkm = self.bound_value(sym::LOCAL_FUNCTION_KEY_MAP);
        if !lfkm.is_nil() {
            let msg = self.unibyte_string(b"\x0c\nFunction key map translations");
            self.describe_map_tree(&[
                lfkm,
                Value::NIL,
                Value::NIL,
                prefix,
                msg,
                nomenu,
                sym::T,
                Value::NIL,
                Value::NIL,
                buffer,
            ])?;
        }
        let idm = self.bound_value(sym::INPUT_DECODE_MAP);
        if !idm.is_nil() {
            let msg = self.unibyte_string(b"\x0c\nInput decoding map translations");
            self.describe_map_tree(&[
                idm,
                Value::NIL,
                Value::NIL,
                prefix,
                msg,
                nomenu,
                sym::T,
                Value::NIL,
                Value::NIL,
                buffer,
            ])?;
        }
        Ok(())
    }

    fn bound_value(&self, symbol: Value) -> Value {
        let v = self.find_symbol_value(symbol);
        if v == sym::UNBOUND { Value::NIL } else { v }
    }

    fn buffer_local_value_or_default(&self, symbol: Value, buffer: Value) -> Value {
        match self.buffer_local_get(buffer, symbol) {
            Some(v) => v,
            None => self.bound_value(symbol),
        }
    }
}

const K_FILTER: Value = sym::K_FILTER;

pub fn push_key_description(ch: i64) -> Vec<u32> {
    let mut out = Vec::new();
    let mut c = ch & (META_MODIFIER | (META_MODIFIER - 1));
    let c2 = c & !(ALT_MODIFIER
        | CTRL_MODIFIER
        | HYPER_MODIFIER
        | META_MODIFIER
        | SHIFT_MODIFIER
        | SUPER_MODIFIER);
    if !(0..=MAX_CHAR as i64).contains(&c2) {
        out.extend(format!("[{c}]").chars().map(|ch| ch as u32));
        return out;
    }
    let tab_as_ci = c2 == b'\t' as i64 && c & META_MODIFIER != 0;
    let push = |out: &mut Vec<u32>, s: &str| out.extend(s.chars().map(|ch| ch as u32));
    if c & ALT_MODIFIER != 0 {
        push(&mut out, "A-");
        c -= ALT_MODIFIER;
    }
    if c & CTRL_MODIFIER != 0
        || (c2 < b' ' as i64 && c2 != 27 && c2 != b'\t' as i64 && c2 != 13)
        || tab_as_ci
    {
        push(&mut out, "C-");
        c &= !CTRL_MODIFIER;
    }
    if c & HYPER_MODIFIER != 0 {
        push(&mut out, "H-");
        c -= HYPER_MODIFIER;
    }
    if c & META_MODIFIER != 0 {
        push(&mut out, "M-");
        c -= META_MODIFIER;
    }
    if c & SHIFT_MODIFIER != 0 {
        push(&mut out, "S-");
        c -= SHIFT_MODIFIER;
    }
    if c & SUPER_MODIFIER != 0 {
        push(&mut out, "s-");
        c -= SUPER_MODIFIER;
    }
    if c < 0o40 {
        if c == 0o33 {
            push(&mut out, "ESC");
        } else if tab_as_ci {
            push(&mut out, "i");
        } else if c == b'\t' as i64 {
            push(&mut out, "TAB");
        } else if c == 13 {
            push(&mut out, "RET");
        } else if c > 0 && c <= 26 {
            out.push((c + 0o140) as u32);
        } else {
            out.push((c + 0o100) as u32);
        }
    } else if c == 0o177 {
        push(&mut out, "DEL");
    } else if c == b' ' as i64 {
        push(&mut out, "SPC");
    } else {
        out.push(c as u32);
    }
    out
}

fn push_text_char_description(c: u32) -> Vec<u32> {
    if c < 0o40 {
        vec![b'^' as u32, c + 64]
    } else if c == 0o177 {
        vec![b'^' as u32, b'?' as u32]
    } else {
        vec![c]
    }
}

fn f_keymapp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(if lisp.keymapp(args[0]) {
        sym::T
    } else {
        Value::NIL
    })
}

fn f_keymap_parent(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.keymap_parent(args[0], true)
}

fn keymap_prompt(lisp: &mut Lisp, map: Value) -> LispResult {
    let mut map = lisp.get_keymap(map, false, false)?;
    while map.is_cons() {
        let tem = lisp.xcar(map);
        if tem.is_string() {
            return Ok(tem);
        } else if lisp.keymapp(tem) {
            let tem = keymap_prompt(lisp, tem)?;
            if !tem.is_nil() {
                return Ok(tem);
            }
        }
        map = lisp.xcdr(map);
    }
    Ok(Value::NIL)
}

fn f_keymap_prompt(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    keymap_prompt(lisp, args[0])
}

fn f_set_keymap_parent(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.set_keymap_parent(args[0], args[1])
}

fn f_make_keymap(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let string = args.first().copied().unwrap_or(Value::NIL);
    let tail = if string.is_nil() {
        Value::NIL
    } else {
        lisp.list(&[string])
    };
    lisp.stack.push(tail);
    let table = lisp.make_char_table(sym::KEYMAP, Value::NIL);
    lisp.stack.pop();
    let table = table?;
    let rest = lisp.cons(table, tail);
    Ok(lisp.cons(sym::KEYMAP, rest))
}

fn f_make_sparse_keymap(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let string = args.first().copied().unwrap_or(Value::NIL);
    Ok(lisp.make_sparse_keymap(string))
}

fn f_map_keymap_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, keymap) = (args[0], args[1]);
    let keymap = lisp.get_keymap(keymap, true, true)?;
    lisp.map_keymap_internal(keymap, &mut |l, key, val| {
        l.funcall(function, &[key, val])?;
        Ok(())
    })
}

fn f_map_keymap(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (function, keymap) = (args[0], args[1]);
    let sort_first = args.get(2).copied().unwrap_or(Value::NIL);
    if !sort_first.is_nil() {
        return lisp.funcall(sym::MAP_KEYMAP_SORTED, &[function, keymap]);
    }
    lisp.map_keymap(
        keymap,
        &mut |l, key, val| {
            l.funcall(function, &[key, val])?;
            Ok(())
        },
        true,
    )?;
    Ok(Value::NIL)
}

fn f_copy_keymap(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.copy_keymap_1(args[0], 0)
}

fn f_command_remapping(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let position = args.get(1).copied().unwrap_or(Value::NIL);
    let keymaps = args.get(2).copied().unwrap_or(Value::NIL);
    lisp.command_remapping(args[0], position, keymaps)
}

fn f_key_binding(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = |i: usize| args.get(i).copied().unwrap_or(Value::NIL);
    lisp.key_binding(arg(0), arg(1), arg(2), arg(3))
}

fn f_minor_mode_key_binding(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let key = args[0];
    let accept_default = args.get(1).copied().unwrap_or(Value::NIL);
    let maps = lisp.current_minor_maps()?;
    let base = lisp.stack.len();
    for (m, k) in &maps {
        lisp.stack.push(*m);
        lisp.stack.push(*k);
    }
    let mut out = Vec::new();
    for (mode, map) in maps {
        if map.is_nil() {
            continue;
        }
        let binding = match lisp.lookup_key(map, key, accept_default) {
            Ok(b) => b,
            Err(e) => {
                lisp.stack.truncate(base);
                return Err(e);
            }
        };
        if binding.is_nil() || binding.is_fixnum() {
            continue;
        }
        if lisp.keymapp(binding) {
            let pair = lisp.cons(mode, binding);
            lisp.stack.push(pair);
            out.push(pair);
        } else if out.is_empty() {
            let pair = lisp.cons(mode, binding);
            lisp.stack.truncate(base);
            return Ok(lisp.list(&[pair]));
        }
    }
    let result = lisp.list(&out);
    lisp.stack.truncate(base);
    Ok(result)
}

fn f_define_key(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let remove = args.get(3).copied().unwrap_or(Value::NIL);
    lisp.define_key(args[0], args[1], args[2], !remove.is_nil())
}

fn f_lookup_key(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let accept_default = args.get(2).copied().unwrap_or(Value::NIL);
    lisp.lookup_key(args[0], args[1], accept_default)
}

fn f_use_global_map(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let keymap = lisp.get_keymap(args[0], true, true)?;
    lisp.current_global_map = keymap;
    Ok(Value::NIL)
}

fn f_use_local_map(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let mut keymap = args[0];
    if !keymap.is_nil() {
        keymap = lisp.get_keymap(keymap, true, true)?;
    }
    lisp.cur_mut().keymap = keymap;
    Ok(Value::NIL)
}

fn f_current_local_map(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.cur().keymap)
}

fn f_current_global_map(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(lisp.current_global_map)
}

fn f_current_minor_mode_maps(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let maps: Vec<Value> = lisp
        .current_minor_maps()?
        .into_iter()
        .map(|m| m.1)
        .collect();
    Ok(lisp.list(&maps))
}

fn f_current_active_maps(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let olp = args.first().copied().unwrap_or(Value::NIL);
    let position = args.get(1).copied().unwrap_or(Value::NIL);
    lisp.current_active_maps(olp, position)
}

fn f_accessible_keymaps(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prefix = args.get(1).copied().unwrap_or(Value::NIL);
    lisp.accessible_keymaps(args[0], prefix)
}

fn f_key_description(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prefix = args.get(1).copied().unwrap_or(Value::NIL);
    lisp.key_description(args[0], prefix)
}

fn f_keymap_get_keyelt(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    lisp.get_keyelt(args[0], !args[1].is_nil())
}

fn check_vector_or_char_table(lisp: &mut Lisp, v: Value) -> LispResult<()> {
    if lisp.is_vector(v) || lisp.is_char_table(v) {
        Ok(())
    } else {
        Err(lisp.wrong_type(sym::VECTOR_OR_CHAR_TABLE_P, v))
    }
}

fn f_describe_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let vector = args[0];
    let mut describer = args.get(1).copied().unwrap_or(Value::NIL);
    if describer.is_nil() {
        describer = sym::PRINC;
    }
    let count = lisp.specpdl.len();
    let buffer = lisp.current_buffer;
    lisp.specbind(sym::STANDARD_OUTPUT, buffer)?;
    let r = check_vector_or_char_table(lisp, vector).and_then(|()| {
        let mark = lisp.roots_mark();
        let r = lisp.describe_vector(
            vector,
            Value::NIL,
            describer,
            true,
            false,
            Value::NIL,
            Value::NIL,
            false,
            false,
        );
        lisp.roots_release(mark, r)
    });
    lisp.unbind_to_value(count, r.map(|()| Value::NIL))
}

fn f_help_describe_vector(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (vector, prefix, describer, partial, shadow, entire_map, mention_shadow) = (
        args[0], args[1], args[2], args[3], args[4], args[5], args[6],
    );
    let count = lisp.specpdl.len();
    let buffer = lisp.current_buffer;
    lisp.specbind(sym::STANDARD_OUTPUT, buffer)?;
    let r = check_vector_or_char_table(lisp, vector).and_then(|()| {
        let mark = lisp.roots_mark();
        let r = lisp.describe_vector(
            vector,
            prefix,
            describer,
            false,
            !partial.is_nil(),
            shadow,
            entire_map,
            true,
            !mention_shadow.is_nil(),
        );
        lisp.roots_release(mark, r)
    });
    lisp.unbind_to_value(count, r.map(|()| Value::NIL))
}

fn f_single_key_description(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let no_angles = args.get(1).copied().unwrap_or(Value::NIL);
    lisp.single_key_description(args[0], no_angles)
}

fn f_text_char_description(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let c = lisp.check_character(args[0])?;
    if c >= 0x80 {
        return Ok(lisp.string_from_chars(&[c], true));
    }
    let chars = push_text_char_description(c);
    Ok(lisp.string_from_chars(&chars, false))
}

fn f_where_is_internal(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let arg = |i: usize| args.get(i).copied().unwrap_or(Value::NIL);
    lisp.where_is_internal(arg(0), arg(1), arg(2), arg(3), arg(4))
}

fn f_describe_buffer_bindings(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let prefix = args.get(1).copied().unwrap_or(Value::NIL);
    let menus = args.get(2).copied().unwrap_or(Value::NIL);
    let mark = lisp.roots_mark();
    let r = lisp.describe_buffer_bindings(args[0], prefix, menus);
    lisp.roots_release(mark, r.map(|()| Value::NIL))
}
