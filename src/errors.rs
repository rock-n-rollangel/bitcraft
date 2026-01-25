#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    InvalidArrayStride,
    InvalidArrayCount,
    InvalidFieldSize,
    InvalidFragment,
    InvalidFieldKind,
    EmptyArrayElement,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    OutOfBounds,
    TooManyBitsRead,
    PacketTooShort,
}