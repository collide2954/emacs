use std::str::FromStr;

use emacs_core::composite::{
    LGLYPH_IX_ADJUSTMENT, LGLYPH_IX_ASCENT, LGLYPH_IX_CHAR, LGLYPH_IX_CODE, LGLYPH_IX_DESCENT,
    LGLYPH_IX_FROM, LGLYPH_IX_LBEARING, LGLYPH_IX_RBEARING, LGLYPH_IX_TO, LGLYPH_IX_WIDTH,
    LGLYPH_SIZE,
};
use emacs_core::error::LispResult;
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::value::Value;
use harfrust::{BufferClusterLevel, Direction, GlyphInfo, GlyphPosition, Language, UnicodeBuffer};

use crate::ftcrfont::{open_font_data, with_backend};

fn set(lisp: &mut Lisp, glyph: Value, idx: usize, val: Value) -> LispResult<()> {
    lisp.aset_value(glyph, Value::fixnum(idx as i64), val)?;
    Ok(())
}

fn reverse_clusters(glyphs: &mut [(GlyphInfo, GlyphPosition)]) {
    glyphs.reverse();
    let mut start = 0;
    while start < glyphs.len() {
        let cluster = glyphs[start].0.cluster;
        let mut end = start + 1;
        while end < glyphs.len() && glyphs[end].0.cluster == cluster {
            end += 1;
        }
        glyphs[start..end].reverse();
        start = end;
    }
}

pub fn hbfont_shape(lisp: &mut Lisp, lgstring: Value, direction: Value) -> LispResult {
    let font_object = lisp.lgstring_font(lgstring);
    if !lisp.font_object_p(font_object) {
        return Err(lisp.wrong_type(sym::FONT_OBJECT, font_object));
    }
    let mut text_len = lisp.lgstring_glyph_len(lgstring);
    let mut buffer = UnicodeBuffer::new();
    buffer.reserve(text_len);
    let mut chars = Vec::with_capacity(text_len);
    let mut i = 0;
    while i < text_len {
        let g = lisp.lgstring_glyph(lgstring, i);
        if g.is_nil() {
            break;
        }
        let c = lisp.lglyph_int(g, LGLYPH_IX_CHAR);
        let ch = char::from_u32(c as u32).unwrap_or(char::REPLACEMENT_CHARACTER);
        buffer.add(ch, i as u32);
        chars.push(c);
        i += 1;
    }
    text_len = i;
    if text_len == 0 {
        return Ok(Value::NIL);
    }
    buffer.set_cluster_level(BufferClusterLevel::MonotoneGraphemes);
    if !direction.is_nil()
        && !lisp
            .find_symbol_value(sym::BIDI_DISPLAY_REORDERING)
            .is_nil()
    {
        buffer.set_direction(if direction == sym::R2L {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        });
    }
    let mut lang = lisp.find_symbol_value(sym::CURRENT_ISO639_LANGUAGE);
    if lang.is_cons() {
        lang = lisp.xcar(lang);
    }
    if lang.is_symbol() {
        let name = lisp.symbol_name_string(lang);
        if let Ok(language) = Language::from_str(&name) {
            buffer.set_language(language);
        }
    }
    buffer.guess_segment_properties();
    let buffer_direction = buffer.direction();
    let Some(open) = open_font_data(lisp, font_object) else {
        return Ok(Value::fixnum(0));
    };
    let (shaped, position_unit) = with_backend(|backend| backend.shape(&open.borrow(), buffer));
    let glyph_len = shaped.len();
    if glyph_len > lisp.lgstring_glyph_len(lgstring) {
        return Ok(Value::fixnum(-(glyph_len as i64)));
    }
    let mut glyphs: Vec<(GlyphInfo, GlyphPosition)> = shaped
        .glyph_infos()
        .iter()
        .copied()
        .zip(shaped.glyph_positions().iter().copied())
        .collect();
    let buf_reversed = matches!(
        buffer_direction,
        Direction::RightToLeft | Direction::BottomToTop
    );
    if buf_reversed {
        reverse_clusters(&mut glyphs);
    }
    let Some(driver) = lisp.font_object(font_object).driver else {
        return Ok(Value::fixnum(0));
    };
    let text_len = text_len as i64;
    let (mut from, mut to, mut cluster_offset) = (-1i64, 0i64, 0i64);
    let incr = if buf_reversed { -1 } else { 1 };
    for (i, &(info, pos)) in glyphs.iter().enumerate() {
        let mut lglyph = lisp.lgstring_glyph(lgstring, i);
        let mut new_lglyph = false;
        if lglyph.is_nil() {
            new_lglyph = true;
            lglyph = lisp.vector(vec![Value::NIL; LGLYPH_SIZE]);
            set(lisp, lgstring, i + 2, lglyph)?;
        }
        if i64::from(info.cluster) != from {
            from = i64::from(info.cluster);
            let mut j = i;
            while j < glyph_len && i64::from(glyphs[j].0.cluster) == from {
                j += 1;
            }
            to = if j == glyph_len {
                text_len - 1
            } else {
                i64::from(glyphs[j].0.cluster) - 1
            };
            cluster_offset = 0;
            if buf_reversed {
                cluster_offset = to - from;
            }
        }
        set(lisp, lglyph, LGLYPH_IX_FROM, Value::fixnum(from))?;
        if !(!new_lglyph && to == text_len - 1 && lisp.lglyph_int(lglyph, LGLYPH_IX_TO) > to) {
            set(lisp, lglyph, LGLYPH_IX_TO, Value::fixnum(to))?;
        }
        let char_idx = (from + cluster_offset).clamp(from, to.max(from));
        cluster_offset += incr;
        set(
            lisp,
            lglyph,
            LGLYPH_IX_CHAR,
            Value::fixnum(chars[char_idx as usize]),
        )?;
        let code = info.glyph_id;
        set(lisp, lglyph, LGLYPH_IX_CODE, Value::fixnum(i64::from(code)))?;
        let metrics = (driver.text_extents)(lisp, font_object, &[code]);
        for (idx, v) in [
            (LGLYPH_IX_WIDTH, metrics.width),
            (LGLYPH_IX_LBEARING, metrics.lbearing),
            (LGLYPH_IX_RBEARING, metrics.rbearing),
            (LGLYPH_IX_ASCENT, metrics.ascent),
            (LGLYPH_IX_DESCENT, metrics.descent),
        ] {
            set(lisp, lglyph, idx, Value::fixnum(i64::from(v)))?;
        }
        let xoff = (f64::from(pos.x_offset) * position_unit).round() as i64;
        let yoff = -(f64::from(pos.y_offset) * position_unit).round() as i64;
        let wadjust = (f64::from(pos.x_advance) * position_unit).round() as i64;
        if xoff != 0 || yoff != 0 || wadjust != i64::from(metrics.width) {
            let adjustment = lisp.vector(vec![
                Value::fixnum(xoff),
                Value::fixnum(yoff),
                Value::fixnum(wadjust),
            ]);
            set(lisp, lglyph, LGLYPH_IX_ADJUSTMENT, adjustment)?;
        }
    }
    Ok(Value::fixnum(glyph_len as i64))
}

pub fn hbfont_combining_capability(_lisp: &mut Lisp, _font: Value) -> Value {
    sym::T
}
