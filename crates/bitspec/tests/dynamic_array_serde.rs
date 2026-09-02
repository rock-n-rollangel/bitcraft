//! JSON schema definitions for dynamic (length-prefixed) arrays.

#![cfg(feature = "serde")]

use bitspec::schema::Schema;
use bitspec::serde::SchemaDef;
use bitspec::value::Value;

#[test]
fn json_fixed_count_still_parses_as_number() {
    let json = r#"{
        "fields": [
            { "name": "items", "kind": { "type": "Array", "count": 2, "stride_bits": 8, "offset_bits": 0 },
              "signed": false, "assemble": "ConcatMsb",
              "fragments": [{ "offset_bits": 0, "len_bits": 8 }] }
        ]
    }"#;
    let def: SchemaDef = serde_json::from_str(json).unwrap();
    let schema = Schema::try_from(def).unwrap();
    let parsed = schema.parse(&[0x01, 0x02]).unwrap();
    assert_eq!(
        parsed.get("items"),
        Some(&Value::Array(vec![Value::U64(1), Value::U64(2)]))
    );
}

#[test]
fn json_from_field_count_parses_dynamic_array() {
    let json = r#"{
        "fields": [
            { "name": "len", "kind": { "type": "Scalar" }, "signed": false, "assemble": "ConcatMsb",
              "fragments": [{ "offset_bits": 0, "len_bits": 8 }] },
            { "name": "items",
              "kind": { "type": "Array", "count": { "from_field": "len" }, "stride_bits": 8, "offset_bits": 8 },
              "signed": false, "assemble": "ConcatMsb",
              "fragments": [{ "offset_bits": 0, "len_bits": 8 }] }
        ]
    }"#;
    let def: SchemaDef = serde_json::from_str(json).unwrap();
    let schema = Schema::try_from(def).unwrap();
    let parsed = schema.parse(&[0x02, 0x0A, 0x0B]).unwrap();
    assert_eq!(parsed.get("len"), Some(&Value::U64(2)));
    assert_eq!(
        parsed.get("items"),
        Some(&Value::Array(vec![Value::U64(10), Value::U64(11)]))
    );
}
