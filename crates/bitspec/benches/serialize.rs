use bitspec::{
    assembly::{ArrayCount, Assemble, BitOrder},
    field::{ArraySpec, Field, FieldKind},
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

fn array_schema(count: ArrayCount, offset_bits: usize, n: usize) -> (Schema, BTreeMap<String, Value>) {
    let mut fields = Vec::new();
    if matches!(count, ArrayCount::FromField(_)) {
        fields.push(Field {
            name: "len".into(),
            kind: FieldKind::Scalar,
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 16)],
            transform: None,
        });
    }
    fields.push(Field {
        name: "arr".into(),
        kind: FieldKind::Array(ArraySpec { count, stride_bits: 8, offset_bits }),
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(0, 8)],
        transform: None,
    });
    let schema = Schema::compile(&fields, None).unwrap();
    let values: Vec<Value> = (0..n).map(|i| Value::U64((i % 256) as u64)).collect();
    let obj = BTreeMap::from([("arr".to_string(), Value::Array(values))]);
    (schema, obj)
}

fn bench_serialize_array(c: &mut Criterion) {
    for &n in &[10usize, 1000] {
        let (schema, obj) = array_schema(ArrayCount::Fixed(n), 0, n);
        c.bench_function(&format!("serialize_array_{}", n), |b| {
            b.iter(|| schema.serialize(&obj).unwrap());
        });
    }
}

fn bench_serialize_dynamic_array(c: &mut Criterion) {
    for &n in &[10usize, 1000] {
        let (schema, obj) = array_schema(ArrayCount::FromField("len".into()), 16, n);
        c.bench_function(&format!("serialize_dynamic_array_{}", n), |b| {
            b.iter(|| schema.serialize(&obj).unwrap());
        });
    }
}

criterion_group!(
    benches,
    bench_serialize_scalars,
    bench_serialize_array,
    bench_serialize_dynamic_array
);
criterion_main!(benches);
