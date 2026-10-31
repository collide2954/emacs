use crate::lisp::Lisp;
use crate::timefns::current_timespec;

const RAND_BITS: u32 = 31;
const EMACS_INT_WIDTH: u32 = 64;
const FIXNUM_BITS: u32 = 62;
const INTMASK: u64 = (1 << FIXNUM_BITS) - 1;

#[cfg(windows)]
pub struct Random {
    ix: i32,
    iy: i32,
    iz: i32,
    holdrand: u32,
}

#[cfg(windows)]
impl Default for Random {
    fn default() -> Self {
        Random {
            ix: 3172,
            iy: 9814,
            iz: 20125,
            holdrand: 1,
        }
    }
}

#[cfg(windows)]
const RAND_MAX_X: i32 = 30269;
#[cfg(windows)]
const RAND_MAX_Y: i32 = 30307;
#[cfg(windows)]
const RAND_MAX_Z: i32 = 30323;

#[cfg(windows)]
impl Random {
    fn rand(&mut self) -> i32 {
        self.holdrand = self.holdrand.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((self.holdrand >> 16) & 0x7fff) as i32
    }

    fn rand_as183(&mut self) -> i32 {
        self.ix = (171 * self.ix) % RAND_MAX_X;
        self.iy = (172 * self.iy) % RAND_MAX_Y;
        self.iz = (170 * self.iz) % RAND_MAX_Z;
        (self.ix + self.iy + self.iz) & 0x7fff
    }

    pub fn random(&mut self) -> i64 {
        let a = self.rand_as183();
        let b = self.rand_as183();
        let c = self.rand_as183();
        i64::from(a.wrapping_shl(30) | b.wrapping_shl(15) | c)
    }

    pub fn srandom(&mut self, seed: u32) {
        self.holdrand = seed;
        self.ix = self.rand() % RAND_MAX_X;
        self.iy = self.rand() % RAND_MAX_Y;
        self.iz = self.rand() % RAND_MAX_Z;
    }
}

#[cfg(unix)]
const RAND_DEG: usize = 31;
#[cfg(unix)]
const RAND_SEP: usize = 3;

#[cfg(unix)]
pub struct Random {
    state: [i32; RAND_DEG],
    fptr: usize,
    rptr: usize,
}

#[cfg(unix)]
impl Default for Random {
    fn default() -> Self {
        let mut r = Random {
            state: [0; RAND_DEG],
            fptr: RAND_SEP,
            rptr: 0,
        };
        r.srandom(1);
        r
    }
}

#[cfg(unix)]
impl Random {
    pub fn random(&mut self) -> i64 {
        let val = (self.state[self.fptr] as u32).wrapping_add(self.state[self.rptr] as u32);
        self.state[self.fptr] = val as i32;
        let result = val >> 1;
        self.fptr += 1;
        if self.fptr >= RAND_DEG {
            self.fptr = 0;
            self.rptr += 1;
        } else {
            self.rptr += 1;
            if self.rptr >= RAND_DEG {
                self.rptr = 0;
            }
        }
        i64::from(result)
    }

    pub fn srandom(&mut self, seed: u32) {
        let seed = if seed == 0 { 1 } else { seed };
        let mut word = seed as i32;
        self.state[0] = word;
        for i in 1..RAND_DEG {
            let hi = i64::from(word) / 127_773;
            let lo = i64::from(word) % 127_773;
            word = (16_807 * lo - 2_836 * hi) as i32;
            if word < 0 {
                word = word.wrapping_add(2_147_483_647);
            }
            self.state[i] = word;
        }
        self.fptr = RAND_SEP;
        self.rptr = 0;
        for _ in 0..RAND_DEG * 10 {
            self.random();
        }
    }
}

#[cfg(windows)]
fn os_random_seed() -> Option<u32> {
    let mut buf = [0u8; 4];
    emacs_w32::crypt_gen_random(&mut buf).then(|| u32::from_ne_bytes(buf))
}

#[cfg(unix)]
fn os_random_seed() -> Option<u32> {
    let mut buf = [0u8; 4];
    emacs_unix::getrandom(&mut buf).then(|| u32::from_ne_bytes(buf))
}

pub fn init_random(lisp: &mut Lisp) {
    let v = os_random_seed().unwrap_or_else(|| {
        let (sec, nsec) = current_timespec();
        (i64::from(std::process::id()) ^ sec ^ nsec) as u32
    });
    lisp.random.srandom(v);
}

impl Lisp {
    pub fn seed_random(&mut self, seed: &[u8]) {
        let mut arg = [0u8; 4];
        for (i, b) in seed.iter().enumerate() {
            arg[i % arg.len()] ^= b;
        }
        self.random.srandom(u32::from_ne_bytes(arg));
    }

    pub fn get_random(&mut self) -> i64 {
        let mut val: u64 = 0;
        for _ in 0..FIXNUM_BITS.div_ceil(RAND_BITS) {
            val = (self.random.random() as u64)
                ^ (val << RAND_BITS)
                ^ (val >> (EMACS_INT_WIDTH - RAND_BITS));
        }
        val ^= val >> (EMACS_INT_WIDTH - FIXNUM_BITS);
        (val & INTMASK) as i64
    }

    #[cfg(windows)]
    pub fn get_random_ulong(&mut self) -> u64 {
        const ULONG_WIDTH: u32 = 32;
        let mut r: u32 = 0;
        for _ in 0..ULONG_WIDTH.div_ceil(RAND_BITS) {
            r = (self.random.random() as u32) ^ (r << RAND_BITS) ^ (r >> (ULONG_WIDTH - RAND_BITS));
        }
        u64::from(r)
    }

    #[cfg(unix)]
    pub fn get_random_ulong(&mut self) -> u64 {
        const ULONG_WIDTH: u32 = 64;
        let mut r: u64 = 0;
        for _ in 0..ULONG_WIDTH.div_ceil(RAND_BITS) {
            r = (self.random.random() as u64) ^ (r << RAND_BITS) ^ (r >> (ULONG_WIDTH - RAND_BITS));
        }
        r
    }
}
