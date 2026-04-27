use bitspec::{
    assembly::{Assemble, BitOrder},
    field::{ArraySpec, Field, FieldKind},
    fragment::Fragment,
    schema::Schema,
};
use criterion::{Criterion, criterion_group, criterion_main};

fn scalar_field(iter: usize) -> Field {
    Field {
        name: format!("f{}", iter),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![Fragment::new(iter * 16, 16)],
        transform: None,
    }
}

fn scalar_schema(n: usize) -> Schema {
    let fields: Vec<Field> = (0..n).map(scalar_field).collect();
    Schema::compile(&fields, None).unwrap()
}

fn packet(total_bits: usize) -> Vec<u8> {
    let total_bytes = (total_bits + 7) / 8;
    (0..total_bytes).map(|i| (i * 31 % 256) as u8).collect()
}

fn bench_parse_scalars(c: &mut Criterion) {
    for &n in &[1usize, 10, 50, 100] {
        let schema = scalar_schema(n);
        let data = packet(n * 16);
        c.bench_function(&format!("parse_scalars_{}", n), |b| {
            b.iter(|| schema.parse(&data).unwrap());
        });
    }
}

fn bench_parse_array(c: &mut Criterion) {
    for &n in &[10usize, 1000] {
        let field = Field {
            name: "arr".into(),
            kind: FieldKind::Array(ArraySpec { count: n, stride_bits: 8, offset_bits: 0 }),
            signed: false,
            assemble: Assemble::Concat(BitOrder::MsbFirst),
            fragments: vec![Fragment::new(0, 8)],
            transform: None,
        };
        let schema = Schema::compile(&[field], None).unwrap();
        let data = packet(n * 8);
        c.bench_function(&format!("parse_array_{}", n), |b| {
            b.iter(|| schema.parse(&data).unwrap());
        });
    }
}

fn bench_parse_non_contiguous(c: &mut Criterion) {
    let field = Field {
        name: "x".into(),
        kind: FieldKind::Scalar,
        signed: false,
        assemble: Assemble::Concat(BitOrder::MsbFirst),
        fragments: vec![
            Fragment::new(0, 8),
            Fragment::new(16, 8),
            Fragment::new(32, 8),
        ],
        transform: None,
    };
    let schema = Schema::compile(&[field], None).unwrap();
    let data = packet(40);
    c.bench_function("parse_non_contiguous_fragments", |b| {
        b.iter(|| schema.parse(&data).unwrap());
    });
}

criterion_group!(benches, bench_parse_scalars, bench_parse_array, bench_parse_non_contiguous);
criterion_main!(benches);
