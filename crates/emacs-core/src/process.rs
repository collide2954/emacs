use std::time::{Duration, Instant};

use num_bigint::BigInt;

use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::object::Vectorlike;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::Value;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("processp", 1, 1, f_processp),
    Subr::fixed("num-processors", 0, 1, f_num_processors),
    Subr::fixed("get-buffer-process", 1, 1, f_get_buffer_process),
];

#[derive(Clone, Debug)]
pub struct Process {
    pub tty_name: Value,
    pub name: Value,
    pub command: Value,
    pub filter: Value,
    pub sentinel: Value,
    pub log: Value,
    pub buffer: Value,
    pub childp: Value,
    pub plist: Value,
    pub type_: Value,
    pub mark: Value,
    pub status: Value,
    pub decode_coding_system: Value,
    pub decoding_buf: Value,
    pub encode_coding_system: Value,
    pub encoding_buf: Value,
    pub write_queue: Value,
    pub gnutls_cred_type: Value,
    pub gnutls_boot_parameters: Value,
    pub stderrproc: Value,
    pub thread: Value,
}

impl Process {
    pub fn trace(&self, out: &mut Vec<Value>) {
        out.extend_from_slice(&[
            self.tty_name,
            self.name,
            self.command,
            self.filter,
            self.sentinel,
            self.log,
            self.buffer,
            self.childp,
            self.plist,
            self.type_,
            self.mark,
            self.status,
            self.decode_coding_system,
            self.decoding_buf,
            self.encode_coding_system,
            self.encoding_buf,
            self.write_queue,
            self.gnutls_cred_type,
            self.gnutls_boot_parameters,
            self.stderrproc,
            self.thread,
        ]);
    }
}

impl Lisp {
    pub fn processp(&self, object: Value) -> bool {
        object.is_vectorlike() && matches!(self.heap.vectorlike_ref(object), Vectorlike::Process(_))
    }
}

fn f_processp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    Ok(if lisp.processp(args[0]) {
        sym::T
    } else {
        Value::NIL
    })
}

fn f_get_buffer_process(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let buffer = args[0];
    if buffer.is_nil() {
        return Ok(Value::NIL);
    }
    crate::buffer::f_get_buffer(lisp, &[buffer])?;
    Ok(Value::NIL)
}

#[cfg(windows)]
pub type ULong = u32;
#[cfg(not(windows))]
pub type ULong = u64;

const NPROC_MINIMUM: ULong = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum NprocQuery {
    All,
    Current,
    CurrentOverridable,
}

fn available_parallelism() -> ULong {
    std::thread::available_parallelism()
        .map_or(0, |n| ULong::try_from(n.get()).unwrap_or(ULong::MAX))
}

#[cfg(target_os = "linux")]
fn nprocessors_conf() -> ULong {
    if let Ok(text) = std::fs::read_to_string("/sys/devices/system/cpu/possible") {
        let mut count: ULong = 0;
        for range in text.trim().split(',') {
            let (lo, hi) = range.split_once('-').unwrap_or((range, range));
            match (lo.parse::<ULong>(), hi.parse::<ULong>()) {
                (Ok(lo), Ok(hi)) if lo <= hi => count += hi - lo + 1,
                _ => {
                    count = 0;
                    break;
                }
            }
        }
        if count > 0 {
            return count;
        }
    }
    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
        let count = entries
            .flatten()
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.strip_prefix("cpu")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            })
            .count();
        if count > 0 {
            return ULong::try_from(count).unwrap_or(ULong::MAX);
        }
    }
    available_parallelism()
}

#[cfg(not(target_os = "linux"))]
fn nprocessors_conf() -> ULong {
    available_parallelism()
}

fn num_processors_available(query: NprocQuery) -> ULong {
    if query == NprocQuery::Current {
        let nprocs = available_parallelism();
        if nprocs > 0 {
            return nprocs;
        }
    } else {
        let mut nprocs = nprocessors_conf();
        if cfg!(target_os = "linux") && (nprocs == 1 || nprocs == 2) {
            let nprocs_current = available_parallelism();
            if nprocs_current > nprocs {
                nprocs = nprocs_current;
            }
        }
        if nprocs > 0 {
            return nprocs;
        }
    }
    NPROC_MINIMUM
}

fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

pub fn parse_omp_threads(threads: Option<std::ffi::OsString>) -> ULong {
    let Some(threads) = threads else {
        return 0;
    };
    let threads = threads.to_string_lossy();
    let bytes = threads.as_bytes();
    let mut i = 0;
    while i < bytes.len() && c_isspace(bytes[i]) {
        i += 1;
    }
    if i < bytes.len() && bytes[i].is_ascii_digit() {
        let mut value: ULong = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(ULong::from(bytes[i] - b'0')))
                .unwrap_or(ULong::MAX);
            i += 1;
        }
        while i < bytes.len() && c_isspace(bytes[i]) {
            i += 1;
        }
        if i == bytes.len() || bytes[i] == b',' {
            return value;
        }
    }
    0
}

fn num_processors(mut query: NprocQuery) -> ULong {
    let mut nproc_limit = ULong::MAX;
    if query == NprocQuery::CurrentOverridable {
        let omp_env_threads = parse_omp_threads(std::env::var_os("OMP_NUM_THREADS"));
        let mut omp_env_limit = parse_omp_threads(std::env::var_os("OMP_THREAD_LIMIT"));
        if omp_env_limit == 0 {
            omp_env_limit = ULong::MAX;
        }
        if omp_env_threads != 0 {
            return omp_env_threads.min(omp_env_limit);
        }
        nproc_limit = omp_env_limit;
        query = NprocQuery::Current;
    }
    if nproc_limit > NPROC_MINIMUM {
        let nprocs = num_processors_available(query);
        nproc_limit = nprocs.min(nproc_limit);
    }
    nproc_limit
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Wait {
    Minimum,
    Timeout,
    Forever,
}

impl Lisp {
    pub fn wait_reading_process_output(
        &mut self,
        time_limit: i64,
        nsecs: i64,
        read_kbd: i32,
        wait_for_cell: Value,
    ) -> LispResult<i32> {
        let mut end_time = None;
        let wait = if time_limit < 0 || nsecs < 0 {
            Wait::Minimum
        } else if time_limit > 0 || nsecs > 0 {
            end_time = Instant::now().checked_add(
                Duration::from_secs(time_limit as u64) + Duration::from_nanos(nsecs as u64),
            );
            Wait::Timeout
        } else {
            Wait::Forever
        };
        loop {
            let mut timeout_reduced_for_timers = false;
            if read_kbd >= 0 {
                self.maybe_quit()?;
            }
            if !wait_for_cell.is_nil() && !self.xcar(wait_for_cell).is_nil() {
                break;
            }
            let mut timeout = if wait == Wait::Timeout {
                match end_time {
                    Some(end) => {
                        let now = Instant::now();
                        if end <= now {
                            break;
                        }
                        end - now
                    }
                    None => Duration::MAX,
                }
            } else {
                Duration::from_secs(if wait < Wait::Timeout { 0 } else { 100_000 })
            };
            if wait_for_cell.is_nil() {
                let timer_delay = self.timer_check()?;
                if read_kbd != 0 && self.requeued_command_events_pending_p() {
                    break;
                }
                if let Some((sec, nsec)) = timer_delay {
                    let delay = Duration::new(sec.max(0) as u64, nsec as u32);
                    if delay < timeout {
                        timeout = delay;
                        timeout_reduced_for_timers = true;
                    }
                }
            }
            if !(read_kbd != 0 && self.detect_input_pending()?) {
                std::thread::sleep(timeout);
            }
            if wait < Wait::Forever && !timeout_reduced_for_timers {
                break;
            }
            if read_kbd != 0 && self.detect_input_pending_run_timers()? {
                self.swallow_events()?;
                if self.detect_input_pending_run_timers()? {
                    break;
                }
            }
            if read_kbd != 0 && self.requeued_command_events_pending_p() {
                break;
            }
            if !wait_for_cell.is_nil() && self.detect_input_pending()? {
                self.swallow_events()?;
                if self.detect_input_pending()? {
                    break;
                }
            }
            if !wait_for_cell.is_nil() && !self.xcar(wait_for_cell).is_nil() {
                break;
            }
        }
        Ok(-1)
    }
}

fn f_num_processors(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let query = args[0];
    let n = num_processors(if query == sym::ALL {
        NprocQuery::All
    } else if query == sym::CURRENT {
        NprocQuery::Current
    } else {
        NprocQuery::CurrentOverridable
    });
    Ok(lisp.bignum(BigInt::from(n)))
}

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar_bool(sym::DELETE_EXITED_PROCESSES, true);
    lisp.defvar(sym::PROCESS_CONNECTION_TYPE, sym::T);
    lisp.defvar(sym::PROCESS_ADAPTIVE_READ_BUFFERING, Value::NIL);
    lisp.defvar_bool(sym::PROCESS_PRIORITIZE_LOWER_FDS, false);
    let interrupt = lisp.list(&[sym::INTERNAL_DEFAULT_INTERRUPT_PROCESS]);
    lisp.defvar(sym::INTERRUPT_PROCESS_FUNCTIONS, interrupt);
    let signal = lisp.list(&[sym::INTERNAL_DEFAULT_SIGNAL_PROCESS]);
    lisp.defvar(sym::SIGNAL_PROCESS_FUNCTIONS, signal);
    lisp.defvar(sym::INTERNAL__DAEMON_SOCKNAME, Value::NIL);
    lisp.defvar_int(sym::READ_PROCESS_OUTPUT_MAX, 65536);
    lisp.defvar_bool(sym::FAST_READ_PROCESS_OUTPUT, true);
    lisp.defvar_int(sym::PROCESS_ERROR_PAUSE_TIME, 1);
}
