#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    InvalidArrayStride,
    InvalidArrayCount,
    InvalidFieldSize,
    InvalidFragment,
    InvalidFieldKind,
    EmptyArrayElement,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    OutOfBounds,
    TooManyBitsRead,
    PacketTooShort,
}