use crate::error::LispResult;
use crate::keyboard::Kvar;
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("recursive-edit", 0, 0, f_recursive_edit).interactive(""),
    Subr::fixed("top-level", 0, 0, f_top_level).interactive(""),
    Subr::fixed("exit-recursive-edit", 0, 0, f_exit_recursive_edit).interactive(""),
    Subr::fixed("abort-recursive-edit", 0, 0, f_abort_recursive_edit).interactive(""),
];

impl Lisp {
    pub fn command_loop(&mut self) -> LispResult {
        if self.minibuf.command_loop_level > 0 || self.minibuf.minibuf_level > 0 {
            return self.internal_catch(sym::EXIT, |l| l.command_loop_2(sym::ERROR));
        }
        loop {
            self.internal_catch(sym::TOP_LEVEL, |l| l.top_level_1())?;
            self.internal_catch(sym::TOP_LEVEL, |l| l.command_loop_2(sym::ERROR))?;
            if self.noninteractive {
                crate::emacs::f_kill_emacs(self, &[sym::T, Value::NIL])?;
            }
        }
    }

    fn command_loop_2(&mut self, handlers: Value) -> LispResult {
        loop {
            let val = self.condition_case_internal(
                handlers,
                |l| l.command_loop_1(),
                |l, data| l.cmd_error(data),
            )?;
            if val.is_nil() {
                return Ok(Value::NIL);
            }
        }
    }

    pub fn recursive_edit_1(&mut self) -> LispResult {
        let count = self.specpdl_index();
        if self.minibuf.command_loop_level > 0 {
            self.specbind(sym::STANDARD_OUTPUT, sym::T)?;
            self.specbind(sym::STANDARD_INPUT, sym::T)?;
            self.specbind(sym::SYMBOLS_WITH_POS_ENABLED, Value::NIL)?;
            self.specbind(sym::PRINT_SYMBOLS_BARE, Value::NIL)?;
        }
        self.specbind(sym::INHIBIT_REDISPLAY, Value::NIL)?;
        self.xdisp.redisplaying_p = false;
        self.specbind(sym::UNDO_AUTO__UNDOABLY_CHANGED_BUFFERS, Value::NIL)?;
        let val = self.command_loop()?;
        if val == sym::T {
            self.quit()?;
        }
        if val.is_string() {
            return Err(self.signal_list(sym::ERROR, &[val]));
        }
        if self.functionp(val) {
            self.call0(val)?;
        }
        self.unbind_to(count)?;
        Ok(Value::NIL)
    }

    pub fn recursive_edit(&mut self) -> LispResult {
        let count = self.specpdl_index();
        let selected_buffer = self.window(self.windows.selected_window).contents;
        let buffer =
            if self.minibuf.command_loop_level >= 0 && self.current_buffer != selected_buffer {
                self.current_buffer
            } else {
                Value::NIL
            };
        self.minibuf.command_loop_level += 1;
        self.windows.update_mode_lines = 17;
        self.record_unwind(recursive_edit_unwind, buffer);
        if self.minibuf.command_loop_level > 0
            && !self.var_true(sym::MULTIPLE_TERMINALS_MERGE_KEYBOARDS)
        {
            let selected_frame = self.frames.selected_frame;
            self.temporarily_switch_to_single_kboard(Some(selected_frame))?;
        }
        self.recursive_edit_1()?;
        self.unbind_to(count)?;
        Ok(Value::NIL)
    }

    fn safe_run_hooks_maybe_narrowed(&mut self, hook: Value) -> LispResult<()> {
        let count = self.specpdl_index();
        self.specbind(sym::INHIBIT_QUIT, sym::T)?;
        if self.cur().long_line_optimizations_p
            && self.int_var(sym::LONG_LINE_OPTIMIZATIONS_REGION_SIZE) > 0
        {
            let pt = self.cur().pt;
            let begv = self.get_large_narrowing_begv(pt);
            let zv = self.get_large_narrowing_zv(pt);
            if begv != 1 || zv != self.cur().z() {
                self.labeled_narrow_to_region(
                    Value::fixnum(begv as i64),
                    Value::fixnum(zv as i64),
                    sym::LONG_LINE_OPTIMIZATIONS_IN_COMMAND_HOOKS,
                )?;
            }
        }
        let r = self.run_hook_with_args(&[hook, hook], |l, args| l.safe_run_hook_funcall(args));
        self.unbind_to_value(count, r)?;
        Ok(())
    }

    fn select_selected_window_buffer(&mut self) {
        let contents = self.window(self.windows.selected_window).contents;
        self.set_buffer_internal(contents);
    }

    fn record_last_commands(&mut self) {
        let this_command = self.plain_value(sym::THIS_COMMAND);
        self.set_kvar(Kvar::LastCommand, this_command);
        let real_this_command = self.plain_value(sym::REAL_THIS_COMMAND);
        self.set_kvar(Kvar::RealLastCommand, real_this_command);
        if !self.plain_value(sym::LAST_COMMAND_EVENT).is_cons() {
            self.set_kvar(Kvar::LastRepeatableCommand, real_this_command);
        }
    }

    fn command_loop_1(&mut self) -> LispResult {
        let mut prev_modiff: i64 = 0;
        let mut prev_buffer = Value::NIL;
        self.set_kvar(Kvar::PrefixArg, Value::NIL);
        self.set_kvar(Kvar::LastPrefixArg, Value::NIL);
        self.set_plain_value(sym::DEACTIVATE_MARK, Value::NIL);
        self.keyboard.waiting_for_input = false;
        self.cancel_echoing();
        self.keyboard.this_command_key_count = 0;
        self.keyboard.this_single_command_key_start = 0;
        if self.plain_value(sym::MEMORY_FULL).is_nil() {
            if !self.plain_value(sym::POST_COMMAND_HOOK).is_nil()
                && self.var_true(sym::RUN_HOOKS_ENABLED)
            {
                self.safe_run_hooks_maybe_narrowed(sym::POST_COMMAND_HOOK)?;
            }
            if !self.xdisp.echo_area_buffer[0].is_nil() {
                self.resize_echo_area_exactly()?;
            }
            if !self.plain_value(sym::DELAYED_WARNINGS_LIST).is_nil() {
                self.safe_run_hooks(sym::DELAYED_WARNINGS_HOOK)?;
            }
        }
        self.record_last_commands();

        loop {
            self.kill_emacs_unless_selected_frame_live()?;
            self.select_selected_window_buffer();
            self.set_plain_value(sym::DEACTIVATE_MARK, Value::NIL);
            self.keyboard.ignore_mouse_drag_p = false;
            if self.minibuf.minibuf_level != 0
                && !self.xdisp.echo_area_buffer[0].is_nil()
                && self.windows.minibuf_window == self.xdisp.echo_area_window
                && self.is_number(self.plain_value(sym::MINIBUFFER_MESSAGE_TIMEOUT))
            {
                unreachable!("the minibuffer message timeout needs resize_mini_window");
            }
            self.set_plain_value(sym::THIS_COMMAND, Value::NIL);
            self.set_plain_value(sym::REAL_THIS_COMMAND, Value::NIL);
            self.set_plain_value(sym::THIS_ORIGINAL_COMMAND, Value::NIL);
            self.set_plain_value(sym::THIS_COMMAND_KEYS_SHIFT_TRANSLATED, Value::NIL);

            self.init_raw_keybuf_count();
            let keys = self.read_key_sequence(Value::NIL, false, true, true, false)?;
            self.kill_emacs_unless_selected_frame_live()?;
            self.select_selected_window_buffer();
            let n = self.int_var(sym::NUM_INPUT_KEYS);
            self.set_forwarded_int(sym::NUM_INPUT_KEYS, n + 1);

            match keys {
                Some(keys) if keys.is_empty() => return Ok(Value::NIL),
                None => {
                    self.cancel_echoing();
                    self.keyboard.this_command_key_count = 0;
                    self.keyboard.this_single_command_key_start = 0;
                }
                Some(keys) => {
                    let last = keys[keys.len() - 1];
                    self.set_plain_value(sym::LAST_COMMAND_EVENT, last);
                    let sw = self.windows.selected_window;
                    if self.window(sw).flags.force_start {
                        self.window_mut(sw).flags.force_start = false;
                    }
                    let mut cmd = self.keyboard.read_key_sequence_cmd;
                    if !self.plain_value(sym::EXECUTING_KBD_MACRO).is_nil()
                        && !self.plain_value(sym::QUIT_FLAG).is_nil()
                    {
                        self.set_plain_value(sym::EXECUTING_KBD_MACRO, sym::T);
                        self.maybe_quit()?;
                    }
                    prev_buffer = self.current_buffer;
                    prev_modiff = self.cur().text.modiff;
                    self.keyboard.last_point_position = self.cur().pt;
                    let last_pt = self.cur().pt;
                    self.set_plain_value(sym::DISABLE_POINT_ADJUSTMENT, Value::NIL);
                    self.set_plain_value(sym::DEACTIVATE_MARK, Value::NIL);
                    self.set_plain_value(sym::THIS_ORIGINAL_COMMAND, cmd);
                    if !self.keyboard.read_key_sequence_remapped.is_nil() {
                        cmd = self.keyboard.read_key_sequence_remapped;
                    }
                    let limit = self.keyboard.lossage_limit;
                    if self.keyboard.total_keys < limit {
                        self.keyboard.total_keys += 1;
                    }
                    let entry = self.cons(Value::NIL, cmd);
                    self.set_recent_key(self.keyboard.recent_keys_index, entry);
                    self.keyboard.recent_keys_index += 1;
                    if self.keyboard.recent_keys_index >= limit {
                        self.keyboard.recent_keys_index = 0;
                    }
                    self.set_plain_value(sym::THIS_COMMAND, cmd);
                    self.set_plain_value(sym::REAL_THIS_COMMAND, cmd);

                    self.safe_run_hooks_maybe_narrowed(sym::PRE_COMMAND_HOOK)?;

                    if self.plain_value(sym::THIS_COMMAND).is_nil() {
                        self.call0(sym::UNDEFINED)?;
                    } else {
                        self.call0(sym::UNDO_AUTO__ADD_BOUNDARY)?;
                        self.point_before_last_command_or_undo = self.cur().pt;
                        self.buffer_before_last_command_or_undo = self.current_buffer;
                        self.update_redisplay_ticks(0, Value::NIL)?;
                        self.xdisp.display_working_on_window_p = false;
                        let this_command = self.plain_value(sym::THIS_COMMAND);
                        self.funcall(sym::COMMAND_EXECUTE, &[this_command])?;
                        self.xdisp.display_working_on_window_p = false;
                    }
                    self.keyboard.last_point_position = last_pt;
                    let current_prefix_arg = self.plain_value(sym::CURRENT_PREFIX_ARG);
                    self.set_kvar(Kvar::LastPrefixArg, current_prefix_arg);

                    self.safe_run_hooks_maybe_narrowed(sym::POST_COMMAND_HOOK)?;

                    let selected_frame = self.frames.selected_frame;
                    if !self.xdisp.echo_area_buffer[0].is_nil()
                        && self.xdisp.echo_area_window
                            == self.frame(selected_frame).minibuffer_window
                    {
                        self.resize_echo_area_exactly()?;
                    }
                    if !self.plain_value(sym::DELAYED_WARNINGS_LIST).is_nil() {
                        self.safe_run_hooks(sym::DELAYED_WARNINGS_HOOK)?;
                    }
                    self.record_last_commands();
                    self.keyboard.this_command_key_count = 0;
                    self.keyboard.this_single_command_key_start = 0;

                    if self.kboards[self.current_kboard].immediate_echo
                        && !self.call0(sym::INTERNAL_ECHO_KEYSTROKES_PREFIX)?.is_nil()
                    {
                        let kb = self.current_kboard;
                        self.kboards[kb].immediate_echo = false;
                        self.echo_now()?;
                    } else {
                        self.cancel_echoing();
                    }

                    if !self.plain_value(sym::MARK_ACTIVE).is_nil()
                        && self.var_true(sym::RUN_HOOKS_ENABLED)
                    {
                        self.update_active_region(prev_buffer, prev_modiff)?;
                    }
                }
            }

            let sw = self.windows.selected_window;
            let pt = self.cur().pt;
            let last_point_position = self.keyboard.last_point_position;
            if self.current_buffer == prev_buffer
                && self.window(sw).contents == self.current_buffer
                && last_point_position != pt
            {
                let (begv, zv) = (self.cur().begv, self.cur().zv);
                if self.plain_value(sym::DISABLE_POINT_ADJUSTMENT).is_nil()
                    && self
                        .plain_value(sym::GLOBAL_DISABLE_POINT_ADJUSTMENT)
                        .is_nil()
                    && !self.var_true(sym::COMPOSITION_BREAK_AT_POINT)
                {
                    if last_point_position > begv
                        && last_point_position < zv
                        && self
                            .composition_adjust_point(last_point_position, last_point_position)?
                            != last_point_position
                    {
                        self.windows.windows_or_buffers_changed = 21;
                    }
                    let modified = self.cur().text.modiff != prev_modiff;
                    self.adjust_point_for_property(last_point_position, modified)?;
                } else if pt > begv
                    && pt < zv
                    && self.composition_adjust_point(last_point_position, pt)? != pt
                {
                    self.windows.windows_or_buffers_changed = 39;
                }
            }

            if !self.kvar(Kvar::DefiningKbdMacro).is_nil() && self.kvar(Kvar::PrefixArg).is_nil() {
                self.finalize_kbd_macro_chars();
            }
        }
    }

    fn update_active_region(&mut self, prev_buffer: Value, prev_modiff: i64) -> LispResult<()> {
        let transient_mark_mode = self.plain_value(sym::TRANSIENT_MARK_MODE);
        if transient_mark_mode == sym::IDENTITY {
            self.set_plain_value(sym::TRANSIENT_MARK_MODE, Value::NIL);
        } else if transient_mark_mode == sym::ONLY {
            self.set_plain_value(sym::TRANSIENT_MARK_MODE, sym::IDENTITY);
        }
        if !self.plain_value(sym::DEACTIVATE_MARK).is_nil() {
            self.call0(sym::DEACTIVATE_MARK)?;
        } else {
            let selection_ok = !crate::frame::f_window_system(self, &[Value::NIL])?.is_nil() || {
                let symval = self.find_symbol_value(sym::TTY_SELECT_ACTIVE_REGIONS);
                symval != sym::UNBOUND
                    && !symval.is_nil()
                    && !crate::terminal::f_terminal_parameter(
                        self,
                        &[Value::NIL, sym::XTERM__SET_SELECTION],
                    )?
                    .is_nil()
            };
            let mark = self.cur().mark;
            let transient_mark_mode = self.plain_value(sym::TRANSIENT_MARK_MODE);
            let select_active_regions = self.plain_value(sym::SELECT_ACTIVE_REGIONS);
            let this_command = self.plain_value(sym::THIS_COMMAND);
            let inhibit_commands = self.plain_value(sym::SELECTION_INHIBIT_UPDATE_COMMANDS);
            if selection_ok
                && !self.marker(mark).buffer.is_nil()
                && (if select_active_regions == sym::ONLY {
                    self.car_safe(transient_mark_mode) == sym::ONLY
                } else {
                    !select_active_regions.is_nil() && !transient_mark_mode.is_nil()
                })
                && self.memq(this_command, inhibit_commands).is_nil()
            {
                let extract = self.plain_value(sym::REGION_EXTRACT_FUNCTION);
                let txt = self.funcall(extract, &[Value::NIL])?;
                self.stack.push(txt);
                let r = (|| -> LispResult<()> {
                    if crate::fns::f_length(self, &[txt])?.as_fixnum().unwrap_or(0) > 0 {
                        self.funcall(sym::GUI_SET_SELECTION, &[sym::PRIMARY, txt])?;
                    }
                    crate::eval::f_run_hook_with_args(self, &[sym::POST_SELECT_REGION_HOOK, txt])?;
                    Ok(())
                })();
                self.stack.pop();
                r?;
            }
            if self.current_buffer != prev_buffer || self.cur().text.modiff != prev_modiff {
                self.run_hook(sym::ACTIVATE_MARK_HOOK)?;
            }
        }
        self.set_plain_value(sym::SAVED_REGION_SELECTION, Value::NIL);
        Ok(())
    }

    fn invisible_run_end(&mut self, pos: usize, window: Value) -> LispResult<Option<(i64, bool)>> {
        let (val, overlay) =
            self.get_char_property_and_overlay(Value::fixnum(pos as i64), sym::INVISIBLE, window)?;
        if val.is_nil() {
            return Ok(None);
        }
        let inv = self.text_prop_means_invisible(val);
        if inv == 0 {
            return Ok(None);
        }
        let shows_strings = self.is_overlay(overlay)
            && (!crate::overlay::f_overlay_get(self, &[overlay, sym::AFTER_STRING])?.is_nil()
                || !crate::overlay::f_overlay_get(self, &[overlay, sym::BEFORE_STRING])?.is_nil());
        Ok(Some((inv, shows_strings)))
    }

    fn adjust_point_for_property(&mut self, last_pt: usize, modified: bool) -> LispResult<()> {
        let mut check_composition = !modified;
        let mut check_display = true;
        let mut check_invisible = true;
        let mut orig_pt = self.cur().pt as i64;
        let window = self.windows.selected_window;
        while check_composition || check_display || check_invisible {
            let (pt, begv, zv) = (self.cur().pt, self.cur().begv, self.cur().zv);
            if check_composition && pt > begv && pt < zv {
                let beg = self.composition_adjust_point(last_pt, pt)?;
                if beg != pt {
                    self.goto_char_internal(beg);
                    check_display = true;
                    check_invisible = true;
                }
            }
            check_composition = false;

            let (pt, begv, zv) = (self.cur().pt, self.cur().begv, self.cur().zv);
            if check_display && pt > begv && pt < zv {
                let (val, overlay) = self.get_char_property_and_overlay(
                    Value::fixnum(pt as i64),
                    sym::DISPLAY,
                    window,
                )?;
                if !val.is_nil() && self.display_prop_intangible_p(val, pt)? {
                    let range = if self.is_overlay(overlay) {
                        let ov = self.overlay(overlay);
                        Some((val, ov.start, ov.end))
                    } else {
                        self.get_property_and_range(pt as i64, sym::DISPLAY, Value::NIL)?
                    };
                    if let Some((val, beg, end)) = range {
                        let empty_string = val.is_string() && self.string_chars(val) == 0;
                        if beg < pt || (beg <= pt && empty_string) {
                            let target = if pt < last_pt {
                                if empty_string {
                                    beg.saturating_sub(1).max(begv)
                                } else {
                                    beg
                                }
                            } else {
                                end
                            };
                            self.goto_char_internal(target);
                            check_composition = true;
                            check_invisible = true;
                        }
                    }
                }
            }
            check_display = false;

            let pt_before_invis = self.cur().pt;
            let (pt, begv, zv) = (self.cur().pt, self.cur().begv, self.cur().zv);
            if check_invisible && pt > begv && pt < zv {
                let mut ellipsis = false;
                let (mut beg, mut end) = (pt, pt);
                while end < zv {
                    let Some((inv, shows_strings)) = self.invisible_run_end(end, window)? else {
                        break;
                    };
                    ellipsis = ellipsis || inv > 1 || shows_strings;
                    let tmp = crate::textprop::f_next_single_char_property_change(
                        self,
                        &[
                            Value::fixnum(end as i64),
                            sym::INVISIBLE,
                            Value::NIL,
                            Value::NIL,
                        ],
                    )?;
                    end = match tmp.as_fixnum() {
                        Some(n) if n >= 0 => n as usize,
                        _ => zv,
                    };
                }
                while beg > begv {
                    let Some((inv, shows_strings)) = self.invisible_run_end(beg - 1, window)?
                    else {
                        break;
                    };
                    ellipsis = ellipsis || inv > 1 || shows_strings;
                    let tmp = crate::textprop::f_previous_single_char_property_change(
                        self,
                        &[
                            Value::fixnum(beg as i64),
                            sym::INVISIBLE,
                            Value::NIL,
                            Value::NIL,
                        ],
                    )?;
                    beg = match tmp.as_fixnum() {
                        Some(n) if n >= 0 => n as usize,
                        _ => begv,
                    };
                }

                let pt = self.cur().pt;
                if beg < pt && end > pt {
                    let target = if orig_pt == pt as i64 && (last_pt < beg || last_pt > end) {
                        orig_pt = -1;
                        if pt < last_pt { end } else { beg }
                    } else if pt < last_pt {
                        beg
                    } else {
                        end
                    };
                    self.goto_char_internal(target);
                    check_composition = true;
                    check_display = true;
                }

                let (val, _) = self.get_char_property_and_overlay(
                    Value::fixnum(beg as i64),
                    sym::DISPLAY,
                    window,
                )?;
                let shown = !val.is_nil() && self.display_prop_intangible_p(val, beg)?;
                if shown {
                    self.goto_char_internal(pt_before_invis);
                }

                if !modified && !shown && !ellipsis && beg < end {
                    let pt = self.cur().pt;
                    if last_pt == beg && pt == end && end < zv {
                        check_composition = true;
                        check_display = true;
                        self.goto_char_internal(end + 1);
                    } else if last_pt == end && pt == beg && beg > begv {
                        check_composition = true;
                        check_display = true;
                        self.goto_char_internal(beg - 1);
                    } else if pt != (if pt < last_pt { beg } else { end }) {
                        let here = self.get_pos_property(
                            Value::fixnum(pt as i64),
                            sym::INVISIBLE,
                            window,
                        )?;
                        if self.text_prop_means_invisible(here) != 0 {
                            let other = if pt == beg { end } else { beg };
                            let there = self.get_pos_property(
                                Value::fixnum(other as i64),
                                sym::INVISIBLE,
                                window,
                            )?;
                            if self.text_prop_means_invisible(there) == 0 {
                                check_composition = true;
                                check_display = true;
                                self.goto_char_internal(other);
                            }
                        }
                    }
                }
            }
            check_invisible = false;
        }
        Ok(())
    }
}

fn recursive_edit_unwind(lisp: &mut Lisp, buffer: Value) -> LispResult<()> {
    if lisp.is_buffer(buffer) {
        crate::buffer::f_set_buffer(lisp, &[buffer])?;
    }
    lisp.minibuf.command_loop_level -= 1;
    lisp.windows.update_mode_lines = 18;
    Ok(())
}

fn user_error(lisp: &mut Lisp, msg: &str) -> crate::error::Flow {
    let message = lisp.string(msg);
    lisp.signal_list(sym::USER_ERROR, &[message])
}

fn f_recursive_edit(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    lisp.recursive_edit()
}

fn f_top_level(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Err(lisp.throw(sym::TOP_LEVEL, Value::NIL))
}

fn f_exit_recursive_edit(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    if lisp.minibuf.command_loop_level > 0 || lisp.minibuf.minibuf_level > 0 {
        return Err(lisp.throw(sym::EXIT, Value::NIL));
    }
    Err(user_error(lisp, "No recursive edit is in progress"))
}

fn f_abort_recursive_edit(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    if lisp.minibuf.command_loop_level > 0 || lisp.minibuf.minibuf_level > 0 {
        return Err(lisp.throw(sym::EXIT, sym::T));
    }
    Err(user_error(lisp, "No recursive edit is in progress"))
}
