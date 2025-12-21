use crate::meta::{Data, Deserialization};

#[derive(Debug)]
pub enum Typ {
    Type { index: usize },

    Enum { index: usize },

    Int,
    Long,
    Double,
    Bytes,
    String,
    Bool,
    BareVector(Box<Typ>),
    Vector(Box<Typ>),

    Int128,
    Int256,

    Generic { index: usize },
}

impl Typ {
    pub(super) fn de(&self, data: &Data, visited_types: &mut Vec<bool>, visited_enums: &mut Vec<bool>) -> Deserialization {
        match self {
            Typ::Type { index } => data.type_de(*index, visited_types, visited_enums),
            Typ::Enum { index } => data.enum_de(*index, visited_types, visited_enums),
            Typ::Int => Deserialization::Infallible(4),
            Typ::Long => Deserialization::Infallible(8),
            Typ::Double => Deserialization::Infallible(8),
            Typ::Bytes => Deserialization::Checked,
            Typ::String => Deserialization::Checked,
            Typ::Bool => Deserialization::Unchecked(4),
            Typ::BareVector(_) => Deserialization::Checked,
            Typ::Vector(_) => Deserialization::Checked,
            Typ::Int128 => Deserialization::Infallible(16),
            Typ::Int256 => Deserialization::Infallible(32),
            Typ::Generic { .. } => Deserialization::Checked,
        }
    }

    pub(crate) fn ready_de(&self, data: &Data) -> Deserialization {
        match self {
            Typ::Type { index } => data.types[*index].combinator.de,
            Typ::Enum { index } => dbg!(&data.enums[*index]).de,
            Typ::Int => Deserialization::Infallible(4),
            Typ::Long => Deserialization::Infallible(8),
            Typ::Double => Deserialization::Infallible(8),
            Typ::Bytes => Deserialization::Checked,
            Typ::String => Deserialization::Checked,
            Typ::Bool => Deserialization::Unchecked(4),
            Typ::BareVector(_) => Deserialization::Checked,
            Typ::Vector(_) => Deserialization::Checked,
            Typ::Int128 => Deserialization::Infallible(16),
            Typ::Int256 => Deserialization::Infallible(32),
            Typ::Generic { .. } => Deserialization::Checked,
        }
    }

    pub(super) fn check_recursion(
        &self,
        data: &Data,
        visited: &mut Vec<Option<Option<bool>>>,
    ) -> bool {
        match self {
            Typ::Type { index } => data.check_recursion(visited, *index),
            Typ::Enum { index } => data.enums[*index]
                .variants
                .iter()
                .any(|&x| data.check_recursion(visited, x)),
            Typ::BareVector(typ) => typ.check_recursion(data, visited),
            Typ::Vector(typ) => typ.check_recursion(data, visited),
            _ => false,
        }
    }
}
