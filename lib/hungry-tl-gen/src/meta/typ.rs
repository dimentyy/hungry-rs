use std::collections::HashSet;
use crate::meta::{Data, TypeOrEnum};

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
    pub(super) fn check_recursion(
        &self,
        data: &Data,
        visited: &mut HashSet<TypeOrEnum>,
        root: usize,
    ) -> bool {
        let value = match self {
            Typ::Type { index, params } => {
                if !params.is_empty() {
                    todo!()
                }

                if *index == root {
                    return true;
                }

                TypeOrEnum::Type(*index)
            }
            Typ::Enum { index, params } => {
                if !params.is_empty() {
                    todo!()
                }

                TypeOrEnum::Enum(*index)
            }
            Typ::BareVector(typ) => return typ.check_recursion(data, visited, root),
            Typ::Vector(typ) => return typ.check_recursion(data, visited, root),
            Typ::Generic { .. } => todo!(),
            _ => return false,
        };

        data.check_recursion(visited, root, value)
    }
}
