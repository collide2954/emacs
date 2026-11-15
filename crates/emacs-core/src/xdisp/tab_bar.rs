use crate::bidi::BidiDir;
use crate::dispextern::{GlyphRowRef, It, TAB_BAR_ITEM_CAPTION, TAB_BAR_ITEM_NSLOTS, TEXT_AREA};
use crate::dispnew::matrix::{WhichMatrix, clear_glyph_matrix, clear_glyph_row};
use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;
use crate::xfaces::{DEFAULT_FACE_ID, TAB_BAR_FACE_ID};

impl Lisp {
    pub(crate) fn build_desired_tab_bar_string(&mut self, f: Value) -> LispResult<()> {
        let empty = self.string("");
        self.frame_mut(f).desired_tab_bar_string = empty;
        for i in 0..self.frame(f).n_tab_bar_items {
            let items = self.frame(f).tab_bar_items;
            let caption = self.vector_items(items)[i * TAB_BAR_ITEM_NSLOTS + TAB_BAR_ITEM_CAPTION];
            let caption = crate::fns::f_copy_sequence(self, &[caption])?;
            let base = self.stack.len();
            self.stack.push(caption);
            let index = Value::fixnum((i * TAB_BAR_ITEM_NSLOTS) as i64);
            let props = self.list(&[sym::MENU_ITEM, index]);
            let end = Value::fixnum(self.string_chars(caption) as i64);
            let r = crate::textprop::f_add_text_properties(
                self,
                &[Value::fixnum(0), end, props, caption],
            );
            if let Err(e) = r {
                self.stack.truncate(base);
                return Err(e);
            }
            let desired = self.frame(f).desired_tab_bar_string;
            let joined = crate::fns::f_concat(self, &[desired, caption]);
            self.stack.truncate(base);
            self.frame_mut(f).desired_tab_bar_string = joined?;
        }
        Ok(())
    }

    fn next_row(row: GlyphRowRef) -> GlyphRowRef {
        match row {
            GlyphRowRef::Window {
                window,
                desired,
                vpos,
            } => GlyphRowRef::Window {
                window,
                desired,
                vpos: vpos + 1,
            },
            GlyphRowRef::Scratch => GlyphRowRef::Scratch,
        }
    }

    fn display_tab_bar_line(&mut self, it: &mut It, height: i32) -> LispResult<()> {
        let row = it.glyph_row.expect("tab bar glyph row");
        let max_x = it.last_visible_x;
        {
            let r = self.glyph_row_mut(row);
            clear_glyph_row(r);
            r.enabled_p = true;
            r.y = it.current_y;
        }
        it.start_of_box_run_p = true;
        'out: {
            while it.current_x < max_x {
                if !self.get_next_display_element(it)? {
                    if height < 0 && it.hpos == 0 {
                        return Ok(());
                    }
                    break;
                }
                let n_glyphs_before = self.glyph_row(row).used[TEXT_AREA] as usize;
                let it_before = it.clone();
                self.produce_glyphs_macro(it)?;
                let nglyphs = self.glyph_row(row).used[TEXT_AREA] as usize - n_glyphs_before;
                let mut i = 0;
                let mut x = it_before.current_x;
                while i < nglyphs {
                    let pixel_width = {
                        let r = self.glyph_row(row);
                        i32::from(r.glyphs[r.area[TEXT_AREA] + n_glyphs_before + i].pixel_width)
                    };
                    if x + pixel_width > max_x {
                        self.glyph_row_mut(row).used[TEXT_AREA] = n_glyphs_before as _;
                        *it = it_before;
                        if n_glyphs_before == 0
                            && (it.vpos > 0 || it.current.string_pos.charpos < it.end_charpos - 1)
                        {
                            break;
                        }
                        break 'out;
                    }
                    it.hpos += 1;
                    x += pixel_width;
                    i += 1;
                }
                let enough = it.at_end_of_line_p();
                self.set_iterator_to_next(it, true)?;
                if enough {
                    break;
                }
            }
        }
        let displays_text_p = self.glyph_row(row).used[TEXT_AREA] != 0;
        self.glyph_row_mut(row).displays_text_p = displays_text_p;
        if !displays_text_p && self.find_symbol_value(sym::AUTO_RESIZE_TAB_BARS) != sym::GROW_ONLY {
            it.face_id = DEFAULT_FACE_ID as i32;
        }
        self.extend_face_to_end_of_line(it)?;
        {
            let r = self.glyph_row_mut(row);
            let used = r.used[TEXT_AREA] as usize;
            let start = r.area[TEXT_AREA];
            if used > 0 {
                r.glyphs[start + used - 1].right_box_line_p = true;
                if used == 1 {
                    r.glyphs[start].left_box_line_p = true;
                }
            }
        }
        let mut height = height - (it.max_ascent + it.max_descent);
        if height > 0 {
            height %= self.frame(it.f).geometry.line_height as i32;
            it.max_ascent += height / 2;
            it.max_descent += (height + 1) / 2;
        }
        self.compute_line_metrics(it);
        let last_visible_y = it.last_visible_y;
        {
            let r = self.glyph_row_mut(row);
            if !displays_text_p {
                r.height = last_visible_y - r.y;
                r.phys_height = r.height;
                r.visible_height = r.height;
                r.ascent = 0;
                r.phys_ascent = 0;
                r.extra_line_spacing = 0;
            }
            r.full_width_p = true;
            r.continued_p = false;
            r.truncated_on_left_p = false;
            r.truncated_on_right_p = false;
        }
        it.current_x = 0;
        it.hpos = 0;
        it.wrap_prefix_width = 0;
        it.current_y += self.glyph_row(row).height;
        it.vpos += 1;
        it.glyph_row = Some(Self::next_row(row));
        Ok(())
    }
}

impl Lisp {
    pub(crate) fn tab_bar_height(&mut self, f: Value, pixelwise: bool) -> LispResult<(i32, i32)> {
        self.with_gc_deferred(|lisp| lisp.tab_bar_height_1(f, pixelwise))
    }

    fn tab_bar_height_1(&mut self, f: Value, pixelwise: bool) -> LispResult<(i32, i32)> {
        let w = self.frame(f).tab_bar_window;
        let nrows = self
            .window_matrix(w, WhichMatrix::Desired)
            .map_or(1, |m| m.rows.len());
        let temp_row = GlyphRowRef::Window {
            window: w,
            desired: true,
            vpos: nrows - 1,
        };
        let mut it = It::default();
        self.init_iterator(&mut it, w, -1, -1, Some(temp_row), TAB_BAR_FACE_ID)?;
        self.glyph_row_mut(temp_row).reversed_p = false;
        it.first_visible_x = 0;
        it.last_visible_x = self.window(w).pixel_width as i32;
        let string = self.frame(f).desired_tab_bar_string;
        let multibyte = i32::from(self.string_multibyte(string));
        self.reseat_to_string(&mut it, None, string, 0, 0, 0, multibyte)?;
        it.paragraph_embedding = BidiDir::L2R;
        clear_glyph_row(self.glyph_row_mut(temp_row));
        if self.find_symbol_value(sym::TAB_BAR_TRUNCATE).is_nil() {
            while !it.at_end_p() {
                it.glyph_row = Some(temp_row);
                self.display_tab_bar_line(&mut it, -1)?;
            }
        } else {
            it.glyph_row = Some(temp_row);
            self.display_tab_bar_line(&mut it, -1)?;
        }
        clear_glyph_row(self.glyph_row_mut(temp_row));
        let n_rows = if it.vpos > 0 { it.vpos } else { -1 };
        let line_height = self.frame(f).geometry.line_height as i32;
        let height = if pixelwise {
            it.current_y
        } else {
            (it.current_y + line_height - 1) / line_height
        };
        Ok((height, n_rows))
    }

    pub(crate) fn redisplay_tab_bar(&mut self, f: Value) -> LispResult<bool> {
        self.frame_mut(f).flags.tab_bar_redisplayed = true;
        let w = self.frame(f).tab_bar_window;
        if !self.is_window(w) || self.window(w).total_lines == 0 {
            self.frame_mut(f).flags.tab_bar_resized = true;
            return Ok(false);
        }
        self.with_gc_deferred(|lisp| lisp.redisplay_tab_bar_1(f, w))
    }

    fn change_tab_bar_height(&mut self, f: Value, height: i32) -> LispResult<()> {
        if let Some(hook) = self.frame_terminal_hooks(f).change_tab_bar_height {
            hook(self, f, i64::from(height))?;
        }
        Ok(())
    }

    fn redisplay_tab_bar_1(&mut self, f: Value, w: Value) -> LispResult<bool> {
        self.build_desired_tab_bar_string(f)?;
        let (new_height, new_nrows) = self.tab_bar_height(f, true)?;
        let auto_resize = self.find_symbol_value(sym::AUTO_RESIZE_TAB_BARS);
        let grow_only = auto_resize == sym::GROW_ONLY;
        let window_height = self.window(w).pixel_height as i32;
        if self.frame(f).n_tab_bar_rows == 0 {
            self.frame_mut(f).n_tab_bar_rows = i64::from(new_nrows);
            if new_height != window_height {
                self.frames.default_tab_bar_height = i64::from(new_height);
            }
        }
        if i64::from(new_nrows) > self.frame(f).n_tab_bar_rows
            || (grow_only
                && !self.frame(f).flags.minimize_tab_bar_window_p
                && new_height > window_height)
            || (!grow_only && new_height < window_height)
        {
            self.change_tab_bar_height(f, new_height)?;
            if i64::from(new_nrows) != self.frame(f).n_tab_bar_rows {
                self.frame_mut(f).n_tab_bar_rows = i64::from(new_nrows);
            }
            if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                clear_glyph_matrix(m);
            }
            self.frame_mut(f).fonts_changed = true;
            return Ok(true);
        }
        let first = GlyphRowRef::Window {
            window: w,
            desired: true,
            vpos: 0,
        };
        let mut it = It::default();
        self.init_iterator(&mut it, w, -1, -1, Some(first), TAB_BAR_FACE_ID)?;
        it.first_visible_x = 0;
        it.last_visible_x = self.window(w).pixel_width as i32;
        self.glyph_row_mut(first).reversed_p = false;
        let string = self.frame(f).desired_tab_bar_string;
        let multibyte = i32::from(self.string_multibyte(string));
        self.reseat_to_string(&mut it, None, string, 0, 0, 0, multibyte)?;
        it.paragraph_embedding = BidiDir::L2R;
        if self.frame(f).n_tab_bar_rows > 0 {
            let border_value = self.find_symbol_value(sym::TAB_BAR_BORDER);
            let border = if let Some(n) = border_value
                .as_fixnum()
                .filter(|&n| (i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&n))
            {
                n
            } else if border_value == sym::INTERNAL_BORDER_WIDTH {
                self.frame_internal_border_width(f)
            } else if border_value == sym::BORDER_WIDTH {
                self.frame(f).geometry.border_width
            } else {
                0
            }
            .max(0) as i32;
            let mut rows = self.frame(f).n_tab_bar_rows as i32;
            let height = ((it.last_visible_y - border) / rows).max(1);
            let mut extra = it.last_visible_y - border - height * rows;
            while it.current_y < it.last_visible_y {
                let mut h = 0;
                if extra > 0 && rows > 0 {
                    rows -= 1;
                    h = (extra + rows - 1) / rows;
                    extra -= h;
                }
                self.display_tab_bar_line(&mut it, height + h)?;
            }
        } else {
            while it.current_y < it.last_visible_y {
                self.display_tab_bar_line(&mut it, 0)?;
            }
        }
        if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
            m.no_scrolling_p = true;
        }
        self.window_mut(w).flags.must_be_updated_p = true;
        if !auto_resize.is_nil() {
            let mut change_height_p = it.current.string_pos.charpos < it.end_charpos;
            let last = match it.glyph_row.expect("tab bar glyph row") {
                GlyphRowRef::Window {
                    window,
                    desired,
                    vpos,
                } => GlyphRowRef::Window {
                    window,
                    desired,
                    vpos: vpos - 1,
                },
                GlyphRowRef::Scratch => GlyphRowRef::Scratch,
            };
            let line_height = self.frame(f).geometry.line_height as i32;
            let (displays, height, bottom) = {
                let r = self.glyph_row(last);
                (r.displays_text_p, r.height, r.y + r.height)
            };
            if !displays && height >= line_height {
                change_height_p = true;
            }
            if displays && bottom > it.last_visible_y {
                change_height_p = true;
            }
            if change_height_p {
                let (new_height, nrows) = self.tab_bar_height(f, true)?;
                let window_height = self.window(w).pixel_height as i32;
                let change = if grow_only && !self.frame(f).flags.minimize_tab_bar_window_p {
                    new_height > window_height
                } else {
                    new_height != window_height
                };
                self.frame_mut(f).flags.minimize_tab_bar_window_p = false;
                if change {
                    self.change_tab_bar_height(f, new_height)?;
                    self.frames.default_tab_bar_height = i64::from(new_height);
                    if let Some(m) = self.window_matrix_mut(w, WhichMatrix::Desired) {
                        clear_glyph_matrix(m);
                    }
                    self.frame_mut(f).n_tab_bar_rows = i64::from(nrows);
                    self.frame_mut(f).fonts_changed = true;
                    return Ok(true);
                }
            }
        }
        self.frame_mut(f).flags.minimize_tab_bar_window_p = false;
        Ok(false)
    }
}
