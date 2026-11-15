use crate::bidi::BidiDir;
use crate::dispextern::{
    DrawGlyphsFace, GlyphRowRef, It, TAB_BAR_ITEM_BINDING, TAB_BAR_ITEM_CAPTION,
    TAB_BAR_ITEM_ENABLED_P, TAB_BAR_ITEM_HELP, TAB_BAR_ITEM_KEY, TAB_BAR_ITEM_NSLOTS, TEXT_AREA,
};
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

pub(crate) struct TabBarHit {
    hpos: usize,
    vpos: usize,
    glyph: crate::dispextern::Glyph,
    prop_idx: usize,
    close_p: bool,
    same_p: bool,
}

impl Lisp {
    fn tab_bar_item_info(&mut self, f: Value, charpos: i64) -> LispResult<Option<(usize, bool)>> {
        let string = self.frame(f).current_tab_bar_string;
        if !string.is_string() {
            return Ok(None);
        }
        let charpos = Value::fixnum(charpos.min(self.string_chars(string) as i64).max(0));
        let prop = crate::textprop::f_get_text_property(self, &[charpos, sym::MENU_ITEM, string])?;
        let Some(prop_idx) = prop.as_fixnum() else {
            return Ok(None);
        };
        let close = crate::textprop::f_get_text_property(self, &[charpos, sym::CLOSE_TAB, string])?;
        Ok(Some((prop_idx as usize, !close.is_nil())))
    }

    fn get_tab_bar_item(&mut self, f: Value, x: i64, y: i64) -> LispResult<Option<TabBarHit>> {
        let w = self.frame(f).tab_bar_window;
        let hit = self.x_y_to_hpos_vpos(w, x, y);
        let Some(glyph) = hit.glyph else {
            return Ok(None);
        };
        let Some((prop_idx, close_p)) = self.tab_bar_item_info(f, glyph.charpos)? else {
            return Ok(None);
        };
        Ok(Some(TabBarHit {
            hpos: hit.hpos,
            vpos: hit.vpos,
            glyph,
            prop_idx,
            close_p,
            same_p: prop_idx as i64 == self.frame(f).last_tab_bar_item,
        }))
    }

    fn tab_bar_window_xy(&self, f: Value, x: i64, y: i64) -> (i64, i64) {
        let w = self.frame(f).tab_bar_window;
        (
            x - self.frame_internal_border_width(f),
            self.frame_to_window_pixel_y(w, y),
        )
    }

    pub fn handle_tab_bar_click(
        &mut self,
        f: Value,
        x: i64,
        y: i64,
        down_p: bool,
    ) -> LispResult<Value> {
        let (x, y) = self.tab_bar_window_xy(f, x, y);
        let Some(hit) = self.get_tab_bar_item(f, x, y)? else {
            return Ok(self.cons(sym::TAB_BAR, Value::NIL));
        };
        let items = self.frame(f).tab_bar_items;
        let slot = |lisp: &Lisp, idx: usize| lisp.vector_items(items)[hit.prop_idx + idx];
        if slot(self, TAB_BAR_ITEM_ENABLED_P).is_nil() {
            return Ok(Value::NIL);
        }
        let mouse_highlight = !self.find_symbol_value(sym::MOUSE_HIGHLIGHT).is_nil();
        let draw = match (down_p, hit.close_p) {
            (true, true) => DrawGlyphsFace::ImageSunken,
            (false, true) => DrawGlyphsFace::ImageRaised,
            (_, false) => DrawGlyphsFace::MouseFace,
        };
        if mouse_highlight {
            self.show_mouse_face(f, draw, true)?;
        }
        self.frame_mut(f).last_tab_bar_item = if down_p { hit.prop_idx as i64 } else { -1 };
        let caption = slot(self, TAB_BAR_ITEM_CAPTION);
        let caption = crate::fns::f_copy_sequence(self, &[caption])?;
        let base = self.stack.len();
        self.stack.push(caption);
        let (key, binding) = (
            slot(self, TAB_BAR_ITEM_KEY),
            slot(self, TAB_BAR_ITEM_BINDING),
        );
        let close = if hit.close_p { sym::T } else { Value::NIL };
        let item = self.list(&[key, binding, close]);
        let props = self.list(&[sym::MENU_ITEM, item]);
        let end = Value::fixnum(self.string_chars(caption) as i64);
        let r =
            crate::textprop::f_add_text_properties(self, &[Value::fixnum(0), end, props, caption]);
        self.stack.truncate(base);
        r?;
        let object = self.cons(caption, Value::fixnum(0));
        Ok(self.cons(sym::TAB_BAR, object))
    }

    pub(crate) fn note_tab_bar_highlight(&mut self, f: Value, x: i64, y: i64) -> LispResult<()> {
        let window = self.frame(f).tab_bar_window;
        if x <= 0 || y <= 0 {
            self.clear_mouse_face(f)?;
            return Ok(());
        }
        let Some(hit) = self.get_tab_bar_item(f, x, y)? else {
            self.clear_mouse_face(f)?;
            return Ok(());
        };
        let items = self.frame(f).tab_bar_items;
        if !hit.same_p {
            self.clear_mouse_face(f)?;
            let terminal = self.frame(f).terminal;
            let mouse_down_p = self.gui_mouse_grabbed(terminal) == Some(f);
            let last = self.frame(f).last_tab_bar_item;
            if mouse_down_p && last != hit.prop_idx as i64 && last != -1 {
                return Ok(());
            }
            let mut draw = if mouse_down_p {
                DrawGlyphsFace::ImageSunken
            } else {
                DrawGlyphsFace::ImageRaised
            };
            let enabled_p = self.vector_items(items)[hit.prop_idx + TAB_BAR_ITEM_ENABLED_P];
            if !enabled_p.is_nil() && !self.find_symbol_value(sym::MOUSE_HIGHLIGHT).is_nil() {
                let glyphs: Vec<crate::dispextern::Glyph> = self
                    .window_matrix(window, WhichMatrix::Current)
                    .expect("tab bar matrix")
                    .rows[hit.vpos]
                    .area_glyphs(TEXT_AREA)
                    .to_vec();
                let string = self.vector_items(items)[hit.prop_idx + TAB_BAR_ITEM_CAPTION];
                let mut mouse_face_id = None;
                let mut hpos0 = hit.hpos + 1;
                let mut hpos_caption = 0;
                if string.is_string() {
                    let mut i = hit.hpos as isize;
                    while i >= 0 {
                        match self.tab_bar_item_info(f, glyphs[i as usize].charpos)? {
                            Some((idx, _)) if idx == hit.prop_idx => hpos0 -= 1,
                            _ => break,
                        }
                        i -= 1;
                    }
                    hpos_caption = hit.hpos - hpos0;
                    let mouse_face = crate::textprop::f_get_text_property(
                        self,
                        &[Value::fixnum(hpos_caption as i64), sym::MOUSE_FACE, string],
                    )?;
                    if !mouse_face.is_nil() {
                        let id = self.lookup_named_face(Some(window), f, mouse_face, false)?;
                        mouse_face_id = Some(if id < 0 {
                            self.compute_char_face(f, i32::from(b' '), mouse_face)?
                        } else {
                            id as usize
                        });
                        draw = DrawGlyphsFace::MouseFace;
                    }
                }
                let hlinfo = *self.mouse_hl_info(f);
                if draw == DrawGlyphsFace::MouseFace {
                    let b = crate::textprop::f_previous_single_property_change(
                        self,
                        &[
                            Value::fixnum(hpos_caption as i64 + 1),
                            sym::MOUSE_FACE,
                            string,
                            Value::NIL,
                        ],
                    )?;
                    let begpos = b.as_fixnum().unwrap_or(0) as usize;
                    let e = crate::textprop::f_next_single_property_change(
                        self,
                        &[
                            Value::fixnum(begpos as i64),
                            sym::MOUSE_FACE,
                            string,
                            Value::NIL,
                        ],
                    )?;
                    let endpos = e
                        .as_fixnum()
                        .map_or(self.string_chars(string), |n| n as usize);
                    let width = |from: usize, to: usize| -> i32 {
                        glyphs[from..to]
                            .iter()
                            .map(|g| i32::from(g.pixel_width))
                            .sum()
                    };
                    let beg_x = width(0, hpos0 + begpos);
                    let end_x = width(hpos0 + begpos, hpos0 + endpos);
                    if window == hlinfo.mouse_face_window
                        && hlinfo.mouse_face_beg_col <= hit.hpos as i32
                        && (hit.hpos as i32) < hlinfo.mouse_face_end_col
                        && hlinfo.mouse_face_beg_row == hit.vpos as i32
                    {
                        return Ok(());
                    }
                    let hl = self.mouse_hl_info_mut(f);
                    hl.mouse_face_window = window;
                    hl.mouse_face_face_id = mouse_face_id.unwrap_or(0);
                    hl.mouse_face_beg_row = hit.vpos as i32;
                    hl.mouse_face_end_row = hit.vpos as i32;
                    hl.mouse_face_past_end = false;
                    hl.mouse_face_beg_col = (hpos0 + begpos) as i32;
                    hl.mouse_face_end_col = (hpos0 + endpos) as i32;
                    hl.mouse_face_beg_x = beg_x;
                    hl.mouse_face_end_x = end_x;
                } else {
                    let x: i32 = glyphs[..hit.hpos]
                        .iter()
                        .map(|g| i32::from(g.pixel_width))
                        .sum();
                    let total_pixel_width = i32::from(hit.glyph.pixel_width);
                    let hl = self.mouse_hl_info_mut(f);
                    hl.mouse_face_face_id = TAB_BAR_FACE_ID;
                    hl.mouse_face_beg_col = hit.hpos as i32;
                    hl.mouse_face_beg_row = hit.vpos as i32;
                    hl.mouse_face_beg_x = x;
                    hl.mouse_face_past_end = false;
                    hl.mouse_face_end_col = hit.hpos as i32 + 1;
                    hl.mouse_face_end_row = hit.vpos as i32;
                    hl.mouse_face_end_x = x + total_pixel_width;
                    hl.mouse_face_window = window;
                }
                self.show_mouse_face(f, draw, true)?;
            }
        }
        let help = self.vector_items(items)[hit.prop_idx + TAB_BAR_ITEM_HELP];
        let x = &mut self.xdisp;
        x.help_echo_object = Value::NIL;
        x.help_echo_window = Value::NIL;
        x.help_echo_pos = -1;
        x.help_echo_string = help;
        Ok(())
    }
}
