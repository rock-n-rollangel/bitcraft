
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assemble {
    ConcatMsb,
    ConcatLsb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOrder {
    MsbFirst,
    LsbFirst,
}

impl Default for BitOrder {
    fn default() -> Self {
        BitOrder::MsbFirst
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    I64(i64),
    U64(u64),
    Array(Vec<Value>),
}

#[derive(Debug, Clone)]
pub enum ArrayCount {
    Fixed(usize),
}
