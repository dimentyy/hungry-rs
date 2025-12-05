use crate::meta::{Data, Error, GenericArg, TypeOrEnum};
use std::collections::HashSet;

#[derive(Debug)]
pub enum Typ {
    Type { index: usize, params: Vec<Typ> },

    Enum { index: usize, params: Vec<Typ> },

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
    pub(crate) fn check_recursion(
        &self,
        data: &Data,
        visited: &mut HashSet<TypeOrEnum>,
        root: TypeOrEnum,
    ) -> bool {
        let value = match self {
            Typ::Type { index, params } => {
                assert!(params.is_empty());

                TypeOrEnum::Type(*index)
            }
            Typ::Enum { index, params } => {
                assert!(params.is_empty());

                TypeOrEnum::Enum(*index)
            }
            Typ::BareVector(typ) => return typ.check_recursion(data, visited, root),
            Typ::Vector(typ) => return typ.check_recursion(data, visited, root),
            Typ::Generic { index } => unimplemented!(),
            _ => return false,
        };

        if value == root {
            return true;
        }

        data.check_recursion(visited, root, value)
    }
}
