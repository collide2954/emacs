use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

impl Lisp {
    fn window_buffer_changed(&self, w: Value) -> bool {
        let b = self.buffer(self.window(w).contents);
        (b.text.save_modiff < b.text.modiff) != self.window(w).flags.last_had_star
    }

    pub(crate) fn prepare_menu_bars(&mut self) -> LispResult<()> {
        let all_windows =
            self.windows.windows_or_buffers_changed != 0 || self.windows.update_mode_lines != 0;
        let some_windows = self.redisplay_some_p();
        let function = self.find_symbol_value(sym::PRE_REDISPLAY_FUNCTION);
        if self.functionp(function) {
            let mut windows = if all_windows { sym::T } else { Value::NIL };
            if all_windows && some_windows {
                let mut ws = self.window_list()?;
                windows = Value::NIL;
                while ws.is_cons() {
                    let this = self.xcar(ws);
                    if !self.needs_no_redisplay(this) {
                        windows = self.cons(this, windows);
                    }
                    ws = self.xcdr(ws);
                }
            }
            self.safe_call(true, &[function, windows])?;
        }
        if all_windows {
            for f in self.frames.list.clone() {
                if some_windows && self.frame_selected_window_unchanged(f) {
                    continue;
                }
                let fl = &self.frame(f).flags;
                if !fl.tooltip
                    && (fl.iconified
                        || fl.visible
                        || (self.frame_redisplay_p(f) && self.frame_window_p(f)))
                {
                    self.gui_consider_frame_title(f)?;
                }
            }
        }
        if all_windows {
            let count = self.specpdl_index();
            let mut menu_bar_hooks_run = false;
            self.record_unwind_save_match_data()?;
            for f in self.frames.list.clone() {
                let w = self.frame(f).selected_window;
                if self.frame(f).flags.tooltip {
                    continue;
                }
                if some_windows && self.frame_selected_window_unchanged(f) {
                    continue;
                }
                menu_bar_hooks_run = self.update_menu_bar(f, false, menu_bar_hooks_run, w)?;
                self.update_tab_bar(f)?;
                self.update_tool_bar(f)?;
            }
            self.unbind_to(count)
        } else {
            let sw = self.windows.selected_window;
            let sf = self.window(sw).frame;
            self.update_menu_bar(sf, true, false, sw)?;
            self.update_tab_bar(sf)?;
            self.update_tool_bar(sf)
        }
    }

    fn frame_selected_window_unchanged(&self, f: Value) -> bool {
        let w = self.frame(f).selected_window;
        !self.frame(f).flags.redisplay
            && !self.window(w).flags.redisplay
            && !self.buffer(self.window(w).contents).text.redisplay
    }

    fn update_menu_bar(
        &mut self,
        f: Value,
        save_match_data: bool,
        hooks_run: bool,
        w: Value,
    ) -> LispResult<bool> {
        let mut hooks_run = hooks_run;
        if self.var_true(sym::INHIBIT_MENUBAR_UPDATE) {
            return Ok(hooks_run);
        }
        let wanted = if self.frame_window_p(f) {
            self.frame(f).flags.external_menu_bar
        } else {
            self.frame(f).geometry.menu_bar_lines > 0
        };
        if wanted
            && (self.windows.windows_or_buffers_changed != 0
                || self.windows.update_mode_lines != 0
                || self.window_buffer_changed(w))
        {
            let prev = self.current_buffer;
            let count = self.specpdl_index();
            self.specbind(sym::INHIBIT_MENUBAR_UPDATE, sym::T)?;
            let contents = self.window(w).contents;
            self.set_buffer_internal(contents);
            if save_match_data {
                self.record_unwind_save_match_data()?;
            }
            if self
                .find_symbol_value(sym::OVERRIDING_LOCAL_MAP_MENU_FLAG)
                .is_nil()
            {
                self.specbind(sym::OVERRIDING_TERMINAL_LOCAL_MAP, Value::NIL)?;
                self.specbind(sym::OVERRIDING_LOCAL_MAP, Value::NIL)?;
            }
            if !hooks_run {
                self.safe_run_hooks(sym::ACTIVATE_MENUBAR_HOOK)?;
                self.safe_run_hooks(sym::MENU_BAR_UPDATE_HOOK)?;
                hooks_run = true;
            }
            self.set_plain_value(sym::MENU_UPDATING_FRAME, f);
            let old = self.frame(f).menu_bar_items;
            let items = self.menu_bar_items(old)?;
            self.frame_mut(f).menu_bar_items = items;
            if self.frame_window_p(f) {
                unreachable!("external menu bars need set_frame_menubar");
            }
            self.window_mut(w).flags.update_mode_line = true;
            self.unbind_to(count)?;
            self.set_buffer_internal(prev);
        }
        Ok(hooks_run)
    }

    fn bar_needs_update(&self, f: Value) -> bool {
        let w = self.frame(f).selected_window;
        self.windows.windows_or_buffers_changed != 0
            || self.window(w).flags.update_mode_line
            || self.windows.update_mode_lines != 0
            || self.window_buffer_changed(w)
    }

    fn update_tab_bar(&mut self, f: Value) -> LispResult<()> {
        let tab_bar_window = self.frame(f).tab_bar_window;
        let do_update = if self.frame_window_p(f) && self.is_window(tab_bar_window) {
            self.window(tab_bar_window).total_lines > 0
        } else {
            self.frame(f).geometry.tab_bar_lines > 0
        };
        if do_update && self.bar_needs_update(f) {
            unreachable!("update_tab_bar needs tab_bar_items");
        }
        Ok(())
    }

    fn update_tool_bar(&mut self, f: Value) -> LispResult<()> {
        if self.frame(f).flags.external_tool_bar && self.bar_needs_update(f) {
            unreachable!("update_tool_bar needs tool_bar_items");
        }
        Ok(())
    }
}
