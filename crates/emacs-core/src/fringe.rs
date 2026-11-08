use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub const NO_FRINGE_BITMAP: i64 = 0;
pub const MAX_STANDARD_FRINGE_BITMAPS: i64 = 25;

impl Lisp {
    pub fn lookup_fringe_bitmap(&self, bitmap: Value) -> i64 {
        let bitmap = self.get(bitmap, sym::FRINGE);
        let Some(bn) = bitmap.as_fixnum() else {
            return 0;
        };
        if bn > NO_FRINGE_BITMAP && bn < MAX_STANDARD_FRINGE_BITMAPS {
            return bn;
        }
        0
    }
}
