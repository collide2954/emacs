use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::lread::Source;
use crate::object::PropRun;
use crate::symbols as sym;
use crate::value::Value;

impl Lisp {
    pub fn current_buffer_value(&self) -> Value {
        Value::NIL
    }

    pub fn is_buffer(&self, _v: Value) -> bool {
        false
    }

    pub fn is_marker(&self, _v: Value) -> bool {
        false
    }

    pub fn buffer_live_p(&self, _buffer: Value) -> bool {
        false
    }

    pub fn buffer_local_get(&self, _buffer: Value, _symbol: Value) -> Option<Value> {
        None
    }

    pub fn buffer_local_set(&mut self, _buffer: Value, _symbol: Value, _value: Value) {}

    pub fn buffer_local_remove(&mut self, _buffer: Value, _symbol: Value) {}

    pub fn local_binding_exists(&self, symbol: Value) -> bool {
        let buffer = self.current_buffer_value();
        self.buffer_local_get(buffer, symbol).is_some()
    }

    pub fn local_variable_in(&self, symbol: Value, buffer: Value) -> bool {
        self.buffer_local_get(buffer, symbol).is_some()
    }

    pub fn buffer_read_char(&mut self, _buffer: Value) -> i64 {
        -1
    }

    pub fn buffer_unread_char(&mut self, _buffer: Value) {}

    pub fn buffer_line_column(&self, _buffer: Value) -> (i64, i64) {
        (1, 0)
    }

    pub fn buffer_source(&mut self, stream: Value) -> LispResult<Source> {
        Err(self.wrong_type(sym::BUFFERP, stream))
    }

    pub fn buffer_source_finish(&mut self, _stream: Value, _source: &Source) {}

    pub fn insert_chars_into(&mut self, dest: Value, _chars: &[u32]) -> LispResult<()> {
        Err(self.wrong_type(sym::BUFFERP, dest))
    }

    pub fn buffer_at_line_start(&self, _buffer: Value) -> bool {
        true
    }

    pub fn noninteractive(&self) -> bool {
        true
    }

    pub fn echo_area_append(&mut self, text: &str) -> LispResult<()> {
        use std::io::Write;
        let _ = self.stdout.write_all(text.as_bytes());
        Ok(())
    }

    pub fn set_text_properties(
        &mut self,
        start: Value,
        end: Value,
        properties: Value,
        object: Value,
    ) -> LispResult<()> {
        if !object.is_string() {
            return Err(self.wrong_type(sym::STRINGP, object));
        }
        let len = self.string_chars(object) as i64;
        let s = self.check_fixnum(start)?;
        let e = self.check_fixnum(end)?;
        let (s, e) = if s <= e { (s, e) } else { (e, s) };
        if s < 0 || e > len {
            return Err(self.args_out_of_range(start, end));
        }
        let (s, e) = (s as usize, e as usize);
        if s == e {
            return Ok(());
        }
        let runs = std::mem::take(&mut self.heap.string_mut(object).props);
        let mut out: Vec<PropRun> = Vec::with_capacity(runs.len() + 2);
        for run in runs {
            if run.end <= s || run.start >= e {
                out.push(run);
                continue;
            }
            if run.start < s {
                out.push(PropRun {
                    start: run.start,
                    end: s,
                    plist: run.plist,
                });
            }
            if run.end > e {
                out.push(PropRun {
                    start: e,
                    end: run.end,
                    plist: run.plist,
                });
            }
        }
        if !properties.is_nil() {
            out.push(PropRun {
                start: s,
                end: e,
                plist: properties,
            });
        }
        out.sort_by_key(|r| r.start);
        self.heap.string_mut(object).props = out;
        Ok(())
    }
}
