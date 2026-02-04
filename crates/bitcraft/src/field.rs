//! Definition of logical fields used to build a [crate::Schema].

/// A single named field in a schema: either a scalar or an array of scalars.
#[derive(Debug, Clone)]
pub struct Field {
    /// Name used in the parsed result map.
    pub name: String,
    /// Whether this is a scalar or an array, and array parameters.
    pub kind: FieldKind,
    /// If true, the assembled value is interpreted as signed and sign-extended.
    pub signed: bool,
    /// How [crate::fragment::Fragment]s are concatenated (MSB-first or LSB-first).
    pub assemble: crate::assembly::Assemble,
    /// Bit ranges that make up this field (one or more, possibly non-contiguous).
    pub fragments: Vec<crate::fragment::Fragment>,
}

/// Distinguishes scalar fields from fixed-length array fields.
#[derive(Debug, Clone)]
pub enum FieldKind {
    /// Single value assembled from one or more fragments.
    Scalar,
    /// Repeated element with fixed count and stride.
    Array(ArraySpec),
}

/// Parameters for an array field: count, stride, and start offset in bits.
#[derive(Debug, Clone)]
pub struct ArraySpec {
    /// Number of elements.
    pub count: usize,
    /// Distance in bits between the start of consecutive elements.
    pub stride_bits: usize,
    /// Bit offset where the first element starts.
    pub offset_bits: usize,
}
