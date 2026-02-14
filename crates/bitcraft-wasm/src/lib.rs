//! WASM bindings for the `bitcraft` binary schema engine.
//!
//! This crate exposes a compact API to JavaScript for parsing binary
//! payloads according to a JSON schema definition. Internally it uses
//! the `bitcraft` crate to describe how bits are laid out in a payload
//! and the `bitcraft-transform` crate to turn raw values into
//! human‑friendly data (scaling, offsets, text decoding, enums, etc.).
//!
//! At a high level you:
//! - **Describe your fields** in JSON using the shape in `schema_def`
//!   (field name, kind, bit fragments, signedness, etc.).
//! - **Optionally attach transforms** using the shape in `transform_def`
//!   (base type, scaling, encoding, enum map, …).
//! - **Compile** the schema once, and **parse** binary payloads many
//!   times from JavaScript.
//!
//! The entry point from JS is the [`WasmSchema`] type:
//!
//! ```text
//! // Pseudo TypeScript example
//! //
//! // const schemaJson = JSON.stringify({
//! //   fields: [
//! //     {
//! //       name: "id",
//! //       kind: { type: "Scalar" },
//! //       signed: false,
//! //       assemble: "ConcatMsb",
//! //       fragments: [{ offset_bits: 0, len_bits: 16 }],
//! //       transform: { base: "Int", scale: 0.5, offset: 100 }
//! //     }
//! //   ]
//! // });
//! //
//! // const wasmSchema = new WasmSchema(schemaJson);
//! // const result = wasmSchema.parse(someUint8Array);
//! // // result is a JS object: { id: 123.5 }
//! ```
//!
//! Error values are converted to `JsValue` with a `Debug` representation,
//! which makes it easy to inspect failures from JavaScript.

mod convert;

use std::collections::HashMap;

use bitcraft_transform::Value;
use bitcraft::serde::SchemaDef;
use wasm_bindgen::prelude::*;

/// Compiled schema that can be used from JavaScript to parse binary data.
///
/// A `WasmSchema` owns a compiled [`bitcraft::schema::Schema`] plus any
/// per‑field transforms that should be applied to the raw values.
///
/// Typical usage from JavaScript/TypeScript is:
///
/// ```text
/// // const schema = new WasmSchema(schemaJson);
/// // const parsed = schema.parse(bytes);
/// // console.log(parsed.someField);
/// ```
#[wasm_bindgen]
pub struct WasmSchema {
    /// Compiled bit‑level schema describing how to read the payload.
    schema: bitcraft::schema::Schema,
    /// Optional value transforms keyed by field name.
    transforms: HashMap<String, bitcraft_transform::Transform>,
}

#[wasm_bindgen]
impl WasmSchema {
    /// Creates a new compiled schema from a JSON definition.
    ///
    /// The `schema_json` string must deserialize into [`SchemaDef`], which
    /// in turn describes:
    ///
    /// - **Fields**: their name, kind (scalar or fixed‑size array),
    ///   signedness and assemble strategy.
    /// - **Fragments**: the bit ranges that make up each field.
    /// - **Transforms** (optional): how to post‑process raw values using
    ///   `bitcraft-transform` (base type, scale/offset, encodings, enums).
    ///
    /// On success this compiles the schema and prepares any transforms so
    /// that it can be reused to parse many payloads efficiently.
    #[wasm_bindgen(constructor)]
    pub fn new(schema_json: &str) -> Result<WasmSchema, JsValue> {
        let def: SchemaDef = serde_json::from_str(schema_json).map_err(convert::error_to_js)?;

        let transforms = convert::schema_def_to_transforms(&def).map_err(convert::error_to_js)?;

        let write_config =
            convert::write_config_def_to_write_config(&def).map_err(convert::error_to_js)?;

        let fields = convert::schema_def_to_fields(&def).map_err(convert::error_to_js)?;

        let schema = bitcraft::schema::Schema::compile(&fields, write_config)
            .map_err(convert::error_to_js)?;

        Ok(WasmSchema { schema, transforms })
    }

    /// Parses a binary payload according to this compiled schema.
    ///
    /// - `data` is the raw byte slice (for example a `Uint8Array` passed from JS).
    /// - The return value is a JavaScript object (`JsValue`) where keys are
    ///   field names and values have been converted through any configured
    ///   transforms (see [`schema_def_to_transforms`](crate::convert::schema_def_to_transforms)).
    ///
    /// On error a `JsValue` containing a debug string is returned.
    pub fn parse(&self, data: &[u8]) -> Result<JsValue, JsValue> {
        let raw_map = self.schema.parse(data).map_err(convert::error_to_js)?;

        let mut out = std::collections::BTreeMap::<String, Value>::new();
        for (name, value) in raw_map {
            let transformed_value = self.apply_transform(name.as_str(), value)?;
            out.insert(name, transformed_value);
        }

        convert::map_to_js(out)
    }

    /// Applies a per‑field transform if configured, otherwise passes through the value.
    ///
    /// This is an internal helper and is not exported to JavaScript.
    fn apply_transform(
        &self,
        name: &str,
        value: bitcraft::assembly::Value,
    ) -> Result<Value, JsValue> {
        match self.transforms.get(name) {
            Some(transform) => transform.apply(value).map_err(convert::error_to_js),
            None => Ok(convert::value_to_transform_value(value)),
        }
    }

    pub fn serialize(&self, obj: JsValue) -> Result<Vec<u8>, JsValue> {
      // Convert JS object into a generic Rust structure
      let raw: HashMap<String, serde_json::Value> =
          serde_wasm_bindgen::from_value(obj)
              .map_err(|e| JsValue::from_str(&e.to_string()))?;

      // Convert serde_json::Value -> bitcraft::Value
      let mut map = HashMap::new();

      for (k, v) in raw {
          map.insert(k, convert::convert_json_value(v)?);
      }

      self.schema
          .serialize(&map)
          .map_err(convert::error_to_js)
  }
}

#[test]
fn test_schema_def_to_fields() {
    let json = r#"
    {
      "fields": [
        {
          "name": "id",
          "kind": { "type": "Scalar" },
          "signed": false,
          "assemble": "ConcatMsb",
          "fragments": [{ "offset_bits": 0, "len_bits": 16 }]
        }
      ],
      "write_config": { "bit_order": "MsbFirst" }
    }
    "#;

    let def: SchemaDef = serde_json::from_str(json).unwrap();
    let fields = convert::schema_def_to_fields(&def).unwrap();
    let write_config = convert::write_config_def_to_write_config(&def).unwrap();

    assert_eq!(
        write_config.unwrap().bit_order,
        bitcraft::assembly::BitOrder::MsbFirst
    );
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "id");
}

#[test]
fn test_schema_def_to_fields_array() {
    let json = r#"
    {
      "fields": [
        {
          "name": "id",
          "kind": { "type": "Scalar" },
          "signed": false,
          "assemble": "ConcatMsb",
          "fragments": [{ "offset_bits": 0, "len_bits": 16 }],
          "transform": { "base": "Int", "scale": 0.5, "offset": 100 }
        },
        {
          "name": "values",
          "kind": { "type": "Array", "count": 10, "stride_bits": 8, "offset_bits": 16 },
          "signed": false,
          "assemble": "ConcatMsb",
          "fragments": [{ "offset_bits": 0, "len_bits": 8 }]
        }
      ]
    }
    "#;

    let def: SchemaDef = serde_json::from_str(json).unwrap();
    let transforms = convert::schema_def_to_transforms(&def).unwrap();
    let fields = convert::schema_def_to_fields(&def).unwrap();
    let write_config = convert::write_config_def_to_write_config(&def).unwrap();

    assert_eq!(transforms.len(), 1);
    assert_eq!(
        transforms.get("id").unwrap().base,
        bitcraft_transform::Base::Int
    );
    assert_eq!(transforms.get("id").unwrap().scale, Some(0.5));
    assert_eq!(transforms.get("id").unwrap().offset, Some(100.0));

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "id");
    assert_eq!(fields[1].name, "values");

    assert!(write_config.is_none());
}
