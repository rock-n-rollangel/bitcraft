use crate::errors::ParseError;

pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    fn read_bit(&mut self) -> Result<u8, ParseError> {
        let byte_index = self.bit_pos / 8;
        let bit_index = self.bit_pos % 8;

        let byte = *self.data.get(byte_index).ok_or(ParseError::OutOfBounds)?;
        let bit = (byte >> (7 - bit_index)) & 1;

        self.bit_pos += 1;

        Ok(bit)
    }

    pub fn read_bits(&mut self, n: usize) -> Result<u64, ParseError> {
        if n > 64 {
            return Err(ParseError::TooManyBitsRead);
        }

        let needed_bits = self.bit_pos + n;
        if needed_bits > self.data.len() * 8 {
            return Err(ParseError::OutOfBounds);
        }

        let mut value = 0u64;

        for _ in 0..n {
            let bit = self.read_bit()? as u64;
            value = (value << 1) | bit;
        }

        Ok(value)
    }

    pub fn read_bits_at(&self, bit_pos: usize, n: usize) -> Result<u64, ParseError> {
        let mut reader = BitReader {
            data: self.data,
            bit_pos,
        };

        reader.read_bits(n)
    }

    pub fn skip_bits(&mut self, n: usize) {
        self.bit_pos += n;
    }

    pub fn align_to(&mut self, bits: usize) {
        if bits == 0 {
            return;
        }

        let rem = self.bit_pos % bits;
        if rem != 0 {
            self.bit_pos += bits - rem;
        }
    }
}

pub fn sign_extend(value: u64, bits: usize) -> i64 {
    let shift = 64 - bits;
    ((value << shift) as i64) >> shift
}

pub fn reverse_bits_n(mut x: u64, n: usize) -> u64 {
    let mut r = 0u64;
    for _ in 0..n {
        r = (r << 1) | (x & 1);
        x >>= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bit() {
        let mut bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bit().unwrap(), 1);
    }

    #[test]
    fn test_read_bits() {
        let mut bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bits(8).unwrap(), 0b11111111);
    }

    #[test]
    fn test_read_bits_at() {
        let bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bits_at(0, 8).unwrap(), 0b11111111);
    }

    #[test]
    fn test_skip_bits() {
        let mut bit_reader = BitReader::new(&[0b11111111]);
        bit_reader.skip_bits(8);
        assert_eq!(bit_reader.bit_pos, 8);
    }

    #[test]
    fn test_align_to() {
        let mut bit_reader = BitReader::new(&[0b11111111]);
        bit_reader.skip_bits(2);

        bit_reader.align_to(4);
        assert_eq!(bit_reader.bit_pos, 4);

        bit_reader.align_to(8);
        assert_eq!(bit_reader.bit_pos, 8);
    }

    #[test]
    fn test_read_bits_out_of_bounds() {
        let mut bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bits(9).unwrap_err(), ParseError::OutOfBounds);
    }

    #[test]
    fn test_read_bits_more_than_64() {
        let mut bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bits(65).unwrap_err(), ParseError::TooManyBitsRead);
    }

    #[test]
    fn test_read_bits_at_out_of_bounds() {
        let bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bits_at(0, 9).unwrap_err(), ParseError::OutOfBounds);
    }

    #[test]
    fn test_read_bits_at_more_than_64() {
        let bit_reader = BitReader::new(&[0b11111111]);
        assert_eq!(bit_reader.read_bits_at(0, 65).unwrap_err(), ParseError::TooManyBitsRead);
    }

    #[test]
    fn test_sign_extend() {
        assert_eq!(sign_extend(0b11111111, 8), -1);
    }

    #[test]
    fn test_reverse_bits_n() {
        assert_eq!(reverse_bits_n(0b10101010, 8), 0b01010101);
    }
}
