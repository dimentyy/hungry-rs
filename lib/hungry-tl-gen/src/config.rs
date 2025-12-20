use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub name: String,
    pub impl_debug: bool,
    pub derive_clone: bool,
    pub impl_into_enum: bool,
}

pub(crate) struct Prepared {
    pub(crate) schema: Config,
    pub(crate) out_dir: PathBuf,
    pub(crate) derive: String,
}
