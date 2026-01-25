#[derive(Debug, Clone, Copy)]
pub struct Fragment {
    pub offset_bits: usize,
    pub len_bits: usize,
    pub bit_order: crate::assembly::BitOrder,
}

impl Fragment {
    pub fn new(offset_bits: usize, len_bits: usize) -> Self {
        Fragment {
            offset_bits,
            len_bits,
            bit_order: Default::default(),
        }
    }

    pub fn new_with_bit_order(
        offset_bits: usize,
        len_bits: usize,
        bit_order: crate::assembly::BitOrder,
    ) -> Self {
        Fragment {
            offset_bits,
            len_bits,
            bit_order,
        }
    }
}

impl Default for Fragment {
    fn default() -> Self {
        Fragment {
            offset_bits: 0,
            len_bits: 0,
            bit_order: Default::default(),
        }
    }
}
