pub const SHA224_DIGEST_SIZE: usize = 28;
pub const SHA256_DIGEST_SIZE: usize = 32;

const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

pub struct Sha256Ctx {
    state: [u32; 8],
    total: u64,
    buffer: Vec<u8>,
    digest_size: usize,
}

impl Sha256Ctx {
    pub fn sha256() -> Self {
        Sha256Ctx {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            total: 0,
            buffer: Vec::with_capacity(64),
            digest_size: SHA256_DIGEST_SIZE,
        }
    }

    pub fn sha224() -> Self {
        Sha256Ctx {
            state: [
                0xc105_9ed8,
                0x367c_d507,
                0x3070_dd17,
                0xf70e_5939,
                0xffc0_0b31,
                0x6858_1511,
                0x64f9_8fa7,
                0xbefa_4fa4,
            ],
            total: 0,
            buffer: Vec::with_capacity(64),
            digest_size: SHA224_DIGEST_SIZE,
        }
    }

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
        let mut v = self.state;
        for t in 0..64 {
            let m = if t < 16 {
                x[t]
            } else {
                let w2 = x[(t - 2) & 0x0f];
                let w15 = x[(t - 15) & 0x0f];
                let s1 = w2.rotate_left(15) ^ w2.rotate_left(13) ^ (w2 >> 10);
                let s0 = w15.rotate_left(25) ^ w15.rotate_left(14) ^ (w15 >> 3);
                let tm = s1
                    .wrapping_add(x[(t - 7) & 0x0f])
                    .wrapping_add(s0)
                    .wrapping_add(x[t & 0x0f]);
                x[t & 0x0f] = tm;
                tm
            };
            let [a, b, c, d, e, f, g, h] = v;
            let ss0 = a.rotate_left(30) ^ a.rotate_left(19) ^ a.rotate_left(10);
            let ss1 = e.rotate_left(26) ^ e.rotate_left(21) ^ e.rotate_left(7);
            let t0 = ss0.wrapping_add((a & b) | (c & (a | b)));
            let t1 = h
                .wrapping_add(ss1)
                .wrapping_add(g ^ (e & (f ^ g)))
                .wrapping_add(K[t])
                .wrapping_add(m);
            v = [t0.wrapping_add(t1), a, b, c, d.wrapping_add(t1), e, f, g];
        }
        for (s, x) in self.state.iter_mut().zip(v) {
            *s = s.wrapping_add(x);
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        let bits = self.total.wrapping_mul(8);
        let mut pad = vec![0x80u8];
        while (self.buffer.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bits.to_be_bytes());
        self.process_bytes(&pad);
        self.state
            .iter()
            .flat_map(|w| w.to_be_bytes())
            .take(self.digest_size)
            .collect()
    }
}

pub fn sha224_buffer(buffer: &[u8]) -> Vec<u8> {
    let mut ctx = Sha256Ctx::sha224();
    ctx.process_bytes(buffer);
    ctx.finish()
}

pub fn sha256_buffer(buffer: &[u8]) -> Vec<u8> {
    let mut ctx = Sha256Ctx::sha256();
    ctx.process_bytes(buffer);
    ctx.finish()
}
