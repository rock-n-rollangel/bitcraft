//! # bitcraft
//!
//! A library for bit-level parsing of binary data using declarative schemas.
//!
//! Define fields as bit ranges (possibly non-contiguous), specify signedness and
//! bit order, then parse byte slices into structured values. Supports scalar
//! fields and fixed-length arrays with configurable stride.
//!
//! ## Example
//!
//! ```
//! use bitcraft::schema::{Schema, WriteConfig};
//! use bitcraft::field::{Field, FieldKind};
//! use bitcraft::fragment::Fragment;
//! use bitcraft::assembly::{Assemble, BitOrder};
//!
//! let fields = vec![
//!     Field {
//!         name: "id".to_string(),
//!         kind: FieldKind::Scalar,
//!         signed: false,
//!         assemble: Assemble::Concat(BitOrder::MsbFirst),
//!         fragments: vec![Fragment { offset_bits: 0, len_bits: 8, ..Default::default() }],
//!     },
//! ];
//! let schema = Schema::compile(&fields, Some(WriteConfig::default())).unwrap();
//! let parsed = schema.parse(&[0x42]).unwrap();
//! assert_eq!(parsed.get("id"), Some(&bitcraft::assembly::Value::U64(0x42)));
//! ```

pub mod assembly;
pub mod bits;
pub mod compiled;
pub mod errors;
pub mod field;
pub mod fragment;
pub mod schema;

#[cfg(feature = "serde")]
pub mod serde;

#[cfg(feature = "transform")]
pub mod transform;
