//! JSON‑deserializable schema description used by `bitcraft-wasm`.
//!
//! These types describe the *shape* of the binary data to be parsed. They are
//! intended to be constructed from JSON (for example a schema file shipped
//! with your application) and then compiled into core `bitcraft` types.
//!
//! The same shapes are expected on the JavaScript side when you call
//! [`WasmSchema::new`](crate::WasmSchema::new) with a JSON string.

use serde::Deserialize;

/// How individual fragments of bits are assembled into a numeric value.
#[derive(Debug, Deserialize)]
pub enum AssembleDef {
    /// Concatenate fragments most‑significant‑bit first.
    ConcatMsb,
    /// Concatenate fragments least‑significant‑bit first.
    ConcatLsb,
}

/// Bit order to use when reading a fragment.
#[derive(Debug, Deserialize, Default)]
pub enum BitOrderDef {
    #[default]
    /// Most‑significant bit first within the fragment.
    MsbFirst,
    /// Least‑significant bit first within the fragment.
    LsbFirst,
}

#[derive(Debug, Deserialize)]
pub struct WriteConfigDef {
    #[serde(default)]
    pub bit_order: BitOrderDef,
}

/// Top‑level schema definition consisting of a list of fields.
#[derive(Debug, Deserialize)]
pub struct SchemaDef {
    /// All fields that should be parsed from the payload.
    pub fields: Vec<FieldDef>,
    #[serde(default)]
    pub write_config: Option<WriteConfigDef>,
}

/// Description of a single parsed field.
#[derive(Debug, Deserialize)]
pub struct FieldDef {
    /// Human‑readable field name; becomes the key in the output map.
    pub name: String,
    /// Whether this is a scalar or fixed‑size array field.
    pub kind: FieldKindDef,
    /// Whether the assembled value should be interpreted as signed.
    pub signed: bool,
    /// Strategy used to assemble fragments into a single value.
    pub assemble: AssembleDef,
    /// Bit fragments that make up this field.
    pub fragments: Vec<FragmentDef>,

    /// Optional post‑processing transform applied after parsing the raw value.
    #[serde(default)]
    pub transform: Option<crate::transform_def::TransformDef>,
}

/// Kind of field in the schema.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum FieldKindDef {
    /// Single scalar value.
    Scalar,
    /// Fixed‑size array of values laid out with a constant stride.
    Array {
        /// Number of elements in the array.
        count: usize,
        /// Distance in bits between consecutive elements.
        stride_bits: usize,
        /// Bit offset of the first element from the start of the payload.
        offset_bits: usize,
    },
}

/// Bit‑level fragment that contributes to a field value.
#[derive(Debug, Deserialize)]
pub struct FragmentDef {
    /// Offset of the first bit of this fragment from the start of the payload.
    pub offset_bits: usize,
    /// Length of the fragment in bits.
    pub len_bits: usize,
    /// Optional bit order inside the fragment; defaults to MSB‑first.
    #[serde(default)]
    pub bit_order: Option<BitOrderDef>,
}
