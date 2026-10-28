use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::symbols as sym;
use crate::value::Value;

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::TEMPORARY_FILE_DIRECTORY, Value::NIL);
    lisp.defvar_bool(sym::CREATE_LOCKFILES, true);
}

impl Lisp {
    pub fn unlock_all_files(&mut self) -> LispResult<()> {
        let buffers = self.buffers.clone();
        let base = self.stack.len();
        self.stack.extend_from_slice(&buffers);
        for b in buffers {
            if !self.buffer_live_p(b) {
                continue;
            }
            let filename = self
                .buffer_local_get(b, sym::BUFFER_FILE_NAME)
                .unwrap_or(Value::NIL);
            let text = &self.buffer(b).text;
            if filename.is_string() && text.save_modiff < text.modiff {
                self.unlock_file_if_supported(filename)?;
            }
        }
        self.stack.truncate(base);
        Ok(())
    }
}
