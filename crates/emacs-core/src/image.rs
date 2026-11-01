use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

const MAX_IMAGE_SIZE: f64 = 10.0;
const PATH_BITMAPS: &str = if cfg!(windows) {
    "."
} else {
    "/usr/include/X11/bitmaps"
};

pub fn init_vars(lisp: &mut Lisp) {
    if cfg!(windows) {
        lisp.set_plain_value(sym::LIBPNG_VERSION, Value::fixnum(-1));
        lisp.set_plain_value(sym::LIBGIF_VERSION, Value::fixnum(-1));
        lisp.set_plain_value(sym::LIBJPEG_VERSION, Value::fixnum(-1));
    }
    lisp.defvar(sym::IMAGE_TYPES, Value::NIL);
    let max_image_size = lisp.float(MAX_IMAGE_SIZE);
    lisp.defvar(sym::MAX_IMAGE_SIZE, max_image_size);
    lisp.defvar_bool(sym::CROSS_DISABLED_IMAGES, false);
    let bitmaps = lisp.unibyte_string(PATH_BITMAPS.as_bytes());
    let bitmap_path = lisp.list(&[bitmaps]);
    lisp.defvar(sym::X_BITMAP_FILE_PATH, bitmap_path);
    lisp.defvar(sym::IMAGE_CACHE_EVICTION_DELAY, Value::fixnum(300));
    lisp.defvar(sym::IMAGE_SCALING_FACTOR, sym::AUTO);
}
