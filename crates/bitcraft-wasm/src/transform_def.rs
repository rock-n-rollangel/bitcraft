//! JSON‑deserializable description of value transforms used by `bitcraft-wasm`.
//!
//! Transforms are applied *after* the raw bits of a field have been assembled
//! into a primitive value. They are useful for:
//!
//! - Scaling and offsetting numeric values (engineering units).
//! - Decoding bytes into strings.
//! - Trimming and zero‑termination for C‑style strings.
//! - Mapping integer codes to human‑readable enum labels.

use serde::Deserialize;
use std::collections::HashMap;

/// Base type of the value before any transform is applied.
#[derive(Debug, Deserialize)]
pub enum BaseDef {
    /// Signed/unsigned integer value.
    Int,
    /// 32‑bit floating‑point value.
    Float32,
    /// 64‑bit floating‑point value.
    Float64,
    /// Raw bytes (often used together with [`EncodingDef`]).
    Bytes,
}

/// Text encoding to use when interpreting byte values as strings.
#[derive(Debug, Deserialize)]
pub enum EncodingDef {
    /// UTF‑8 encoded string.
    Utf8,
    /// ASCII encoded string.
    Ascii,
}

/// Complete description of how to transform a parsed raw value.
#[derive(Debug, Deserialize)]
pub struct TransformDef {
    /// Base representation of the raw value.
    pub base: BaseDef,
    /// Optional multiplicative scale applied to numeric values.
    pub scale: Option<f64>,
    /// Optional additive offset applied after scaling.
    pub offset: Option<f64>,

    /// Optional text encoding when interpreting bytes as strings.
    pub encoding: Option<EncodingDef>,
    /// Whether string values should stop at the first zero byte.
    pub zero_terminated: Option<bool>,
    /// Whether leading/trailing whitespace should be trimmed.
    pub trim: Option<bool>,

    /// Optional mapping from integer codes to human‑readable labels.
    pub enum_map: Option<HashMap<i64, String>>,
}
