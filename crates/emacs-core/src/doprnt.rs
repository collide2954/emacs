use crate::lisp::Lisp;
use crate::symbols as sym;

impl Lisp {
    pub fn doprnt(&self, format: &str) -> String {
        let quoting_style = self.text_quoting_style();
        format
            .chars()
            .map(|fmtchar| match fmtchar {
                '`' if quoting_style == sym::CURVE => '\u{2018}',
                '\'' if quoting_style == sym::CURVE => '\u{2019}',
                '`' if quoting_style == sym::STRAIGHT => '\'',
                c => c,
            })
            .collect()
    }
}
