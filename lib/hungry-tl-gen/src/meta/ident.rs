use crate::{read, rust};

#[derive(Debug)]
pub struct Ident {
    pub actual: String,
    pub space: Option<String>,
    pub file: String,
}

impl From<&read::Ident<'_>> for Ident {
    fn from(value: &read::Ident<'_>) -> Self {
        Self {
            actual: rust::pascal_case(value.name),
            space: value.space.map(rust::snake_case),
            file: rust::snake_case(value.name),
        }
    }
}
