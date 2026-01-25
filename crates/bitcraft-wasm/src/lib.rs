mod convert;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmSchema {
    inner: bitcraft::schema::Schema,
}

#[wasm_bindgen]
impl WasmSchema {
    #[wasm_bindgen(constructor)]
    pub fn new(schema_json: &str) -> Result<WasmSchema, JsValue> {
        let def: convert::SchemaDef =
            serde_json::from_str(schema_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        let fields =
            convert::schema_def_to_fields(def).map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        let inner = bitcraft::schema::Schema::compile(&fields)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        Ok(WasmSchema { inner })
    }

    pub fn parse(&self, data: &[u8]) -> Result<JsValue, JsValue> {
        let result = self.inner
            .parse(data)
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        convert::map_to_js(result)
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
      ]
    }
    "#;

    let def: convert::SchemaDef = serde_json::from_str(json).unwrap();
    let fields = convert::schema_def_to_fields(def).unwrap();

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
          "fragments": [{ "offset_bits": 0, "len_bits": 16 }]
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

    let def: convert::SchemaDef = serde_json::from_str(json).unwrap();
    let fields = convert::schema_def_to_fields(def).unwrap();

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "id");
    assert_eq!(fields[1].name, "values");
}
