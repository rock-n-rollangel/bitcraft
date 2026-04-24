//! Property: Schema::parse never panics on any byte slice, regardless of
//! whether the slice has sufficient length for the schema. Well-formed result
//! or typed `ReadError` — never a panic.

use bitspec::{
    assembly::{Assemble, BitOrder},
    field::{Field, FieldKind},
    fragment::Fragment,
    schema::Schema,
};
use proptest::prelude::*;

fn arb_field(start_bit: usize) -> BoxedStrategy<(Field, usize)> {
    (1usize..=32)
        .prop_flat_map(move |len_bits| {
            let field = Field {
                name: format!("f_{}", start_bit),
                kind: FieldKind::Scalar,
                signed: false,
                assemble: Assemble::Concat(BitOrder::MsbFirst),
                fragments: vec![Fragment::new(start_bit, len_bits)],
                transform: None,
            };
            Just((field, start_bit + len_bits))
        })
        .boxed()
}

fn arb_dense_schema() -> BoxedStrategy<Schema> {
    (1usize..=8)
        .prop_flat_map(|n_fields| {
            let mut strategies: Vec<BoxedStrategy<(Field, usize)>> = Vec::with_capacity(n_fields);
            let mut cursor = 0usize;
            for _ in 0..n_fields {
                strategies.push(arb_field(cursor));
                cursor += 8;
            }
            strategies
                .into_iter()
                .collect::<Vec<_>>()
                .prop_map(|pairs: Vec<(Field, usize)>| {
                    let fields: Vec<Field> = pairs.into_iter().map(|(f, _)| f).collect();
                    Schema::compile(&fields, None).expect("dense schema compiles")
                })
                .boxed()
        })
        .boxed()
}

proptest! {
    #[test]
    fn parse_never_panics(
        schema in arb_dense_schema(),
        bytes in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        let _ = schema.parse(&bytes);
    }
}
