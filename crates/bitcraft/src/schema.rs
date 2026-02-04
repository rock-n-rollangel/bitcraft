//! Schema: compiled set of fields used to parse byte slices into named values.

use std::collections::BTreeMap;

use crate::{
    assembly::{ArrayCount, Value},
    compiled::{CompiledField, CompiledFieldKind},
    errors::{ReadError, CompileError},
    field::Field,
};

/// A compiled schema: list of [CompiledField]s and total bit length. Use [Schema::compile] to build from [Field]s, then [Schema::parse] to parse bytes.
pub struct Schema {
    total_bits: usize,
    /// Compiled fields in definition order.
    pub fields: Vec<CompiledField>,
}

impl Schema {
    /// Compiles a slice of [Field]s into a schema. Fails if any field is invalid.
    pub fn compile(fields: &[Field]) -> Result<Self, CompileError> {
        let mut compiled_fields: Vec<CompiledField> = Vec::with_capacity(fields.len());
        let mut total_bits = 0;

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

            compiled_fields.push(compiled_field);
        }

        Ok(Self {
            fields: compiled_fields,
            total_bits,
        })
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
}

#[cfg(test)]
mod tests {
    use crate::{
        assembly::Assemble,
        field::{ArraySpec, Field, FieldKind},
        fragment::Fragment,
    };

    use super::*;

    #[test]
    fn test_get_all_empty() {
        let schema = Schema::compile(&vec![]).unwrap();
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
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 1,
                ..Default::default()
            }],
        };
        let schema = Schema::compile(&vec![field]).unwrap();
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
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };
        let field2 = Field {
            name: "test2".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 8,
                len_bits: 16,
                ..Default::default()
            }],
        };
        let schema = Schema::compile(&vec![field1, field2]).unwrap();
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
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&vec![field]).unwrap();
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
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 16,
                ..Default::default()
            }],
        };

        let temperature_field = Field {
            name: "temperature".to_string(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 16,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let values_field = Field {
            name: "values".to_string(),
            kind: FieldKind::Array(ArraySpec {
                count: 5,
                stride_bits: 8,
                offset_bits: 24,
            }),
            signed: false,
            assemble: Assemble::ConcatMsb,
            fragments: vec![Fragment {
                offset_bits: 0,
                len_bits: 8,
                ..Default::default()
            }],
        };

        let schema = Schema::compile(&vec![id_field, temperature_field, values_field]).unwrap();

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
}
