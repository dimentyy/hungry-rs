use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    pub name: String,
    pub impl_debug: bool,
    pub derive_clone: bool,
    pub impl_into_enum: bool,
}

pub(crate) struct Prepared {
    pub(crate) schema: Schema,
    pub(crate) out_dir: PathBuf,
    pub(crate) derive: String,
}
