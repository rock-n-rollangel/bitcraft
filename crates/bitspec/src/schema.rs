//! Schema: compiled set of fields used to parse byte slices into named values.

use std::collections::BTreeMap;

use crate::{
    assembly::{ArrayCount, BitOrder},
    compiled::{CompiledField, CompiledFieldKind},
    errors::{CompileError, ReadError, WriteError},
    field::Field,
    value::Value,
};

#[derive(Debug, Clone)]
pub struct WriteConfig {
    pub bit_order: BitOrder,
}

#[cfg(feature = "serde")]
impl From<crate::serde::WriteConfigDef> for WriteConfig {
    fn from(value: crate::serde::WriteConfigDef) -> Self {
        WriteConfig {
            bit_order: value.bit_order.into(),
        }
    }
}

impl Default for WriteConfig {
    fn default() -> Self {
        WriteConfig {
            bit_order: BitOrder::MsbFirst,
        }
    }
}

/// A compiled schema: list of [`CompiledField`]s and total bit length.
/// Use [`Schema::compile`] to build from [`Field`]s, then [`Schema::parse`] to parse bytes.
#[derive(Debug, Clone)]
pub struct Schema {
    total_bits: usize,
    /// Compiled fields in definition order.
    pub fields: Vec<CompiledField>,
    /// Optional write configuration (bit order for serialize).
    pub write_config: Option<WriteConfig>,
    #[cfg_attr(not(feature = "transform"), allow(dead_code))]
    transforms: std::collections::HashMap<String, crate::transform::Transform>,
}

#[cfg(feature = "serde")]
impl TryFrom<crate::serde::SchemaDef> for Schema {
    type Error = CompileError;

    fn try_from(value: crate::serde::SchemaDef) -> Result<Self, Self::Error> {
        let fields: Vec<Field> = value.fields.into_iter().map(Into::into).collect();
        let write_config = value.write_config.map(Into::into);
        return Self::compile(&fields, write_config);
    }
}

impl Schema {
    /// Compiles a slice of [`Field`]s into a schema. Fails if any field is invalid.
    pub fn compile(
        fields: &[Field],
        write_config: Option<WriteConfig>,
    ) -> Result<Self, CompileError> {
        let mut compiled_fields: Vec<CompiledField> = Vec::with_capacity(fields.len());
        let mut total_bits = 0;
        let mut transforms = std::collections::HashMap::new();

        for field in fields {
            let compiled_field: CompiledField = field.try_into()?;

            match &compiled_field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    for frag in &scalar.fragments {
                        let end = frag.offset_bits + frag.len_bits;
                        total_bits = total_bits.max(end);
                    }
                }
                CompiledFieldKind::Array(array) => {
                    let ArrayCount::Fixed(count) = array.count;
                    let end = array.offset_bits
                        + array.element.total_bits
                        + array.stride_bits * (count - 1);
                    total_bits = total_bits.max(end);
                }
            }

            if let Some(transform) = &field.transform {
                transforms.insert(field.name.clone(), transform.clone());
            }

            compiled_fields.push(compiled_field);
        }

        Ok(Self {
            fields: compiled_fields,
            total_bits,
            write_config,
            transforms,
        })
    }

    #[cfg(feature = "transform")]
    pub fn apply_transforms(
        &self,
        obj: std::collections::BTreeMap<String, crate::value::Value>,
    ) -> Result<std::collections::BTreeMap<String, crate::value::Value>, crate::transform::TransformError> {
        let mut map = std::collections::BTreeMap::new();
        for (name, value) in obj {
            let transformed = match self.transforms.get(&name) {
                Some(transform) => transform.apply(value)?,
                None => value,
            };
            map.insert(name, transformed);
        }
        Ok(map)
    }

    /// Parses `data` according to this schema. Returns a map of field names to [Value]s. Fails if `data` is too short.
    pub fn parse(&self, data: &[u8]) -> Result<BTreeMap<String, Value>, ReadError> {
        if data.len() * 8 < self.total_bits {
            return Err(ReadError::PacketTooShort);
        }

        let mut map: BTreeMap<String, Value> = BTreeMap::new();

        for field in &self.fields {
            match &field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    map.insert(field.name.clone(), scalar.assemble(data)?);
                }
                CompiledFieldKind::Array(array) => {
                    map.insert(field.name.clone(), array.assemble(data)?);
                }
            }
        }

        Ok(map)
    }

    pub fn serialize(
        &self,
        obj: &std::collections::BTreeMap<String, crate::value::Value>,
    ) -> Result<Vec<u8>, WriteError> {
        let total_bytes = (self.total_bits + 7) / 8;
        let mut buf = vec![0u8; total_bytes];

        for field in &self.fields {
            let value = obj
                .get(&field.name)
                .ok_or_else(|| WriteError::MissingField(field.name.clone()))?;

            match &field.kind {
                CompiledFieldKind::Scalar(scalar) => {
                    scalar
                        .disassemble_at(value, &mut buf, 0)
                        .map_err(|e| attach_field_name(e, &field.name))?;
                }
                CompiledFieldKind::Array(array) => {
                    array
                        .disassemble_at(value, &mut buf)
                        .map_err(|e| attach_field_name(e, &field.name))?;
                }
            }
        }

        Ok(buf)
    }
}

fn attach_field_name(err: WriteError, field: &str) -> WriteError {
    match err {
        WriteError::UnsupportedValue { variant, .. } => WriteError::UnsupportedValue {
            field: field.to_string(),
            variant,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        assembly::{Assemble, BitOrder},
        field::{ArraySpec, Field, FieldKind},
        fragment::Fragment,
    };

    use super::*;

    #[test]
    fn test_get_all_empty() {
        let schema = Schema::compile(&vec![], None).unwrap();
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = schema.parse(&data);
        assert_eq!(result, Ok(BTreeMap::new()));
    }

    #[test]
    fn test_get_all_one_field() {
        let field = Field {
            name: "test".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 1)],
            transform: None,
        };
        let schema = Schema::compile(&vec![field], None).unwrap();
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([("test".to_string(), Value::U64(0))]))
        );
    }

    #[test]
    fn test_get_multiple_fields() {
        let field1 = Field {
            name: "test1".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };
        let field2 = Field {
            name: "test2".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(8, 16)],
            transform: None,
        };
        let schema = Schema::compile(&vec![field1, field2], None).unwrap();
        let data = vec![0x01, 0x00, 0x01, 0x04];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([
                ("test1".to_string(), Value::U64(1)),
                ("test2".to_string(), Value::U64(1))
            ]))
        );
    }

    #[test]
    fn test_get_all_array() {
        let field = Field {
            name: "test".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 4,
                stride_bits: 8,
                offset_bits: 0,
            }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };

        let schema = Schema::compile(&vec![field], None).unwrap();
        let data = vec![0x01, 0x02, 0x03, 0x04];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([(
                "test".to_string(),
                Value::Array(vec![
                    Value::U64(1),
                    Value::U64(2),
                    Value::U64(3),
                    Value::U64(4)
                ])
            )]))
        );
    }

    #[test]
    fn test_get_all_array_with_stride() {
        let id_field = Field {
            name: "id".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 16)],
            transform: None,
        };

        let temperature_field = Field {
            name: "temperature".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(16, 8)],
            transform: None,
        };

        let values_field = Field {
            name: "values".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 5,
                stride_bits: 8,
                offset_bits: 24,
            }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };

        let schema =
            Schema::compile(&vec![id_field, temperature_field, values_field], None).unwrap();

        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let result = schema.parse(&data);
        assert_eq!(
            result,
            Ok(BTreeMap::from([
                ("id".to_string(), Value::U64(258)),
                ("temperature".to_string(), Value::U64(3)),
                (
                    "values".to_string(),
                    Value::Array(vec![
                        Value::U64(4),
                        Value::U64(5),
                        Value::U64(6),
                        Value::U64(7),
                        Value::U64(8)
                    ])
                )
            ]))
        );
    }

    #[test]
    fn test_serialize_single_scalar() {
        let field = Field {
            name: "a".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };

        let schema = Schema::compile(&[field], None).unwrap();

        let obj = BTreeMap::from([("a".to_string(), Value::U64(0xAB))]);

        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0xAB]);
    }

    #[test]
    fn test_serialize_multiple_scalars_linear() {
        let a = Field {
            name: "a".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 4)],
            transform: None,
        };

        let b = Field {
            name: "b".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(4, 4)],
            transform: None,
        };

        let schema = Schema::compile(&[a, b], None).unwrap();

        let obj = BTreeMap::from([
            ("a".to_string(), Value::U64(0b1010)),
            ("b".to_string(), Value::U64(0b0101)),
        ]);

        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0b1010_0101]);
    }

    #[test]
    fn test_serialize_non_sequential_fragments() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(4, 2), Fragment::new(0, 2)],
            transform: None,
        };

        let schema = Schema::compile(&[field], None).unwrap();

        // value = 0b1101
        let obj = BTreeMap::from([("x".to_string(), Value::U64(0b1101))]);

        // total_bits = 4 (len 2 + len 2). Fragment (4,2) gets shift=2, writes
        // bits [3..2] of value (0b11) at bit offsets 4..5 → 0b0000_1100.
        // Fragment (0,2) gets shift=0, writes bits [1..0] of value (0b01) at
        // bit offsets 0..1 → 0b0100_1100.
        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0b0100_1100]);
    }

    #[test]
    fn test_serialize_array_dense() {
        let field = Field {
            name: "arr".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 3,
                stride_bits: 8,
                offset_bits: 0, // irrelevant for serialize
            }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };

        let schema = Schema::compile(&[field], None).unwrap();

        let obj = BTreeMap::from([(
            "arr".to_string(),
            Value::Array(vec![Value::U64(1), Value::U64(2), Value::U64(3)]),
        )]);

        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);
    }

    #[test]
    fn test_serialize_respects_fragment_offsets() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(4, 4)],
            transform: None,
        };
        let schema = Schema::compile(&[field], None).unwrap();

        let obj = BTreeMap::from([("x".to_string(), crate::value::Value::U64(0b1011))]);
        let bytes = schema.serialize(&obj).unwrap();
        assert_eq!(bytes, vec![0b0000_1011]);
    }

    #[test]
    fn test_serialize_parse_roundtrip_offset() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(4, 4)],
            transform: None,
        };
        let schema = Schema::compile(&[field], None).unwrap();

        let obj = BTreeMap::from([("x".to_string(), crate::value::Value::U64(0b1011))]);
        let bytes = schema.serialize(&obj).unwrap();
        let parsed = schema.parse(&bytes).unwrap();
        assert_eq!(parsed.get("x"), Some(&crate::value::Value::U64(0b1011)));
    }

    #[test]
    fn test_serialize_parse_roundtrip_dense() {
        let field = Field {
            name: "x".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };

        let schema = Schema::compile(&[field], None).unwrap();

        let obj = BTreeMap::from([("x".to_string(), Value::U64(42))]);

        let bytes = schema.serialize(&obj).unwrap();
        let parsed = schema.parse(&bytes).unwrap();

        assert_eq!(parsed.get("x"), Some(&Value::U64(42)));
    }
}
