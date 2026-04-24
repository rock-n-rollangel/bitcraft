//! Unified `Value` type used across parse, transform, and serialize.
//!
//! This type replaces the previous pair of `assembly::Value` / `transform::Value`.
//! Parse emits `U64`, `I64`, or `Array`. Transforms can additionally produce
//! `F32`, `F64`, `Bytes`, or `String`. Serialize accepts only `U64`, `I64`, and
//! `Array` — passing `F32`/`F64`/`Bytes`/`String` returns [`crate::errors::WriteError::UnsupportedValue`].
//!
//! The serde representation is externally tagged: `{"U64": 42}`, `{"I64": -1}`,
//! `{"F32": 1.5}`, `{"F64": 3.14}`, `{"Bytes": [1, 2, 3]}`, `{"String": "x"}`,
//! `{"Array": [ ... ]}`. This shape is what the TypeScript wrapper produces.
//!
//! ## Example
//!
//! ```
//! use bitspec::value::Value;
//!
//! fn describe(v: &Value) -> &'static str {
//!     match v {
//!         Value::U64(_) => "unsigned int",
//!         Value::I64(_) => "signed int",
//!         Value::F32(_) | Value::F64(_) => "float",
//!         Value::Bytes(_) => "bytes",
//!         Value::String(_) => "string",
//!         Value::Array(_) => "array",
//!     }
//! }
//!
//! assert_eq!(describe(&Value::U64(42)), "unsigned int");
//! assert_eq!(describe(&Value::String("x".into())), "string");
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A value produced by parse or transform, and accepted by serialize.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Value {
    /// Unsigned 64-bit integer. Emitted by parse for unsigned fields.
    U64(u64),
    /// Signed 64-bit integer. Emitted by parse for signed fields.
    I64(i64),
    /// 32-bit floating-point value. Emitted by transforms with `Base::Float32`.
    F32(f32),
    /// 64-bit floating-point value. Emitted by transforms with `Base::Float64` or scale/offset.
    F64(f64),
    /// Raw bytes. Emitted by transforms with `Base::Bytes` and no encoding.
    Bytes(Vec<u8>),
    /// Decoded string. Emitted by transforms with text encoding or an enum map.
    String(String),
    /// Array of values. Emitted by parse for array fields and propagated by transforms.
    Array(Vec<Value>),
}
