//! Property: parse(serialize(x)) == x for any valid schema with values in range.

use bitspec::{
    assembly::{Assemble, BitOrder},
    field::{Field, FieldKind},
    fragment::Fragment,
    schema::Schema,
    value::Value,
};
use proptest::prelude::*;
use std::collections::BTreeMap;

/// Generate a field starting at `start_bit` with a random width (1..=32 bits).
/// Each field's unique name is derived from its start bit position.
#[allow(dead_code)]
fn arb_field_and_value(start: usize) -> BoxedStrategy<(Field, (String, Value), usize)> {
    (1usize..=32, any::<u64>(), "[a-z]{2,6}")
        .prop_map(move |(len_bits, raw, name)| {
            let mask = if len_bits == 64 { u64::MAX } else { (1u64 << len_bits) - 1 };
            let value = raw & mask;
            let field = Field {
                name: name.clone(),
                kind: FieldKind::Scalar,
                signed: false,
                assemble: Assemble::Concat(BitOrder::MsbFirst),
                fragments: vec![Fragment::new(start, len_bits)],
                transform: None,
            };
            (field, (name, Value::U64(value)), start + len_bits)
        })
        .boxed()
}

/// Generate a schema (1..=8 fields) paired with a map of values matching the schema.
/// Fields are densely packed (cursor advances by each field's actual length) so no
/// overlaps and no gaps.
fn arb_schema_and_obj() -> BoxedStrategy<(Schema, BTreeMap<String, Value>)> {
    (1usize..=8)
        .prop_flat_map(|n| {
            // Since each arb_field_and_value's starting bit depends on the cumulative
            // length of previous fields, we can't know the positions upfront. Instead,
            // generate N independent lengths first and thread the cursor ourselves.
            let lens = proptest::collection::vec(1usize..=32, n);
            let names = proptest::collection::vec("[a-z]{2,6}", n);
            let raws = proptest::collection::vec(any::<u64>(), n);
            (lens, names, raws).prop_map(|(lens, names, raws)| {
                let mut fields = Vec::new();
                let mut obj = BTreeMap::new();
                let mut seen = std::collections::HashSet::new();
                let mut cursor = 0usize;
                for ((len_bits, name), raw) in lens.into_iter().zip(names).zip(raws) {
                    if !seen.insert(name.clone()) {
                        // Skip duplicate names; the schema would reject them otherwise.
                        continue;
                    }
                    let mask = if len_bits == 64 { u64::MAX } else { (1u64 << len_bits) - 1 };
                    let value = raw & mask;
                    fields.push(Field {
                        name: name.clone(),
                        kind: FieldKind::Scalar,
                        signed: false,
                        assemble: Assemble::Concat(BitOrder::MsbFirst),
                        fragments: vec![Fragment::new(cursor, len_bits)],
                        transform: None,
                    });
                    obj.insert(name, Value::U64(value));
                    cursor += len_bits;
                }
                if fields.is_empty() {
                    // All names collided; insert a trivial 1-bit field so the schema is valid.
                    fields.push(Field {
                        name: "_z".to_string(),
                        kind: FieldKind::Scalar,
                        signed: false,
                        assemble: Assemble::Concat(BitOrder::MsbFirst),
                        fragments: vec![Fragment::new(0, 1)],
                        transform: None,
                    });
                    obj.insert("_z".to_string(), Value::U64(0));
                }
                let schema = Schema::compile(&fields, None).expect("dense schema compiles");
                (schema, obj)
            }).boxed()
        })
        .boxed()
}

proptest! {
    #[test]
    fn roundtrip_holds((schema, obj) in arb_schema_and_obj()) {
        let bytes = schema.serialize(&obj).unwrap();
        let parsed = schema.parse(&bytes).unwrap();
        for (k, v) in &obj {
            prop_assert_eq!(parsed.get(k), Some(v));
        }
    }
}
