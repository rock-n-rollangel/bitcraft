use bitspec::{
    assembly::{Assemble, BitOrder},
    field::{Field, FieldKind},
    fragment::Fragment,
    schema::Schema,
    value::Value,
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::BTreeMap;

fn scalar_schema(n: usize) -> (Schema, BTreeMap<String, Value>) {
    let fields: Vec<Field> = (0..n).map(|i| Field {
        name: format!("f{}", i),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(i * 16, 16)],
        transform: None,
    }).collect();
    let schema = Schema::compile(&fields, None).unwrap();
    let obj: BTreeMap<String, Value> = (0..n)
        .map(|i| (format!("f{}", i), Value::U64((i as u64) * 7)))
        .collect();
    (schema, obj)
}

fn bench_serialize_scalars(c: &mut Criterion) {
    for &n in &[1usize, 10, 50, 100] {
        let (schema, obj) = scalar_schema(n);
        c.bench_function(&format!("serialize_scalars_{}", n), |b| {
            b.iter(|| schema.serialize(&obj).unwrap());
        });
    }
}

criterion_group!(benches, bench_serialize_scalars);
criterion_main!(benches);
