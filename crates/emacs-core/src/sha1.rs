pub const SHA1_DIGEST_SIZE: usize = 20;

const K1: u32 = 0x5a82_7999;
const K2: u32 = 0x6ed9_eba1;
const K3: u32 = 0x8f1b_bcdc;
const K4: u32 = 0xca62_c1d6;

pub struct Sha1Ctx {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    total: u64,
    buffer: Vec<u8>,
}

impl Default for Sha1Ctx {
    fn default() -> Self {
        Sha1Ctx {
            a: 0x6745_2301,
            b: 0xefcd_ab89,
            c: 0x98ba_dcfe,
            d: 0x1032_5476,
            e: 0xc3d2_e1f0,
            total: 0,
            buffer: Vec::with_capacity(64),
        }
    }
}

impl Sha1Ctx {
    pub fn process_bytes(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        if !self.buffer.is_empty() {
            let take = (64 - self.buffer.len()).min(data.len());
            self.buffer.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.buffer.len() == 64 {
                let block = std::mem::take(&mut self.buffer);
                self.process_block(&block);
                self.buffer = block;
                self.buffer.clear();
            }
        }
        while data.len() >= 64 {
            self.process_block(&data[..64]);
            data = &data[64..];
        }
        self.buffer.extend_from_slice(data);
    }

    fn process_block(&mut self, block: &[u8]) {
        let mut x = [0u32; 16];
        for (i, word) in x.iter_mut().enumerate() {
            *word = u32::from_be_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (self.a, self.b, self.c, self.d, self.e);
        for i in 0..80 {
            let m = if i < 16 {
                x[i]
            } else {
                let tm = x[i & 0x0f] ^ x[(i - 14) & 0x0f] ^ x[(i - 8) & 0x0f] ^ x[(i - 3) & 0x0f];
                x[i & 0x0f] = tm.rotate_left(1);
                x[i & 0x0f]
            };
            let (f, k) = match i / 20 {
                0 => (d ^ (b & (c ^ d)), K1),
                1 => (b ^ c ^ d, K2),
                2 => ((b & c) | (d & (b | c)), K3),
                _ => (b ^ c ^ d, K4),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(m);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        self.a = self.a.wrapping_add(a);
        self.b = self.b.wrapping_add(b);
        self.c = self.c.wrapping_add(c);
        self.d = self.d.wrapping_add(d);
        self.e = self.e.wrapping_add(e);
    }

    pub fn finish(mut self) -> [u8; SHA1_DIGEST_SIZE] {
        let bits = self.total.wrapping_mul(8);
        let mut pad = vec![0x80u8];
        while (self.buffer.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bits.to_be_bytes());
        self.process_bytes(&pad);
        let mut digest = [0u8; SHA1_DIGEST_SIZE];
        for (i, word) in [self.a, self.b, self.c, self.d, self.e].iter().enumerate() {
            digest[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}
