pub const SHA3_224_DIGEST_SIZE: usize = 28;
pub const SHA3_256_DIGEST_SIZE: usize = 32;
pub const SHA3_384_DIGEST_SIZE: usize = 48;
pub const SHA3_512_DIGEST_SIZE: usize = 64;

pub const SHA3_224_BLOCK_SIZE: usize = 144;
pub const SHA3_256_BLOCK_SIZE: usize = 136;
pub const SHA3_384_BLOCK_SIZE: usize = 104;
pub const SHA3_512_BLOCK_SIZE: usize = 72;

const RC: [u64; 24] = [
    0x0000_0000_0000_0001,
    0x0000_0000_0000_8082,
    0x8000_0000_0000_808A,
    0x8000_0000_8000_8000,
    0x0000_0000_0000_808B,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8009,
    0x0000_0000_0000_008A,
    0x0000_0000_0000_0088,
    0x0000_0000_8000_8009,
    0x0000_0000_8000_000A,
    0x0000_0000_8000_808B,
    0x8000_0000_0000_008B,
    0x8000_0000_0000_8089,
    0x8000_0000_0000_8003,
    0x8000_0000_0000_8002,
    0x8000_0000_0000_0080,
    0x0000_0000_0000_800A,
    0x8000_0000_8000_000A,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8080,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8008,
];

const PI: [usize; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
];

const RHO: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
];

pub struct Sha3Ctx {
    state: [u64; 25],
    buffer: Vec<u8>,
    digestlen: usize,
    blocklen: usize,
}

impl Sha3Ctx {
    pub fn new(digestlen: usize, blocklen: usize) -> Self {
        Sha3Ctx {
            state: [0; 25],
            buffer: Vec::with_capacity(blocklen),
            digestlen,
            blocklen,
        }
    }

    pub fn process_bytes(&mut self, mut data: &[u8]) {
        if !self.buffer.is_empty() {
            let left = self.blocklen - self.buffer.len();
            if data.len() < left {
                self.buffer.extend_from_slice(data);
                return;
            }
            self.buffer.extend_from_slice(&data[..left]);
            data = &data[left..];
            let block = std::mem::take(&mut self.buffer);
            self.process_block(&block);
            self.buffer = block;
            self.buffer.clear();
        }
        while data.len() >= self.blocklen {
            self.process_block(&data[..self.blocklen]);
            data = &data[self.blocklen..];
        }
        self.buffer.extend_from_slice(data);
    }

    fn process_block(&mut self, block: &[u8]) {
        let a = &mut self.state;
        for (lane, chunk) in a.iter_mut().zip(block.as_chunks::<8>().0) {
            *lane ^= u64::from_le_bytes(*chunk);
        }
        for rc in RC {
            let mut c = [0u64; 5];
            for (x, cx) in c.iter_mut().enumerate() {
                *cx = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
            }
            for x in 0..5 {
                let d = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
                for y in (0..25).step_by(5) {
                    a[y + x] ^= d;
                }
            }
            let mut last = a[1];
            for (&p, &r) in PI.iter().zip(RHO.iter()) {
                let t = a[p];
                a[p] = last.rotate_left(r);
                last = t;
            }
            for j in (0..25).step_by(5) {
                let t1 = a[j];
                let t2 = a[j + 1];
                a[j] ^= !a[j + 1] & a[j + 2];
                a[j + 1] ^= !a[j + 2] & a[j + 3];
                a[j + 2] ^= !a[j + 3] & a[j + 4];
                a[j + 3] ^= !a[j + 4] & t1;
                a[j + 4] ^= !t1 & t2;
            }
            a[0] ^= rc;
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        let mut block = std::mem::take(&mut self.buffer);
        block.push(0x06);
        block.resize(self.blocklen, 0);
        block[self.blocklen - 1] |= 0x80;
        self.process_block(&block);
        self.state
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .take(self.digestlen)
            .collect()
    }
}

fn sha3_buffer(buffer: &[u8], digestlen: usize, blocklen: usize) -> Vec<u8> {
    let mut ctx = Sha3Ctx::new(digestlen, blocklen);
    ctx.process_bytes(buffer);
    ctx.finish()
}

pub fn sha3_224_buffer(buffer: &[u8]) -> Vec<u8> {
    sha3_buffer(buffer, SHA3_224_DIGEST_SIZE, SHA3_224_BLOCK_SIZE)
}

pub fn sha3_256_buffer(buffer: &[u8]) -> Vec<u8> {
    sha3_buffer(buffer, SHA3_256_DIGEST_SIZE, SHA3_256_BLOCK_SIZE)
}

pub fn sha3_384_buffer(buffer: &[u8]) -> Vec<u8> {
    sha3_buffer(buffer, SHA3_384_DIGEST_SIZE, SHA3_384_BLOCK_SIZE)
}

pub fn sha3_512_buffer(buffer: &[u8]) -> Vec<u8> {
    sha3_buffer(buffer, SHA3_512_DIGEST_SIZE, SHA3_512_BLOCK_SIZE)
}
