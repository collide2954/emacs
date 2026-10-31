use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::error::LispResult;
use crate::lisp::Lisp;
use crate::timefns::{Timespec, current_timespec, timespec_add, timespec_sub};
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AtimerType {
    Absolute,
    Relative,
    Continuous,
}

pub type AtimerCallback = fn(&mut Lisp, &Atimer) -> LispResult<()>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AtimerId(i64);

impl AtimerId {
    pub fn to_value(self) -> Value {
        Value::fixnum(self.0)
    }

    pub fn from_value(v: Value) -> AtimerId {
        AtimerId(v.as_fixnum().unwrap_or(-1))
    }
}

#[derive(Clone)]
pub struct Atimer {
    pub id: AtimerId,
    pub type_: AtimerType,
    pub expiration: Timespec,
    pub interval: Timespec,
    pub fn_: AtimerCallback,
    pub client_data: Value,
}

#[derive(Default)]
struct ItimerState {
    expire: Option<Instant>,
    terminate: bool,
}

struct Itimer {
    shared: Arc<(Mutex<ItimerState>, Condvar)>,
}

impl Itimer {
    fn start(pending_signals: Arc<AtomicBool>) -> Itimer {
        let shared = Arc::new((Mutex::new(ItimerState::default()), Condvar::new()));
        let thread_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("itimer".into())
            .spawn(move || timer_loop(&thread_shared, &pending_signals))
            .expect("spawn itimer thread");
        Itimer { shared }
    }

    fn set(&self, expire: Option<Instant>) {
        let (lock, cvar) = &*self.shared;
        lock.lock().unwrap_or_else(PoisonError::into_inner).expire = expire;
        cvar.notify_one();
    }
}

impl Drop for Itimer {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.shared;
        lock.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .terminate = true;
        cvar.notify_one();
    }
}

fn timer_loop(shared: &(Mutex<ItimerState>, Condvar), pending_signals: &AtomicBool) {
    let (lock, cvar) = shared;
    let mut state = lock.lock().unwrap_or_else(PoisonError::into_inner);
    loop {
        if state.terminate {
            return;
        }
        match state.expire {
            None => state = cvar.wait(state).unwrap_or_else(PoisonError::into_inner),
            Some(expire) => {
                let now = Instant::now();
                if now < expire {
                    state = cvar
                        .wait_timeout(state, expire - now)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                } else {
                    state.expire = None;
                    pending_signals.store(true, Ordering::SeqCst);
                }
            }
        }
    }
}

pub struct Atimers {
    atimers: Vec<Atimer>,
    stopped_atimers: Vec<Atimer>,
    next_id: i64,
    pub pending_signals: Arc<AtomicBool>,
    itimer: Option<Itimer>,
}

impl Default for Atimers {
    fn default() -> Self {
        Atimers {
            atimers: Vec::new(),
            stopped_atimers: Vec::new(),
            next_id: 0,
            pending_signals: Arc::new(AtomicBool::new(false)),
            itimer: None,
        }
    }
}

impl Atimers {
    pub fn trace(&self, out: &mut Vec<Value>) {
        for t in self.atimers.iter().chain(&self.stopped_atimers) {
            out.push(t.client_data);
        }
    }

    fn schedule_atimer(&mut self, t: Atimer) {
        let at = self
            .atimers
            .iter()
            .position(|a| a.expiration >= t.expiration)
            .unwrap_or(self.atimers.len());
        self.atimers.insert(at, t);
    }

    fn setitimer(&mut self, interval: Option<Timespec>) {
        let expire = interval.map(|(sec, nsec)| {
            Instant::now() + Duration::new(sec.max(0) as u64, nsec.clamp(0, 999_999_999) as u32)
        });
        if expire.is_none() && self.itimer.is_none() {
            return;
        }
        let pending_signals = Arc::clone(&self.pending_signals);
        self.itimer
            .get_or_insert_with(|| Itimer::start(pending_signals))
            .set(expire);
    }

    fn set_alarm(&mut self) {
        if let Some(first) = self.atimers.first() {
            let now = current_timespec();
            if first.expiration <= now {
                self.pending_signals.store(true, Ordering::SeqCst);
            } else {
                let interval = timespec_sub(first.expiration, now);
                self.setitimer(Some(interval));
            }
        }
    }
}

impl Lisp {
    pub fn start_atimer(
        &mut self,
        type_: AtimerType,
        timestamp: Timespec,
        fn_: AtimerCallback,
        client_data: Value,
    ) -> AtimerId {
        let a = &mut self.atimer;
        let id = AtimerId(a.next_id);
        a.next_id += 1;
        let (expiration, interval) = match type_ {
            AtimerType::Absolute => (timestamp, (0, 0)),
            AtimerType::Relative => (timespec_add(current_timespec(), timestamp), (0, 0)),
            AtimerType::Continuous => (timespec_add(current_timespec(), timestamp), timestamp),
        };
        a.schedule_atimer(Atimer {
            id,
            type_,
            expiration,
            interval,
            fn_,
            client_data,
        });
        a.set_alarm();
        id
    }

    pub fn cancel_atimer(&mut self, timer: AtimerId) {
        let a = &mut self.atimer;
        for list in [&mut a.atimers, &mut a.stopped_atimers] {
            if let Some(i) = list.iter().position(|t| t.id == timer) {
                list.remove(i);
                break;
            }
        }
    }

    pub fn stop_other_atimers(&mut self, t: Option<AtimerId>) {
        let a = &mut self.atimer;
        let kept = t.and_then(|t| {
            a.atimers
                .iter()
                .position(|p| p.id == t)
                .map(|i| a.atimers.remove(i))
        });
        let mut stopped = std::mem::take(&mut a.atimers);
        stopped.append(&mut a.stopped_atimers);
        a.stopped_atimers = stopped;
        a.atimers.extend(kept);
    }

    pub fn run_all_atimers(&mut self) {
        let a = &mut self.atimer;
        if !a.stopped_atimers.is_empty() {
            let running = std::mem::replace(&mut a.atimers, std::mem::take(&mut a.stopped_atimers));
            for t in running {
                a.schedule_atimer(t);
            }
        }
    }

    fn run_timers(&mut self) -> LispResult<()> {
        let now = current_timespec();
        while self
            .atimer
            .atimers
            .first()
            .is_some_and(|t| t.expiration <= now)
        {
            let mut t = self.atimer.atimers.remove(0);
            self.stack.push(t.client_data);
            let r = (t.fn_)(self, &t);
            self.stack.pop();
            if t.type_ == AtimerType::Continuous {
                t.expiration = timespec_add(now, t.interval);
                self.atimer.schedule_atimer(t);
            }
            if r.is_err() {
                self.atimer.set_alarm();
                return r;
            }
        }
        self.atimer.set_alarm();
        Ok(())
    }

    pub fn do_pending_atimers(&mut self) -> LispResult<()> {
        if self.atimer.atimers.is_empty() {
            return Ok(());
        }
        self.run_timers()
    }

    pub fn turn_on_atimers(&mut self, on: bool) {
        if on {
            self.atimer.set_alarm();
        } else {
            self.atimer.setitimer(None);
        }
    }

    pub fn pending_signals(&self) -> bool {
        self.atimer.pending_signals.load(Ordering::SeqCst)
    }

    pub fn process_pending_signals(&mut self) -> LispResult<()> {
        self.atimer.pending_signals.store(false, Ordering::SeqCst);
        self.do_pending_atimers()
    }
}
