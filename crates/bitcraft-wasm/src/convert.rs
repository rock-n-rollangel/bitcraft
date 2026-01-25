use std::collections::BTreeMap;

use bitcraft::{
    assembly::{Assemble, BitOrder},
    field::{ArraySpec, Field, FieldKind},
    fragment::Fragment,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

#[derive(Debug, Deserialize)]
pub enum AssembleDef {
    ConcatMsb,
    ConcatLsb,
}

#[derive(Debug, Deserialize)]
pub enum BitOrderDef {
    MsbFirst,
    LsbFirst,
}

#[derive(Debug, Deserialize)]
pub struct SchemaDef {
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub kind: FieldKindDef,
    pub signed: bool,
    pub assemble: AssembleDef,
    pub fragments: Vec<FragmentDef>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum FieldKindDef {
    Scalar,
    Array {
        count: usize,
        stride_bits: usize,
        offset_bits: usize,
    },
}

#[derive(Debug, Deserialize)]
pub struct FragmentDef {
    pub offset_bits: usize,
    pub len_bits: usize,
    #[serde(default)]
    pub bit_order: Option<BitOrderDef>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum JsValueOut {
    U64(u64),
    I64(i64),
    Array(Vec<JsValueOut>),
}

// If bitcraft::errors::SchemaError is public, use it.
// Otherwise: change Error to String and use `Err("...".into())`.
type Error = bitcraft::errors::CompileError;

pub fn schema_def_to_fields(def: SchemaDef) -> Result<Vec<Field>, Error> {
    let mut out = Vec::with_capacity(def.fields.len());

    for f in def.fields {
        out.push(field_def_to_field(f)?);
    }

    Ok(out)
}

fn field_def_to_field(def: FieldDef) -> Result<Field, Error> {
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

    let assemble = assemble_def_to_core(def.assemble);
    let fragments = def
        .fragments
        .into_iter()
        .map(fragment_def_to_fragment)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Field {
        name: def.name,
        kind,
        signed: def.signed,
        assemble,
        fragments,
    })
}

fn fragment_def_to_fragment(def: FragmentDef) -> Result<Fragment, Error> {
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

fn assemble_def_to_core(def: AssembleDef) -> Assemble {
    match def {
        AssembleDef::ConcatMsb => Assemble::ConcatMsb,
        AssembleDef::ConcatLsb => Assemble::ConcatLsb,
    }
}

fn value_to_js(v: bitcraft::assembly::Value) -> JsValueOut {
    match v {
        bitcraft::assembly::Value::U64(x) => JsValueOut::U64(x),
        bitcraft::assembly::Value::I64(x) => JsValueOut::I64(x),
        bitcraft::assembly::Value::Array(xs) => {
            JsValueOut::Array(xs.into_iter().map(value_to_js).collect())
        }
    }
}

pub fn map_to_js(
    map: BTreeMap<String, bitcraft::assembly::Value>,
) -> Result<JsValue, JsValue> {
    let out: BTreeMap<String, JsValueOut> = map
        .into_iter()
        .map(|(k, v)| (k, value_to_js(v)))
        .collect();

    serde_wasm_bindgen::to_value(&out)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
