//! Helpers for converting JSON schema definitions into core `bitcraft` types
//! and JavaScript‑friendly values.
//!
//! This module is internal; its functions back the public
//! [`WasmSchema`](crate::WasmSchema) API by:
//!
//! - Converting [`SchemaDef`](crate::schema_def::SchemaDef) into
//!   `bitcraft::field::Field` values.
//! - Building `bitcraft_transform::Transform` values from
//!   [`TransformDef`](crate::transform_def::TransformDef).
//! - Converting parsed values into `JsValue` so they can be consumed
//!   ergonomically from JavaScript/TypeScript.
use std::collections::{BTreeMap, HashMap};

use bitcraft::{
    assembly::{Assemble, BitOrder},
    field::{ArraySpec, Field, FieldKind},
    fragment::Fragment,
};
use bitcraft_transform::{Base, Encoding, Transform};
use serde::Serialize;
use wasm_bindgen::JsValue;
use bitcraft::serde::*;

/// Serializable representation of a parsed value that can be converted to `JsValue`.
///
/// This mirrors [`bitcraft_transform::Value`] but uses concrete Rust types that
/// can be serialized via `serde` and then passed through `serde_wasm_bindgen`
/// into JavaScript.
#[derive(Serialize)]
#[serde(untagged)]
pub enum JsValueOut {
    Int(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<JsValueOut>),
}

/// Convenience alias for the error type used while compiling schemas.
type Error = bitcraft::errors::CompileError;

/// Converts a high‑level [`SchemaDef`] into a list of core `bitcraft` fields.
///
/// This performs basic validation (for example non‑empty names, valid fragment
/// sizes, and sensible array specs) and returns a `CompileError` if anything
/// is inconsistent.
pub fn schema_def_to_fields(def: &SchemaDef) -> Result<Vec<Field>, Error> {
    let mut out = Vec::with_capacity(def.fields.len());

    for f in &def.fields {
        out.push(field_def_to_field(f)?);
    }

    Ok(out)
}

/// Extracts per‑field transforms from a [`SchemaDef`].
///
/// Only fields that specify a `transform` are present in the returned map.
/// The keys are the field names; values are fully‑constructed
/// [`bitcraft_transform::Transform`] instances.
pub fn schema_def_to_transforms(
    def: &SchemaDef,
) -> Result<HashMap<String, bitcraft_transform::Transform>, Error> {
    let mut out = HashMap::new();

    for f in &def.fields {
        if let Some(transform_def) = &f.transform {
            let transform = transform_def_to_transform(&transform_def)?;
            out.insert(f.name.clone(), transform);
        }
    }

    Ok(out)
}

/// Converts a single [`FieldDef`] into a fully‑validated `Field`.
///
/// Validation performed here is intentionally strict so that problems are
/// caught at compile time rather than when parsing a payload.
fn field_def_to_field(def: &FieldDef) -> Result<Field, Error> {
    if def.name.trim().is_empty() {
        return Err(Error::InvalidFieldName);
    }
    if def.fragments.is_empty() {
        return Err(Error::InvalidFieldSize);
    }

    let kind = match def.kind {
        FieldKindDef::Scalar => FieldKind::Scalar,
        FieldKindDef::Array {
            count,
            stride_bits,
            offset_bits,
        } => {
            if count == 0 {
                return Err(Error::InvalidArrayCount);
            }
            if stride_bits == 0 {
                return Err(Error::InvalidArrayStride);
            }

            FieldKind::Array(ArraySpec {
                count,
                stride_bits,
                offset_bits,
            })
        }
    };

    let assemble = assemble_def_to_core(&def.assemble);
    let fragments = def
        .fragments
        .iter()
        .map(fragment_def_to_fragment)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Field {
        name: def.name.clone(),
        kind,
        signed: def.signed,
        assemble,
        fragments: fragments.clone(),
    })
}

/// Validates and converts a JSON‑level fragment description into a core `Fragment`.
///
/// Enforces a positive `len_bits` and translates the optional
/// [`BitOrderDef`](crate::schema_def::BitOrderDef) into a concrete `BitOrder`,
/// defaulting to most‑significant‑bit first when not provided.
fn fragment_def_to_fragment(def: &FragmentDef) -> Result<Fragment, Error> {
    if def.len_bits == 0 {
        return Err(Error::InvalidFragment);
    }

    let bit_order = match def.bit_order {
        Some(BitOrderDef::MsbFirst) => BitOrder::MsbFirst,
        Some(BitOrderDef::LsbFirst) => BitOrder::LsbFirst,
        None => BitOrder::MsbFirst, // good default for most protocols
    };

    Ok(Fragment {
        offset_bits: def.offset_bits,
        len_bits: def.len_bits,
        bit_order,
    })
}

/// Maps the JSON‑level assemble strategy onto the core [`Assemble`] enum.
fn assemble_def_to_core(def: &AssembleDef) -> Assemble {
    match def {
        AssembleDef::ConcatMsb => Assemble::Concat(BitOrder::MsbFirst),
        AssembleDef::ConcatLsb => Assemble::Concat(BitOrder::LsbFirst),
    }
}

/// Converts a `bitcraft_transform::Value` into the serializable [`JsValueOut`] shape.
fn value_to_js(v: bitcraft_transform::Value) -> JsValueOut {
    match v {
        bitcraft_transform::Value::Int(x) => JsValueOut::Int(x),
        bitcraft_transform::Value::Float32(x) => JsValueOut::Float32(x),
        bitcraft_transform::Value::Float64(x) => JsValueOut::Float64(x),
        bitcraft_transform::Value::String(x) => JsValueOut::String(x),
        bitcraft_transform::Value::Bytes(x) => JsValueOut::Bytes(x),
        bitcraft_transform::Value::Array(xs) => {
            JsValueOut::Array(xs.into_iter().map(value_to_js).collect())
        }
    }
}

/// Converts a low‑level `bitcraft::assembly::Value` into a `bitcraft_transform::Value`.
///
/// This is used when no explicit transform is configured for a field but the
/// value still needs to be presented through the `bitcraft-transform` layer.
pub fn value_to_transform_value(v: bitcraft::assembly::Value) -> bitcraft_transform::Value {
    match v {
        bitcraft::assembly::Value::U64(x) => bitcraft_transform::Value::Int(x as i64),
        bitcraft::assembly::Value::I64(x) => bitcraft_transform::Value::Int(x),
        bitcraft::assembly::Value::Array(xs) => {
            bitcraft_transform::Value::Array(xs.into_iter().map(value_to_transform_value).collect())
        }
    }
}

/// Converts a map of parsed values into a JavaScript object.
///
/// Keys are field names and values are first converted into [`JsValueOut`]
/// and then into `JsValue` via `serde_wasm_bindgen`.
pub fn map_to_js(map: BTreeMap<String, bitcraft_transform::Value>) -> Result<JsValue, JsValue> {
    let out: BTreeMap<String, JsValueOut> =
        map.into_iter().map(|(k, v)| (k, value_to_js(v))).collect();

    serde_wasm_bindgen::to_value(&out).map_err(error_to_js)
}

/// Builds a concrete `bitcraft_transform::Transform` from a JSON‑level definition.
fn transform_def_to_transform(def: &TransformDef) -> Result<bitcraft_transform::Transform, Error> {
    Ok(Transform {
        base: match def.base {
            BaseDef::Int => Base::Int,
            BaseDef::Float32 => Base::Float32,
            BaseDef::Float64 => Base::Float64,
            BaseDef::Bytes => Base::Bytes,
        },
        scale: def.scale,
        offset: def.offset,
        encoding: match def.encoding {
            Some(EncodingDef::Utf8) => Some(Encoding::Utf8),
            Some(EncodingDef::Ascii) => Some(Encoding::Ascii),
            None => None,
        },
        zero_terminated: def.zero_terminated,
        trim: def.trim,
        enum_map: def.enum_map.clone(),
    })
}

/// Converts any debug‑printable error into a `JsValue` with a human‑readable message.
///
/// This keeps the surface area of error handling small on the JavaScript side
/// while still retaining detailed information that can be logged or surfaced
/// in developer tools.
pub fn error_to_js<T>(e: T) -> JsValue
where
    T: std::fmt::Debug,
{
    JsValue::from_str(&format!("{e:?}"))
}

pub fn write_config_def_to_write_config(
    def: &SchemaDef,
) -> Result<Option<bitcraft::schema::WriteConfig>, Error> {
    Ok(match &def.write_config {
        Some(write_config) => Some(bitcraft::schema::WriteConfig {
            bit_order: match write_config.bit_order {
                BitOrderDef::MsbFirst => BitOrder::MsbFirst,
                BitOrderDef::LsbFirst => BitOrder::LsbFirst,
            },
        }),
        None => None,
    })
}

pub fn convert_json_value(v: serde_json::Value) -> Result<bitcraft::assembly::Value, JsValue> {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(bitcraft::assembly::Value::I64(i));
            }

            if let Some(u) = n.as_u64() {
                return Ok(bitcraft::assembly::Value::U64(u));
            }

            if let Some(f) = n.as_f64() {
                return Ok(bitcraft::assembly::Value::U64(f.to_bits())); // float as raw bits
            }

            Err(JsValue::from_str("Invalid number"))
        }

        serde_json::Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(convert_json_value(item)?);
            }
            Ok(bitcraft::assembly::Value::Array(out))
        }

        _ => Err(JsValue::from_str("Unsupported value type")),
    }
}
