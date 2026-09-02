//! Integration tests for arrays whose length is read from the packet
//! (`ArrayCount::FromField`).

use std::collections::BTreeMap;

use bitspec::assembly::{ArrayCount, Assemble, BitOrder};
use bitspec::errors::CompileError;
use bitspec::field::{ArraySpec, Field, FieldKind};
use bitspec::fragment::Fragment;
use bitspec::schema::Schema;
use bitspec::value::Value;

fn scalar(name: &str, offset_bits: usize, len_bits: usize) -> Field {
    Field {
        name: name.to_string(),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(offset_bits, len_bits)],
        transform: None,
    }
}

fn dynamic_array(name: &str, count_field: &str, offset_bits: usize) -> Field {
    Field {
        name: name.to_string(),
        kind: FieldKind::Array(ArraySpec {
            count: ArrayCount::FromField(count_field.to_string()),
            stride_bits: 8,
            offset_bits,
        }),
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(0, 8)],
        transform: None,
    }
}

#[test]
fn compile_rejects_unknown_count_field() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "nope", 8)];
    let err = Schema::compile(&fields, None).unwrap_err();
    assert_eq!(err, CompileError::UnknownCountField("nope".to_string()));
}

#[test]
fn compile_rejects_signed_count_field() {
    let mut len = scalar("len", 0, 8);
    len.signed = true;
    let fields = vec![len, dynamic_array("items", "len", 8)];
    let err = Schema::compile(&fields, None).unwrap_err();
    assert_eq!(err, CompileError::InvalidCountField("len".to_string()));
}

#[test]
fn compile_rejects_non_scalar_count_field() {
    let counts = Field {
        kind: FieldKind::Array(ArraySpec {
            count: ArrayCount::Fixed(1),
            stride_bits: 8,
            offset_bits: 0,
        }),
        ..scalar("len", 0, 8)
    };
    let fields = vec![counts, dynamic_array("items", "len", 8)];
    let err = Schema::compile(&fields, None).unwrap_err();
    assert_eq!(err, CompileError::InvalidCountField("len".to_string()));
}

#[test]
fn compile_rejects_field_after_dynamic_array() {
    let fields = vec![
        scalar("len", 0, 8),
        dynamic_array("items", "len", 8),
        scalar("crc", 16, 8),
    ];
    let err = Schema::compile(&fields, None).unwrap_err();
    assert_eq!(err, CompileError::DynamicArrayNotAtTail("items".to_string()));
}

#[test]
fn compile_rejects_fixed_array_overlapping_dynamic_array() {
    let fixed = Field {
        kind: FieldKind::Array(ArraySpec {
            count: ArrayCount::Fixed(2),
            stride_bits: 8,
            offset_bits: 8,
        }),
        ..scalar("pair", 0, 8)
    };
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 16), fixed];
    let err = Schema::compile(&fields, None).unwrap_err();
    assert_eq!(err, CompileError::DynamicArrayNotAtTail("items".to_string()));
}

#[test]
fn compile_rejects_multiple_dynamic_arrays() {
    let fields = vec![
        scalar("len", 0, 8),
        dynamic_array("a", "len", 8),
        dynamic_array("b", "len", 8),
    ];
    let err = Schema::compile(&fields, None).unwrap_err();
    assert_eq!(err, CompileError::MultipleDynamicArrays);
}

#[test]
fn parse_reads_count_from_packet() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let parsed = schema.parse(&[0x03, 0x0A, 0x0B, 0x0C]).unwrap();
    assert_eq!(parsed.get("len"), Some(&Value::U64(3)));
    assert_eq!(
        parsed.get("items"),
        Some(&Value::Array(vec![
            Value::U64(10),
            Value::U64(11),
            Value::U64(12)
        ]))
    );
}

#[test]
fn parse_zero_count_yields_empty_array() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let parsed = schema.parse(&[0x00]).unwrap();
    assert_eq!(parsed.get("items"), Some(&Value::Array(vec![])));
}

#[test]
fn parse_fails_when_packet_shorter_than_count_claims() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let err = schema.parse(&[0x03, 0x0A, 0x0B]).unwrap_err();
    assert_eq!(err, bitspec::errors::ReadError::PacketTooShort);
}

#[test]
fn parse_fails_on_absurd_count_without_allocating() {
    let fields = vec![scalar("len", 0, 64), dynamic_array("items", "len", 64)];
    let schema = Schema::compile(&fields, None).unwrap();

    let mut data = vec![0xFF; 8];
    data.push(0x01);
    let err = schema.parse(&data).unwrap_err();
    assert_eq!(err, bitspec::errors::ReadError::PacketTooShort);
}

#[test]
fn serialize_auto_writes_count_from_array_length() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let obj = BTreeMap::from([(
        "items".to_string(),
        Value::Array(vec![Value::U64(10), Value::U64(11), Value::U64(12)]),
    )]);
    let bytes = schema.serialize(&obj).unwrap();
    assert_eq!(bytes, vec![0x03, 0x0A, 0x0B, 0x0C]);
}

#[test]
fn serialize_accepts_matching_explicit_count() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let obj = BTreeMap::from([
        ("len".to_string(), Value::U64(2)),
        (
            "items".to_string(),
            Value::Array(vec![Value::U64(1), Value::U64(2)]),
        ),
    ]);
    let bytes = schema.serialize(&obj).unwrap();
    assert_eq!(bytes, vec![0x02, 0x01, 0x02]);
}

#[test]
fn serialize_rejects_mismatched_explicit_count() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let obj = BTreeMap::from([
        ("len".to_string(), Value::U64(5)),
        (
            "items".to_string(),
            Value::Array(vec![Value::U64(1), Value::U64(2)]),
        ),
    ]);
    let err = schema.serialize(&obj).unwrap_err();
    assert_eq!(
        err,
        bitspec::errors::WriteError::CountMismatch {
            field: "len".to_string(),
            expected: 2,
            actual: 5,
        }
    );
}

#[test]
fn serialize_rejects_count_wider_than_count_field() {
    let fields = vec![scalar("len", 0, 2), dynamic_array("items", "len", 2)];
    let schema = Schema::compile(&fields, None).unwrap();

    let items: Vec<Value> = (0..4).map(Value::U64).collect();
    let obj = BTreeMap::from([("items".to_string(), Value::Array(items))]);
    let err = schema.serialize(&obj).unwrap_err();
    assert_eq!(
        err,
        bitspec::errors::WriteError::CountOverflow {
            field: "len".to_string(),
            count: 4,
        }
    );
}

#[test]
fn dynamic_array_round_trips() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let data = vec![0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let parsed = schema.parse(&data).unwrap();
    let bytes = schema.serialize(&parsed).unwrap();
    assert_eq!(bytes, data);
}

#[test]
fn serialize_zero_length_dynamic_array() {
    let fields = vec![scalar("len", 0, 8), dynamic_array("items", "len", 8)];
    let schema = Schema::compile(&fields, None).unwrap();

    let obj = BTreeMap::from([("items".to_string(), Value::Array(vec![]))]);
    let bytes = schema.serialize(&obj).unwrap();
    assert_eq!(bytes, vec![0x00]);
}
