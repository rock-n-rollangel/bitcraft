#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    pub signed: bool,
    pub assemble: crate::assembly::Assemble,
    pub fragments: Vec<crate::fragment::Fragment>,
}

#[derive(Debug, Clone)]
pub enum FieldKind {
    Scalar,
    Array(ArraySpec),
}

#[derive(Debug, Clone)]
pub struct ArraySpec {
    pub count: usize,
    pub stride_bits: usize,
    pub offset_bits: usize,
}
