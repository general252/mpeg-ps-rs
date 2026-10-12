pub const BIT_MASK: [u8; 8] = [0x01, 0x03, 0x07, 0x0F, 0x1F, 0x3F, 0x7F, 0xFF];

pub struct BitStream<'a> {
    bits: &'a [u8],
    bytes_offset: usize,
    bits_offset: usize,
    bitsmark: usize,
    bytemark: usize,
}

impl<'a> BitStream<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            bits: buf,
            bytes_offset: 0,
            bits_offset: 0,
            bitsmark: 0,
            bytemark: 0,
        }
    }

    pub fn remain_bytes(&self) -> usize {
        if self.bits_offset > 0 {
            self.bits.len() - self.bytes_offset - 1
        } else {
            self.bits.len() - self.bytes_offset
        }
    }

    pub fn remain_bits(&self) -> usize {
        if self.bits_offset > 0 {
            self.remain_bytes() * 8 + 8 - self.bits_offset
        } else {
            self.remain_bytes() * 8
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.bytes_offset
    }

    pub fn get_bits(&mut self, n: usize) -> u64 {
        assert!(n <= 64, "get_bits: n must be <= 64");
        assert!(self.bytes_offset < self.bits.len(), "get_bits: out of range");

        let mut ret: u64;
        if 8 - self.bits_offset >= n {
            let mask = if n > 0 {
                BIT_MASK[n - 1] as u64
            } else {
                0
            };
            ret = (self.bits[self.bytes_offset] as u64 >> (8 - self.bits_offset - n)) & mask;
            self.bits_offset += n;
            if self.bits_offset == 8 {
                self.bytes_offset += 1;
                self.bits_offset = 0;
            }
        } else {
            let mask = BIT_MASK[8 - self.bits_offset - 1] as u64;
            ret = (self.bits[self.bytes_offset] as u64) & mask;
            self.bytes_offset += 1;
            let mut remaining = n - (8 - self.bits_offset);
            self.bits_offset = 0;
            while remaining > 0 {
                assert!(self.bytes_offset < self.bits.len(), "get_bits: out of range");
                if remaining >= 8 {
                    ret = (ret << 8) | (self.bits[self.bytes_offset] as u64);
                    self.bytes_offset += 1;
                    remaining -= 8;
                } else {
                    let mask = BIT_MASK[remaining - 1] as u64;
                    ret = (ret << remaining) | ((self.bits[self.bytes_offset] as u64 >> (8 - remaining)) & mask);
                    self.bits_offset = remaining;
                    break;
                }
            }
        }
        ret
    }

    pub fn get_bit(&mut self) -> u8 {
        if self.bytes_offset >= self.bits.len() {
            panic!("get_bit: out of range");
        }
        let ret = self.bits[self.bytes_offset] >> (7 - self.bits_offset) & 0x01;
        self.bits_offset += 1;
        if self.bits_offset >= 8 {
            self.bytes_offset += 1;
            self.bits_offset = 0;
        }
        ret
    }

    pub fn uint8(&mut self, n: usize) -> u8 {
        self.get_bits(n) as u8
    }

    pub fn uint16(&mut self, n: usize) -> u16 {
        self.get_bits(n) as u16
    }

    pub fn uint32(&mut self, n: usize) -> u32 {
        self.get_bits(n) as u32
    }

    pub fn get_bytes(&mut self, n: usize) -> &'a [u8] {
        assert!(
            self.bytes_offset + n <= self.bits.len(),
            "get_bytes: out of range"
        );
        assert!(self.bits_offset == 0, "get_bytes: invalid operation, bits_offset must be 0");
        let data = &self.bits[self.bytes_offset..self.bytes_offset + n];
        self.bytes_offset += n;
        data
    }

    pub fn skip_bits(&mut self, n: usize) {
        let bytecount = n / 8;
        let bitscount = n % 8;
        self.bytes_offset += bytecount;
        self.bits_offset += bitscount;
        if self.bits_offset >= 8 {
            self.bytes_offset += 1;
            self.bits_offset -= 8;
        }
    }

    pub fn mark(&mut self) {
        self.bitsmark = self.bits_offset;
        self.bytemark = self.bytes_offset;
    }

    pub fn distance_from_mark(&self) -> usize {
        let bytecount = self.bytes_offset - self.bytemark - 1;
        let bitscount = self.bits_offset + (8 - self.bitsmark);
        bytecount * 8 + bitscount
    }

    pub fn remain_data(&self) -> &'a [u8] {
        &self.bits[self.bytes_offset..]
    }

    pub fn unread(&mut self, n: usize) {
        if n as isize - self.bits_offset as isize <= 0 {
            self.bits_offset -= n;
        } else {
            let mut least = n - self.bits_offset;
            while least >= 8 {
                self.bytes_offset -= 1;
                least -= 8;
            }
            if least > 0 {
                self.bytes_offset -= 1;
                self.bits_offset = 8 - least;
            }
        }
    }

    pub fn next_bits(&mut self, n: usize) -> u64 {
        let r = self.get_bits(n);
        self.unread(n);
        r
    }

    pub fn eos(&self) -> bool {
        self.bytes_offset == self.bits.len() && self.bits_offset == 0
    }

    pub fn bits(&self) -> &[u8] {
        self.bits
    }

    pub fn read_ue(&mut self) -> u64 {
        let mut leading_zero_bits: u64 = 0;
        while self.get_bit() == 0 {
            leading_zero_bits += 1;
        }
        if leading_zero_bits == 0 {
            return 0;
        }
        let info = self.get_bits(leading_zero_bits as usize);
        (1u64 << leading_zero_bits) - 1 + info
    }

    pub fn read_se(&mut self) -> i64 {
        let v = self.read_ue();
        if v % 2 == 0 {
            -1 * (v / 2) as i64
        } else {
            ((v + 1) / 2) as i64
        }
    }
}

pub struct BitStreamWriter {
    bits: Vec<u8>,
    byteoffset: usize,
    bitsoffset: usize,
    bitsmark: usize,
    bytemark: usize,
}

impl BitStreamWriter {
    pub fn new(n: usize) -> Self {
        Self {
            bits: vec![0u8; n],
            byteoffset: 0,
            bitsoffset: 0,
            bitsmark: 0,
            bytemark: 0,
        }
    }

    fn expand_space(&mut self, n: usize) {
        let available = (self.bits.len() - self.byteoffset - 1) * 8 + 8 - self.bitsoffset;
        if available < n {
            let newlen = if self.bits.len() * 8 < n {
                self.bits.len() + n / 8 + 1
            } else {
                self.bits.len() * 2
            };
            let mut tmp = vec![0u8; newlen];
            tmp[..self.bits.len()].copy_from_slice(&self.bits);
            self.bits = tmp;
        }
    }

    pub fn byte_offset(&self) -> usize {
        self.byteoffset
    }

    pub fn bit_offset(&self) -> usize {
        self.bitsoffset
    }

    pub fn mark(&mut self) {
        self.bitsmark = self.bitsoffset;
        self.bytemark = self.byteoffset;
    }

    pub fn distance_from_mark(&self) -> usize {
        let bytecount = self.byteoffset - self.bytemark - 1;
        let bitscount = self.bitsoffset + (8 - self.bitsmark);
        bytecount * 8 + bitscount
    }

    pub fn put_byte(&mut self, v: u8) {
        self.expand_space(8);
        if self.bitsoffset == 0 {
            self.bits[self.byteoffset] = v;
            self.byteoffset += 1;
        } else {
            self.bits[self.byteoffset] |= v >> (self.bitsoffset as u8);
            self.byteoffset += 1;
            self.bits[self.byteoffset] = v & BIT_MASK[self.bitsoffset - 1];
        }
    }

    pub fn put_bytes(&mut self, v: &[u8]) {
        assert_eq!(self.bitsoffset, 0, "put_bytes: bitsoffset must be 0");
        self.expand_space(8 * v.len());
        self.bits[self.byteoffset..self.byteoffset + v.len()].copy_from_slice(v);
        self.byteoffset += v.len();
    }

    pub fn put_repeat_value(&mut self, v: u8, n: usize) {
        assert_eq!(self.bitsoffset, 0, "put_repeat_value: bitsoffset must be 0");
        self.expand_space(8 * n);
        for i in 0..n {
            self.bits[self.byteoffset + i] = v;
        }
        self.byteoffset += n;
    }

    pub fn put_uint8(&mut self, v: u8, n: usize) {
        self.put_uint64(v as u64, n);
    }

    pub fn put_uint16(&mut self, v: u16, n: usize) {
        self.put_uint64(v as u64, n);
    }

    pub fn put_uint32(&mut self, v: u32, n: usize) {
        self.put_uint64(v as u64, n);
    }

    pub fn put_uint64(&mut self, v: u64, n: usize) {
        self.expand_space(n);
        if 8 - self.bitsoffset >= n {
            let mask = if n > 0 { BIT_MASK[n - 1] as u64 } else { 0 };
            self.bits[self.byteoffset] |= ((v & mask) << (8 - self.bitsoffset - n)) as u8;
            self.bitsoffset += n;
            if self.bitsoffset == 8 {
                self.bitsoffset = 0;
                self.byteoffset += 1;
            }
        } else {
            let shift = (n - (8 - self.bitsoffset)) as u32;
            self.bits[self.byteoffset] |= ((v >> shift) & BIT_MASK[8 - self.bitsoffset - 1] as u64) as u8;
            self.byteoffset += 1;
            let mut remaining = n - (8 - self.bitsoffset);
            while remaining >= 8 {
                self.bits[self.byteoffset] = ((v >> (remaining - 8)) & 0xFF) as u8;
                self.byteoffset += 1;
                remaining -= 8;
            }
            self.bitsoffset = remaining;
            if remaining > 0 {
                self.bits[self.byteoffset] |= ((v & (BIT_MASK[remaining - 1] as u64)) << (8 - remaining)) as u8;
            }
        }
    }

    pub fn set_byte(&mut self, v: u8, where_: usize) {
        self.bits[where_] = v;
    }

    pub fn set_uint16(&mut self, v: u16, where_: usize) {
        self.bits[where_] = (v >> 8) as u8;
        self.bits[where_ + 1] = (v & 0xFF) as u8;
    }

    pub fn bits(&self) -> &[u8] {
        if self.byteoffset == self.bits.len() {
            &self.bits
        } else if self.bitsoffset > 0 {
            &self.bits[0..self.byteoffset + 1]
        } else {
            &self.bits[0..self.byteoffset]
        }
    }

    pub fn bits_mut(&mut self) -> &mut [u8] {
        &mut self.bits[0..self.byteoffset]
    }

    pub fn fill_remain_data(&mut self, v: u8) {
        for i in self.byteoffset..self.bits.len() {
            self.bits[i] = v;
        }
        self.byteoffset = self.bits.len();
        self.bitsoffset = 0;
    }

    pub fn reset(&mut self) {
        for b in self.bits.iter_mut() {
            *b = 0;
        }
        self.bitsmark = 0;
        self.bytemark = 0;
        self.bitsoffset = 0;
        self.byteoffset = 0;
    }
}
