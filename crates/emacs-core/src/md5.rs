pub const MD5_DIGEST_SIZE: usize = 16;

const T: [u32; 64] = [
    0xd76a_a478,
    0xe8c7_b756,
    0x2420_70db,
    0xc1bd_ceee,
    0xf57c_0faf,
    0x4787_c62a,
    0xa830_4613,
    0xfd46_9501,
    0x6980_98d8,
    0x8b44_f7af,
    0xffff_5bb1,
    0x895c_d7be,
    0x6b90_1122,
    0xfd98_7193,
    0xa679_438e,
    0x49b4_0821,
    0xf61e_2562,
    0xc040_b340,
    0x265e_5a51,
    0xe9b6_c7aa,
    0xd62f_105d,
    0x0244_1453,
    0xd8a1_e681,
    0xe7d3_fbc8,
    0x21e1_cde6,
    0xc337_07d6,
    0xf4d5_0d87,
    0x455a_14ed,
    0xa9e3_e905,
    0xfcef_a3f8,
    0x676f_02d9,
    0x8d2a_4c8a,
    0xfffa_3942,
    0x8771_f681,
    0x6d9d_6122,
    0xfde5_380c,
    0xa4be_ea44,
    0x4bde_cfa9,
    0xf6bb_4b60,
    0xbebf_bc70,
    0x289b_7ec6,
    0xeaa1_27fa,
    0xd4ef_3085,
    0x0488_1d05,
    0xd9d4_d039,
    0xe6db_99e5,
    0x1fa2_7cf8,
    0xc4ac_5665,
    0xf429_2244,
    0x432a_ff97,
    0xab94_23a7,
    0xfc93_a039,
    0x655b_59c3,
    0x8f0c_cc92,
    0xffef_f47d,
    0x8584_5dd1,
    0x6fa8_7e4f,
    0xfe2c_e6e0,
    0xa301_4314,
    0x4e08_11a1,
    0xf753_7e82,
    0xbd3a_f235,
    0x2ad7_d2bb,
    0xeb86_d391,
];

const S: [[u32; 4]; 4] = [
    [7, 12, 17, 22],
    [5, 9, 14, 20],
    [4, 11, 16, 23],
    [6, 10, 15, 21],
];

pub struct Md5Ctx {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    total: u64,
    buffer: Vec<u8>,
}

impl Default for Md5Ctx {
    fn default() -> Self {
        Md5Ctx {
            a: 0x6745_2301,
            b: 0xefcd_ab89,
            c: 0x98ba_dcfe,
            d: 0x1032_5476,
            total: 0,
            buffer: Vec::with_capacity(64),
        }
    }
}

impl Md5Ctx {
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
            *word = u32::from_le_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (self.a, self.b, self.c, self.d);
        for i in 0..64 {
            let (f, k) = match i / 16 {
                0 => (d ^ (b & (c ^ d)), i),
                1 => (c ^ (d & (b ^ c)), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let t = a
                .wrapping_add(f)
                .wrapping_add(x[k])
                .wrapping_add(T[i])
                .rotate_left(S[i / 16][i % 4])
                .wrapping_add(b);
            a = d;
            d = c;
            c = b;
            b = t;
        }
        self.a = self.a.wrapping_add(a);
        self.b = self.b.wrapping_add(b);
        self.c = self.c.wrapping_add(c);
        self.d = self.d.wrapping_add(d);
    }

    pub fn finish(mut self) -> [u8; MD5_DIGEST_SIZE] {
        let bits = self.total.wrapping_mul(8);
        let mut pad = vec![0x80u8];
        while (self.buffer.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bits.to_le_bytes());
        self.process_bytes(&pad);
        let mut digest = [0u8; MD5_DIGEST_SIZE];
        for (i, word) in [self.a, self.b, self.c, self.d].iter().enumerate() {
            digest[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
        }
        digest
    }
}

pub fn md5_buffer(buffer: &[u8]) -> Vec<u8> {
    let mut ctx = Md5Ctx::default();
    ctx.process_bytes(buffer);
    ctx.finish().to_vec()
}
