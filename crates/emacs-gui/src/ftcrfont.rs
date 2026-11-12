use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use emacs_core::dispextern::{DrawGlyphsFace, GlyphString};
use emacs_core::error::LispResult;
use emacs_core::fns::f_copy_sequence;
use emacs_core::font::{
    FONT_ADSTYLE_INDEX, FONT_AVGWIDTH_INDEX, FONT_DPI_INDEX, FONT_EXTRA_INDEX, FONT_FAMILY_INDEX,
    FONT_FILE_INDEX, FONT_FOUNDRY_INDEX, FONT_INVALID_CODE, FONT_OBJLIST_INDEX,
    FONT_REGISTRY_INDEX, FONT_SIZE_INDEX, FONT_SLANT_INDEX, FONT_SPACING_INDEX, FONT_TYPE_INDEX,
    FONT_WEIGHT_INDEX, FONT_WIDTH_INDEX, FontDriver, FontMetrics,
};
use emacs_core::lisp::Lisp;
use emacs_core::object::Weakness;
use emacs_core::symbols as sym;
use emacs_core::value::Value;

use crate::font::{
    Background, DrawGlyphs, FontBackend, FontEntity, FontQuery, OpenFont, SPACING_MONO,
};

const FC_CHARSET_TABLE: &[(&str, &[u32], Option<&str>)] = &[
    ("iso8859-1", &[0x00A0, 0x00A1, 0x00B4, 0x00BC, 0x00D0], None),
    ("iso8859-2", &[0x00A0, 0x010E], None),
    ("iso8859-3", &[0x00A0, 0x0108], None),
    ("iso8859-4", &[0x00A0, 0x00AF, 0x0128, 0x0156, 0x02C7], None),
    ("iso8859-5", &[0x00A0, 0x0401], None),
    ("iso8859-6", &[0x00A0, 0x060C], None),
    ("iso8859-7", &[0x00A0, 0x0384], None),
    ("iso8859-8", &[0x00A0, 0x05D0], None),
    ("iso8859-9", &[0x00A0, 0x00A1, 0x00BC, 0x011E], None),
    ("iso8859-10", &[0x00A0, 0x00D0, 0x0128, 0x2015], None),
    ("iso8859-11", &[0x00A0, 0x0E01], None),
    ("iso8859-13", &[0x00A0, 0x201C], None),
    ("iso8859-14", &[0x00A0, 0x0174], None),
    ("iso8859-15", &[0x00A0, 0x00A1, 0x00D0, 0x0152], None),
    ("iso8859-16", &[0x00A0, 0x0218], None),
    ("gb2312.1980-0", &[0x4E13], Some("zh-cn")),
    ("big5-0", &[0x9C21], Some("zh-tw")),
    ("jisx0208.1983-0", &[0x4E55], Some("ja")),
    ("ksc5601.1985-0", &[0xAC00], Some("ko")),
    ("cns11643.1992-1", &[0xFE32], Some("zh-tw")),
    ("cns11643.1992-2", &[0x4E33, 0x7934], None),
    ("cns11643.1992-3", &[0x201A9], None),
    ("cns11643.1992-4", &[0x20057], None),
    ("cns11643.1992-5", &[0x20000], None),
    ("cns11643.1992-6", &[0x20003], None),
    ("cns11643.1992-7", &[0x20055], None),
    ("gbk-0", &[0x4E06], Some("zh-cn")),
    ("jisx0212.1990-0", &[0x4E44], None),
    ("jisx0213.2000-1", &[0xFA10], Some("ja")),
    ("jisx0213.2000-2", &[0xFA49], None),
    ("jisx0213.2004-1", &[0x20B9F], None),
    ("viscii1.1-1", &[0x1EA0, 0x1EAE, 0x1ED2], Some("vi")),
    ("tis620.2529-1", &[0x0E01], Some("th")),
    ("microsoft-cp1251", &[0x0401, 0x0490], Some("ru")),
    ("koi8-r", &[0x0401, 0x2219], Some("ru")),
    ("mulelao-1", &[0x0E81], Some("lo")),
    ("unicode-sip", &[0x20000], None),
];

const FTFONT_BOOLEANS: &[&str] = &[
    ":antialias",
    ":hinting",
    ":verticallayout",
    ":autohint",
    ":globaladvance",
    ":outline",
    ":scalable",
    ":minspace",
    ":embolden",
];

const FTFONT_NON_BOOLEANS: &[&str] = &[
    ":family",
    ":style",
    ":slant",
    ":weight",
    ":size",
    ":width",
    ":aspect",
    ":pixelsize",
    ":spacing",
    ":foundry",
    ":hintstyle",
    ":file",
    ":index",
    ":ftface",
    ":rasterizer",
    ":scale",
    ":dpi",
    ":rgba",
    ":lcdfilter",
    ":charset",
    ":lang",
    ":fontversion",
    ":capability",
];

struct Ftfont {
    backend: Option<FontBackend>,
    freetype_font_cache: Value,
    generic_family_list: Value,
    face_cache: Value,
}

thread_local! {
    static FTFONT: RefCell<Ftfont> = const {
        RefCell::new(Ftfont {
            backend: None,
            freetype_font_cache: Value::NIL,
            generic_family_list: Value::NIL,
            face_cache: Value::NIL,
        })
    };
}

pub(crate) fn with_backend<T>(f: impl FnOnce(&mut FontBackend) -> T) -> T {
    FTFONT.with(|ft| f(ft.borrow_mut().backend.get_or_insert_with(FontBackend::new)))
}

pub fn mark_ftfont(roots: &mut Vec<Value>) {
    FTFONT.with(|ft| {
        let ft = ft.borrow();
        roots.extend([
            ft.freetype_font_cache,
            ft.generic_family_list,
            ft.face_cache,
        ]);
    });
}

pub fn syms_of_ftcrfont(lisp: &mut Lisp) {
    let cache = lisp.list(&[sym::T]);
    let monospace = lisp.intern("monospace");
    let sans_serif = lisp.intern("sans-serif");
    let sans = lisp.intern("sans");
    let generic = [monospace, sans_serif, sans].map(|family| lisp.cons(family, sym::T));
    let generic = lisp.list(&generic);
    FTFONT.with(|ft| {
        let mut ft = ft.borrow_mut();
        ft.freetype_font_cache = cache;
        ft.generic_family_list = generic;
        ft.face_cache = Value::NIL;
    });
    let driver = ftcrfont_driver(lisp);
    lisp.register_font_driver(driver, None)
        .expect("register the ftcr font driver");
    let (ftcr, ftcrhb) = (lisp.intern("ftcr"), lisp.intern("ftcrhb"));
    lisp.put(ftcr, sym::FONT_DRIVER_SUPERSEDED_BY, ftcrhb);
    let driver = ftcrhbfont_driver(lisp);
    lisp.register_font_driver(driver, None)
        .expect("register the ftcrhb font driver");
}

pub fn ftcrhbfont_driver(lisp: &mut Lisp) -> FontDriver {
    FontDriver {
        type_: lisp.intern("ftcrhb"),
        list: ftcrhbfont_list,
        match_font: ftcrhbfont_match,
        shape: Some(crate::hbfont::hbfont_shape),
        combining_capability: Some(crate::hbfont::hbfont_combining_capability),
        ..ftcrfont_driver(lisp)
    }
}

pub fn ftcrfont_driver(lisp: &mut Lisp) -> FontDriver {
    FontDriver {
        type_: lisp.intern("ftcr"),
        case_sensitive: false,
        get_cache: ftfont_get_cache,
        list: ftcrfont_list,
        match_font: ftcrfont_match,
        list_family: Some(ftfont_list_family),
        free_entity: None,
        open_font: ftcrfont_open,
        close_font: ftcrfont_close,
        prepare_face: None,
        done_face: None,
        has_char: Some(ftcrfont_has_char),
        encode_char: ftcrfont_encode_char,
        text_extents: ftcrfont_text_extents,
        draw: Some(ftcrfont_draw),
        otf_capability: None,
        shape: None,
        start_for_frame: None,
        end_for_frame: None,
        filter_properties: Some(ftfont_filter_properties),
        cached_font_ok: None,
        combining_capability: Some(ftfont_combining_capability),
    }
}

fn ftfont_get_cache(_lisp: &mut Lisp, _f: Value) -> Value {
    FTFONT.with(|ft| ft.borrow().freetype_font_cache)
}

fn same_family(a: &str, b: &str) -> bool {
    let squeeze = |s: &str| -> Vec<char> {
        s.chars()
            .filter(|c| *c != ' ')
            .flat_map(char::to_lowercase)
            .collect()
    };
    squeeze(a) == squeeze(b)
}

fn ftfont_lookup_cache(lisp: &mut Lisp, key: Value) -> LispResult {
    let mut table = FTFONT.with(|ft| ft.borrow().face_cache);
    let cache = if table.is_nil() {
        Value::NIL
    } else {
        lisp.gethash(key, table, Value::NIL)?
    };
    if !cache.is_nil() {
        return Ok(cache);
    }
    if table.is_nil() {
        table = lisp.make_hash_table_with(sym::EQUAL, Weakness::None);
        FTFONT.with(|ft| ft.borrow_mut().face_cache = table);
    }
    let cache = lisp.cons(Value::NIL, Value::NIL);
    lisp.puthash(key, cache, table)?;
    Ok(cache)
}

fn entity_key(lisp: &Lisp, entity: Value) -> Option<(PathBuf, u32)> {
    let extra = lisp.font_aref(entity, FONT_EXTRA_INDEX);
    let val = lisp.assq(sym::K_FONT_ENTITY, extra);
    if !val.is_cons() {
        return None;
    }
    let key = lisp.xcdr(val);
    let file = lisp.xcar(key);
    let index = lisp.xcdr(key).as_fixnum()?;
    let path = PathBuf::from(String::from_utf8_lossy(lisp.string_bytes(file)).into_owned());
    Some((path, index as u32))
}

fn entity_face(lisp: &Lisp, entity: Value) -> Option<FontEntity> {
    let (path, index) = entity_key(lisp, entity)?;
    with_backend(|backend| backend.face_by_file(&path, index))
}

fn ftfont_pattern_entity(lisp: &mut Lisp, face: &FontEntity, extra: Value) -> LispResult {
    let Some(file) = &face.file else {
        return Ok(Value::NIL);
    };
    let file = lisp.unibyte_string(file.to_string_lossy().as_bytes());
    let key = lisp.cons(file, Value::fixnum(i64::from(face.index)));
    let base = lisp.stack.len();
    lisp.stack.push(key);
    let r = ftfont_pattern_entity_1(lisp, face, extra, key);
    lisp.stack.truncate(base);
    r
}

fn ftfont_pattern_entity_1(
    lisp: &mut Lisp,
    face: &FontEntity,
    extra: Value,
    key: Value,
) -> LispResult {
    let cache = ftfont_lookup_cache(lisp, key)?;
    let entity = lisp.xcar(cache);
    if !entity.is_nil() {
        let val = lisp.font_make_entity();
        for i in 0..FONT_OBJLIST_INDEX {
            let v = lisp.font_aref(entity, i);
            lisp.font_aset(val, i, v);
        }
        let extra = f_copy_sequence(lisp, &[extra])?;
        lisp.font_aset(val, FONT_EXTRA_INDEX, extra);
        lisp.font_put_extra(val, sym::K_FONT_ENTITY, key)?;
        return Ok(val);
    }
    let entity = lisp.font_make_entity();
    lisp.heap.set_car(cache, entity);
    let freetype = lisp.intern("freetype");
    lisp.font_aset(entity, FONT_TYPE_INDEX, freetype);
    lisp.font_aset(entity, FONT_REGISTRY_INDEX, sym::ISO10646_1);
    let family = lisp.font_intern_prop(face.family.as_bytes(), true)?;
    lisp.font_aset(entity, FONT_FAMILY_INDEX, family);
    lisp.font_set_style(
        entity,
        FONT_WEIGHT_INDEX,
        Value::fixnum(i64::from(face.weight)),
    )?;
    lisp.font_set_style(
        entity,
        FONT_SLANT_INDEX,
        Value::fixnum(i64::from(face.slant)),
    )?;
    lisp.font_set_style(
        entity,
        FONT_WIDTH_INDEX,
        Value::fixnum(i64::from(face.width)),
    )?;
    lisp.font_aset(entity, FONT_SIZE_INDEX, Value::fixnum(0));
    if face.spacing == SPACING_MONO {
        lisp.font_aset(
            entity,
            FONT_SPACING_INDEX,
            Value::fixnum(i64::from(face.spacing)),
        );
    }
    lisp.font_aset(entity, FONT_AVGWIDTH_INDEX, Value::fixnum(0));
    let extra = f_copy_sequence(lisp, &[extra])?;
    lisp.font_aset(entity, FONT_EXTRA_INDEX, extra);
    lisp.font_put_extra(entity, sym::K_FONT_ENTITY, key)?;
    Ok(entity)
}

fn ftfont_resolve_generic_family(lisp: &mut Lisp, family: Value) -> LispResult {
    let name = lisp.symbol_name_string(family).to_lowercase();
    let mut family = lisp.intern(&name);
    let mono = lisp.intern("mono");
    let sans = lisp.intern("sans");
    let sans_serif_space = lisp.intern("sans serif");
    if family == mono {
        family = lisp.intern("monospace");
    } else if family == sans || family == sans_serif_space {
        family = lisp.intern("sans-serif");
    }
    let list = FTFONT.with(|ft| ft.borrow().generic_family_list);
    let slot = lisp.assq(family, list);
    if !slot.is_cons() {
        return Ok(Value::NIL);
    }
    if lisp.xcdr(slot) != sym::T {
        return Ok(lisp.xcdr(slot));
    }
    let name = lisp.symbol_name_string(family);
    let matched = with_backend(|backend| {
        backend.match_font(&FontQuery {
            family: Some(name),
            ..FontQuery::default()
        })
    });
    let family = match matched {
        Some(face) => lisp.intern(&face.family),
        None => Value::NIL,
    };
    lisp.heap.set_cdr(slot, family);
    Ok(family)
}

fn ftfont_get_charset(lisp: &mut Lisp, registry: Value) -> LispResult<Option<usize>> {
    let name = lisp.string_bytes(lisp.symbol_name(registry)).to_vec();
    let mut re = Vec::with_capacity(name.len() * 2);
    for &c in &name {
        if c == b'.' {
            re.push(b'\\');
        } else if c == b'*' {
            re.push(b'.');
        }
        re.push(if c == b'?' { b'.' } else { c });
    }
    let regexp = lisp.make_string(&re);
    let base = lisp.stack.len();
    lisp.stack.push(regexp);
    let mut found = None;
    for (i, (name, _, _)) in FC_CHARSET_TABLE.iter().enumerate() {
        match lisp.fast_c_string_match_ignore_case(regexp, name.as_bytes()) {
            Ok(pos) if pos >= 0 => {
                found = Some(i);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                lisp.stack.truncate(base);
                return Err(e);
            }
        }
    }
    lisp.stack.truncate(base);
    Ok(found)
}

struct SpecPattern {
    family: Option<Value>,
    foundry: bool,
    charset: Vec<u32>,
    dpi: bool,
    scalable: Option<bool>,
}

fn ftfont_spec_pattern(lisp: &mut Lisp, spec: Value) -> LispResult<Option<SpecPattern>> {
    if let Some(slant) = lisp.font_aref(spec, FONT_SLANT_INDEX).as_fixnum()
        && slant >> 8 < 100
    {
        return Ok(None);
    }
    let mut dpi = lisp.font_aref(spec, FONT_DPI_INDEX).is_fixnum();
    let mut scalable =
        (lisp.font_aref(spec, FONT_AVGWIDTH_INDEX).as_fixnum() == Some(0)).then_some(true);
    let registry = lisp.font_aref(spec, FONT_REGISTRY_INDEX);
    let mut charset = Vec::new();
    if !(registry.is_nil()
        || registry == sym::ASCII_0
        || registry == sym::ISO10646_1
        || registry == sym::UNICODE_BMP)
    {
        let Some(idx) = ftfont_get_charset(lisp, registry)? else {
            return Ok(None);
        };
        charset.extend_from_slice(FC_CHARSET_TABLE[idx].1);
    }
    let mut script = Value::NIL;
    let mut extra = lisp.font_aref(spec, FONT_EXTRA_INDEX);
    let k_dpi = lisp.intern(":dpi");
    let k_scalable = lisp.intern(":scalable");
    while extra.is_cons() {
        let elt = lisp.xcar(extra);
        let key = lisp.xcar(elt);
        let val = lisp.xcdr(elt);
        if key == k_dpi {
            if val.is_fixnum() {
                dpi = true;
            }
        } else if key == sym::K_OTF {
            if val.is_cons() {
                return Ok(None);
            }
        } else if key == sym::K_SCRIPT {
            script = val;
        } else if key == k_scalable {
            scalable = Some(!val.is_nil());
        }
        extra = lisp.xcdr(extra);
    }
    if !script.is_nil() && charset.is_empty() {
        let representative = lisp.find_symbol_value(sym::SCRIPT_REPRESENTATIVE_CHARS);
        let chars = lisp.assq(script, representative);
        if chars.is_cons() && lisp.xcdr(chars).is_cons() {
            let mut tail = lisp.xcdr(chars);
            while tail.is_cons() {
                if let Some(c) = lisp.xcar(tail).as_fixnum()
                    && (0..=i64::from(emacs_core::character::MAX_CHAR)).contains(&c)
                {
                    charset.push(c as u32);
                }
                tail = lisp.xcdr(tail);
            }
        }
    }
    let family = lisp.font_aref(spec, FONT_FAMILY_INDEX);
    Ok(Some(SpecPattern {
        family: (!family.is_nil()).then_some(family),
        foundry: !lisp.font_aref(spec, FONT_FOUNDRY_INDEX).is_nil(),
        charset,
        dpi,
        scalable,
    }))
}

fn ftfont_list(lisp: &mut Lisp, spec: Value) -> LispResult {
    let Some(pattern) = ftfont_spec_pattern(lisp, spec)? else {
        return Ok(Value::NIL);
    };
    let mut chars = Value::NIL;
    if pattern.charset.is_empty() {
        let extra = lisp.font_aref(spec, FONT_EXTRA_INDEX);
        let val = lisp.assq(sym::K_SCRIPT, extra);
        if !val.is_nil() {
            let representative = lisp.find_symbol_value(sym::SCRIPT_REPRESENTATIVE_CHARS);
            let script = lisp.xcdr(val);
            let val = lisp.assq(script, representative);
            if val.is_cons() && lisp.is_vector(lisp.xcdr(val)) {
                chars = lisp.xcdr(val);
            }
        }
    }
    let spacing = lisp.font_aref(spec, FONT_SPACING_INDEX).as_fixnum();
    let mut family = pattern.family.map(|family| lisp.symbol_name_string(family));
    if let Some(spec_family) = pattern.family {
        let resolved = ftfont_resolve_generic_family(lisp, spec_family)?;
        if !resolved.is_nil() {
            family = Some(lisp.symbol_name_string(resolved));
        }
    }
    let adstyle = lisp.font_aref(spec, FONT_ADSTYLE_INDEX);
    let adstyle = (!adstyle.is_nil() && !lisp.string_bytes(lisp.symbol_name(adstyle)).is_empty())
        .then_some(adstyle);
    if pattern.foundry || pattern.dpi || pattern.scalable == Some(false) || adstyle.is_some() {
        return Ok(Value::NIL);
    }
    let required = pattern.charset;
    let chars_p = lisp.is_vector(chars);
    let any_of: Vec<u32> = lisp
        .vector_items(chars)
        .iter()
        .filter_map(|c| c.as_fixnum())
        .filter(|&c| (0..=i64::from(u32::MAX)).contains(&c))
        .map(|c| c as u32)
        .collect();
    let candidates = with_backend(|backend| {
        let faces = backend.entities();
        faces
            .into_iter()
            .filter(|face| {
                family
                    .as_ref()
                    .is_none_or(|family| face.families.iter().any(|f| same_family(f, family)))
            })
            .filter(|face| {
                spacing.is_none_or(|spacing| {
                    face.spacing != SPACING_MONO || i64::from(face.spacing) == spacing
                })
            })
            .filter(|face| {
                required.iter().all(|&c| backend.face_has_char(face, c))
                    && (!chars_p || any_of.iter().any(|&c| backend.face_has_char(face, c)))
            })
            .collect::<Vec<_>>()
    });
    let extra = lisp.font_aref(spec, FONT_EXTRA_INDEX);
    let base = lisp.stack.len();
    let mut entities = Vec::with_capacity(candidates.len());
    for face in &candidates {
        let entity = match ftfont_pattern_entity(lisp, face, extra) {
            Ok(entity) => entity,
            Err(e) => {
                lisp.stack.truncate(base);
                return Err(e);
            }
        };
        if !entity.is_nil() {
            lisp.stack.push(entity);
            entities.push(entity);
        }
    }
    let val = lisp.list(&entities);
    lisp.stack.truncate(base);
    lisp.font_add_log("ftfont-list", spec, val)?;
    Ok(val)
}

fn ftfont_list2(lisp: &mut Lisp, spec: Value, type_: &str) -> LispResult {
    let list = ftfont_list(lisp, spec)?;
    let type_ = lisp.intern(type_);
    let mut tail = list;
    while tail.is_cons() {
        let entity = lisp.xcar(tail);
        lisp.font_aset(entity, FONT_TYPE_INDEX, type_);
        tail = lisp.xcdr(tail);
    }
    Ok(list)
}

fn ftcrfont_list(lisp: &mut Lisp, _f: Value, spec: Value) -> LispResult {
    ftfont_list2(lisp, spec, "ftcr")
}

fn ftcrhbfont_list(lisp: &mut Lisp, _f: Value, spec: Value) -> LispResult {
    ftfont_list2(lisp, spec, "ftcrhb")
}

fn ftfont_match(lisp: &mut Lisp, spec: Value) -> LispResult {
    let Some(pattern) = ftfont_spec_pattern(lisp, spec)? else {
        return Ok(Value::NIL);
    };
    let query = FontQuery {
        family: pattern.family.map(|family| lisp.symbol_name_string(family)),
        ..FontQuery::default()
    };
    let matched = with_backend(|backend| backend.match_font(&query));
    let mut entity = Value::NIL;
    if let Some(face) = matched {
        let extra = lisp.font_aref(spec, FONT_EXTRA_INDEX);
        entity = ftfont_pattern_entity(lisp, &face, extra)?;
        let spec_family = lisp.font_aref(spec, FONT_FAMILY_INDEX);
        if !spec_family.is_nil() && !entity.is_nil() {
            let generic = FTFONT.with(|ft| ft.borrow().generic_family_list);
            let entity_family = lisp.font_aref(entity, FONT_FAMILY_INDEX);
            if lisp.assq(spec_family, generic).is_nil()
                && lisp.string_bytes(lisp.symbol_name(spec_family))
                    != lisp.string_bytes(lisp.symbol_name(entity_family))
            {
                entity = Value::NIL;
            }
        }
    }
    lisp.font_add_log("ftfont-match", spec, entity)?;
    Ok(entity)
}

fn ftfont_match2(lisp: &mut Lisp, spec: Value, type_: &str) -> LispResult {
    let entity = ftfont_match(lisp, spec)?;
    if !entity.is_nil() {
        let type_ = lisp.intern(type_);
        lisp.font_aset(entity, FONT_TYPE_INDEX, type_);
    }
    Ok(entity)
}

fn ftcrfont_match(lisp: &mut Lisp, _f: Value, spec: Value) -> LispResult {
    ftfont_match2(lisp, spec, "ftcr")
}

fn ftcrhbfont_match(lisp: &mut Lisp, _f: Value, spec: Value) -> LispResult {
    ftfont_match2(lisp, spec, "ftcrhb")
}

fn ftfont_list_family(lisp: &mut Lisp, _f: Value) -> LispResult {
    let families = with_backend(|backend| {
        let mut seen: Vec<String> = Vec::new();
        for face in backend.entities() {
            if !seen.contains(&face.family) {
                seen.push(face.family);
            }
        }
        seen
    });
    let mut list = Value::NIL;
    for family in families {
        let family = lisp.intern(&family);
        list = lisp.cons(family, list);
    }
    Ok(list)
}

pub(crate) fn open_font_data(lisp: &Lisp, font: Value) -> Option<Rc<RefCell<OpenFont>>> {
    lisp.font_object(font)
        .driver_data
        .clone()
        .and_then(|data| data.downcast::<RefCell<OpenFont>>().ok())
}

fn ftcrfont_open(lisp: &mut Lisp, _f: Value, entity: Value, pixel_size: i32) -> LispResult {
    let extra = lisp.font_aref(entity, FONT_EXTRA_INDEX);
    let val = lisp.assq(sym::K_FONT_ENTITY, extra);
    if !val.is_cons() {
        return Ok(Value::NIL);
    }
    let filename = lisp.xcar(lisp.xcdr(val));
    let entity_size = lisp
        .font_aref(entity, FONT_SIZE_INDEX)
        .as_fixnum()
        .unwrap_or(0);
    let size = if entity_size == 0 {
        i64::from(pixel_size)
    } else {
        entity_size
    };
    let Some(face) = entity_face(lisp, entity) else {
        return Ok(Value::NIL);
    };
    let Some(open) = with_backend(|backend| backend.open(&face, pixel_size.max(0) as u32)) else {
        return Ok(Value::NIL);
    };
    let ty = lisp.font_aref(entity, FONT_TYPE_INDEX);
    let font_object = lisp.font_build_object(ty, entity, size as f64)?;
    lisp.font_aset(font_object, FONT_FILE_INDEX, filename);
    let k_minspace = lisp.intern(":minspace");
    let minspace = lisp.assq(k_minspace, extra);
    let no_minspace = minspace.is_cons() && lisp.xcdr(minspace).is_nil();
    let driver = if ty == lisp.intern("ftcrhb") {
        ftcrhbfont_driver(lisp)
    } else {
        ftcrfont_driver(lisp)
    };
    let font = lisp.font_object_mut(font_object);
    font.pixel_size = size as i32;
    font.driver = Some(driver);
    font.encoding_charset = -1;
    font.repertory_charset = -1;
    font.min_width = open.min_width;
    font.max_width = open.max_width;
    font.average_width = open.average_width;
    font.space_width = open.space_width;
    font.ascent = open.ascent;
    if no_minspace {
        font.height = open.line_height;
        font.descent = font.height - font.ascent;
    } else {
        font.descent = open.descent;
        font.height = font.ascent + font.descent;
    }
    if entity_size == 0 {
        font.underline_position = open.underline_position;
        font.underline_thickness = open.underline_thickness;
    } else {
        font.underline_position = -1;
        font.underline_thickness = 0;
    }
    font.baseline_offset = 0;
    font.relative_compose = 0;
    font.default_ascent = 0;
    font.vertical_centering = false;
    font.driver_data = Some(Rc::new(RefCell::new(open)));
    Ok(font_object)
}

fn ftcrfont_close(lisp: &mut Lisp, font: Value) {
    lisp.font_object_mut(font).driver_data = None;
}

fn charset_id(lisp: &mut Lisp, name: &str) -> Option<i32> {
    let symbol = lisp.intern(name);
    if lisp.charsetp(symbol) {
        lisp.check_charset_get_id(symbol).ok()
    } else {
        None
    }
}

fn ftfont_has_char(lisp: &mut Lisp, font: Value, c: i32) -> i32 {
    let adstyle = lisp.font_aref(font, FONT_ADSTYLE_INDEX);
    let cs = if adstyle == sym::JA {
        charset_id(lisp, "japanese-jisx0208")
    } else if adstyle == sym::KO {
        charset_id(lisp, "korean-ksc5601")
    } else {
        None
    };
    if let Some(cs) = cs {
        let invalid = lisp.charset(cs).invalid_code;
        return i32::from(lisp.encode_char(cs, c).is_ok_and(|code| code != invalid));
    }
    if lisp.font_entity_p(font) {
        let Some(face) = entity_face(lisp, font) else {
            return 0;
        };
        return i32::from(with_backend(|backend| {
            backend.face_has_char(&face, c as u32)
        }));
    }
    match open_font_data(lisp, font) {
        Some(open) => i32::from(with_backend(|backend| {
            backend.encode_char(&open.borrow(), c as u32).is_some()
        })),
        None => 0,
    }
}

fn ftcrfont_has_char(lisp: &mut Lisp, font: Value, c: i32) -> i32 {
    if lisp.font_entity_p(font) {
        return ftfont_has_char(lisp, font, c);
    }
    let adstyle = lisp.font_aref(font, FONT_ADSTYLE_INDEX);
    let cs = if adstyle == sym::JA {
        charset_id(lisp, "japanese-jisx0208")
    } else if adstyle == sym::KO {
        charset_id(lisp, "korean-ksc5601")
    } else {
        None
    };
    if let Some(cs) = cs {
        let invalid = lisp.charset(cs).invalid_code;
        return i32::from(lisp.encode_char(cs, c).is_ok_and(|code| code != invalid));
    }
    -1
}

fn ftcrfont_encode_char(lisp: &mut Lisp, font: Value, c: i32) -> u32 {
    let Some(open) = open_font_data(lisp, font) else {
        return FONT_INVALID_CODE;
    };
    with_backend(|backend| backend.encode_char(&open.borrow(), c as u32))
        .unwrap_or(FONT_INVALID_CODE)
}

fn ftcrfont_text_extents(lisp: &mut Lisp, font: Value, code: &[u32]) -> FontMetrics {
    let Some(open) = open_font_data(lisp, font) else {
        return FontMetrics::default();
    };
    let m = with_backend(|backend| backend.text_extents(&mut open.borrow_mut(), code));
    FontMetrics {
        lbearing: m.lbearing,
        rbearing: m.rbearing,
        width: m.width,
        ascent: m.ascent,
        descent: m.descent,
    }
}

fn ftcrfont_draw(
    lisp: &mut Lisp,
    s: &mut GlyphString,
    from: usize,
    to: usize,
    x: i32,
    y: i32,
    with_background: bool,
) -> i32 {
    let len = to - from;
    let Some(open) = open_font_data(lisp, s.font) else {
        return len as i32;
    };
    let frame = lisp.frame(s.f).id as u64;
    let glyphs = s.char2b[from..to].to_vec();
    let background = if with_background {
        let face_font = s
            .face
            .and_then(|face| lisp.face_from_id(s.f, face))
            .map_or(s.font, |face| face.font);
        let fo = lisp.font_object(face_font);
        let (ascent, height) = (fo.ascent, fo.height);
        Some(Background {
            color: s.xgcv.background as u32,
            ascent,
            width: s.width,
            height,
            respect_alpha: s.hl != DrawGlyphsFace::Cursor,
        })
    } else {
        None
    };
    let how = DrawGlyphs {
        x,
        y,
        foreground: s.xgcv.foreground as u32,
        background,
        padding: s.padding_p,
    };
    crate::with_display(|display| {
        if let Some(surface) = display.surface_mut(frame) {
            with_backend(|backend| backend.draw(surface, &mut open.borrow_mut(), &glyphs, &how));
        }
    });
    len as i32
}

fn ftfont_filter_properties(lisp: &mut Lisp, font: Value, alist: Value) -> LispResult<()> {
    lisp.font_filter_properties(font, alist, FTFONT_BOOLEANS, FTFONT_NON_BOOLEANS)
}

fn ftfont_combining_capability(_lisp: &mut Lisp, _font: Value) -> Value {
    Value::NIL
}
