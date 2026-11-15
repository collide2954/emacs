use emacs_core::dispextern::{
    TOOL_BAR_IMAGE_DISABLED_DESELECTED, TOOL_BAR_IMAGE_DISABLED_SELECTED,
    TOOL_BAR_IMAGE_ENABLED_DESELECTED, TOOL_BAR_IMAGE_ENABLED_SELECTED, TOOL_BAR_ITEM_ENABLED_P,
    TOOL_BAR_ITEM_IMAGES, TOOL_BAR_ITEM_LABEL, TOOL_BAR_ITEM_NSLOTS, TOOL_BAR_ITEM_SELECTED_P,
    TOOL_BAR_ITEM_TYPE, TOOL_BAR_ITEM_VERT_ONLY,
};
use emacs_core::error::LispResult;
use emacs_core::image::{CrData, CrFilter};
use emacs_core::lisp::Lisp;
use emacs_core::symbols as sym;
use emacs_core::value::Value;

use crate::pgtkterm::{frame_key, with_output};
use crate::protocol::{ToolBar, ToolBarImage, ToolBarPosition, ToolItem};
use crate::surface::{Rect, Surface};

const DEFAULT_TOOL_BAR_BUTTON_MARGIN: i64 = 4;

#[derive(Clone, Copy, Debug)]
pub struct TbInfo {
    pub last_tool_bar: Value,
    pub style: Value,
    pub hmargin: i64,
    pub vmargin: i64,
    pub n_last_items: usize,
}

impl TbInfo {
    pub fn trace(&self, roots: &mut Vec<Value>) {
        roots.extend_from_slice(&[self.last_tool_bar, self.style]);
    }
}

fn ranged_margin(value: Value) -> Option<i64> {
    value
        .as_fixnum()
        .filter(|&n| (1..=i64::from(i32::MAX)).contains(&n))
}

fn file_for_image(lisp: &Lisp, image: Value) -> Value {
    let mut specified_file = Value::NIL;
    let mut tail = lisp.xcdr(image);
    while specified_file.is_nil() && tail.is_cons() && lisp.xcdr(tail).is_cons() {
        if lisp.xcar(tail) == sym::K_FILE {
            specified_file = lisp.xcar(lisp.xcdr(tail));
        }
        tail = lisp.xcdr(lisp.xcdr(tail));
    }
    specified_file
}

fn xg_pack_tool_bar(lisp: &Lisp, f: Value, pos: Value) {
    let position = if pos == sym::LEFT {
        ToolBarPosition::Left
    } else if pos == sym::RIGHT {
        ToolBarPosition::Right
    } else if pos == sym::TOP {
        ToolBarPosition::Top
    } else {
        ToolBarPosition::Bottom
    };
    with_output(lisp, f, |o| o.toolbar_position = Some(position));
}

fn xg_create_tool_bar(lisp: &Lisp, f: Value) {
    with_output(lisp, f, |o| {
        if o.tbinfo.is_none() {
            o.tbinfo = Some(TbInfo {
                last_tool_bar: Value::NIL,
                style: Value::NIL,
                hmargin: 0,
                vmargin: 0,
                n_last_items: 0,
            });
        }
        o.toolbar_widget = true;
    });
}

fn xg_get_image_for_pixmap(lisp: &Lisp, f: Value, id: usize) -> Option<ToolBarImage> {
    let img = lisp.image(f, id)?;
    let surface = img.cr_data.as_ref()?.surface.as_ref()?;
    let (width, height) = (surface.width.max(0) as u32, surface.height.max(0) as u32);
    let mut canvas = Surface::new(width, height, 0);
    canvas.set_alpha_background(0.0);
    canvas.resize(width, height);
    let pattern = CrData {
        matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        filter: CrFilter::Nearest,
        surface: Some(surface.clone()),
    };
    canvas.draw_image(
        Rect::new(0, 0, width as i32, height as i32),
        (0, 0),
        &pattern,
    );
    Some(ToolBarImage {
        width,
        height,
        rgba: canvas.rgba(),
    })
}

fn tool_bar_margins(lisp: &Lisp) -> (i64, i64) {
    let (mut hmargin, mut vmargin) = (0, 0);
    let margin = lisp.find_symbol_value(sym::TOOL_BAR_BUTTON_MARGIN);
    if let Some(n) = ranged_margin(margin) {
        hmargin = n;
        vmargin = n;
    } else if margin.is_cons() {
        if let Some(n) = ranged_margin(lisp.xcar(margin)) {
            hmargin = n;
        }
        if let Some(n) = ranged_margin(lisp.xcdr(margin)) {
            vmargin = n;
        }
    }
    (
        (hmargin - DEFAULT_TOOL_BAR_BUTTON_MARGIN).max(0),
        (vmargin - DEFAULT_TOOL_BAR_BUTTON_MARGIN).max(0),
    )
}

pub fn update_frame_tool_bar(lisp: &mut Lisp, f: Value) -> LispResult<()> {
    if !with_output(lisp, f, |o| o.widget) {
        return Ok(());
    }
    let (hmargin, vmargin) = tool_bar_margins(lisp);
    if !with_output(lisp, f, |o| o.toolbar_widget) {
        xg_create_tool_bar(lisp, f);
    }
    let style = crate::xsettings::f_tool_bar_get_system_style(lisp, &[])?;
    let tbinfo = with_output(lisp, f, |o| o.tbinfo).expect("tool bar info");
    let (items, n) = (lisp.frame(f).tool_bar_items, lisp.frame(f).n_tool_bar_items);
    if !tbinfo.last_tool_bar.is_nil()
        && !items.is_nil()
        && tbinfo.n_last_items == n
        && tbinfo.hmargin == hmargin
        && tbinfo.vmargin == vmargin
        && lisp.equal(tbinfo.style, style)?
        && lisp.equal(tbinfo.last_tool_bar, items)?
    {
        return Ok(());
    }
    with_output(lisp, f, |o| {
        o.tbinfo = Some(TbInfo {
            last_tool_bar: items,
            style,
            hmargin,
            vmargin,
            n_last_items: n,
        });
    });
    let text_image = style == sym::TEXT_IMAGE_HORIZ;
    let horiz = style == sym::BOTH_HORIZ || text_image;
    let map_stock = lisp.intern("x-gtk-map-stock");
    let mut out = Vec::new();
    for i in 0..n {
        let prop =
            |lisp: &Lisp, idx: usize| lisp.vector_items(items)[i * TOOL_BAR_ITEM_NSLOTS + idx];
        let enabled_p = !prop(lisp, TOOL_BAR_ITEM_ENABLED_P).is_nil();
        let selected_p = !prop(lisp, TOOL_BAR_ITEM_SELECTED_P).is_nil();
        let vert_only = !prop(lisp, TOOL_BAR_ITEM_VERT_ONLY).is_nil();
        let label = if style == sym::IMAGE || (vert_only && horiz) {
            Value::NIL
        } else {
            prop(lisp, TOOL_BAR_ITEM_LABEL)
        };
        if prop(lisp, TOOL_BAR_ITEM_TYPE) == sym::T {
            out.push(ToolItem::Separator);
            continue;
        }
        let mut image = prop(lisp, TOOL_BAR_ITEM_IMAGES);
        if !lisp.valid_image_p(image) {
            continue;
        }
        let specified_file = file_for_image(lisp, image);
        let fboundp = !lisp.heap.symbol_ref(map_stock).function.is_nil();
        if !specified_file.is_nil() && fboundp {
            lisp.funcall(map_stock, &[specified_file])?;
        }
        if lisp.is_vector(image) {
            let idx = match (enabled_p, selected_p) {
                (true, true) => TOOL_BAR_IMAGE_ENABLED_SELECTED,
                (true, false) => TOOL_BAR_IMAGE_ENABLED_DESELECTED,
                (false, true) => TOOL_BAR_IMAGE_DISABLED_SELECTED,
                (false, false) => TOOL_BAR_IMAGE_DISABLED_DESELECTED,
            };
            image = lisp.vector_items(image)[idx];
        }
        let img_id = lisp.lookup_image(f, image, -1)?;
        lisp.prepare_image_for_display(f, img_id)?;
        let failed = lisp
            .image(f, img_id)
            .is_none_or(|img| img.load_failed_p || img.cr_data.is_none());
        if failed {
            continue;
        }
        let w = if style == sym::TEXT {
            None
        } else {
            xg_get_image_for_pixmap(lisp, f, img_id)
        };
        let label = if label.is_nil() {
            None
        } else if label.is_string() {
            Some(String::from_utf8_lossy(lisp.string_bytes(label)).into_owned())
        } else {
            Some(String::new())
        };
        out.push(ToolItem::Button {
            index: i,
            image: w,
            label,
            enabled: enabled_p,
        });
    }
    if n != 0 && with_output(lisp, f, |o| o.toolbar_position.is_none()) {
        let pos = lisp.frame(f).tool_bar_position;
        xg_pack_tool_bar(lisp, f, pos);
    }
    let bar = ToolBar {
        items: out,
        horiz,
        text_image,
        hmargin: hmargin as u32,
        vmargin: vmargin as u32,
        position: with_output(lisp, f, |o| o.toolbar_position),
    };
    let key = frame_key(lisp, f) as u64;
    crate::with_display(|d| d.set_tool_bar(key, bar));
    lisp.frame_mut(f).flags.tool_bar_resized = true;
    Ok(())
}

pub fn free_frame_tool_bar(lisp: &mut Lisp, f: Value) -> LispResult<()> {
    if with_output(lisp, f, |o| o.toolbar_widget) {
        with_output(lisp, f, |o| {
            o.toolbar_widget = false;
            o.toolbar_position = None;
            o.tbinfo = None;
        });
        let key = frame_key(lisp, f) as u64;
        crate::with_display(|d| d.remove_tool_bar(key));
        lisp.adjust_frame_size(f, -1, -1, 2, sym::TOOL_BAR_LINES)?;
    }
    Ok(())
}
