use crate::meta::{Enum, Func, Name, Type};

#[derive(Copy, Clone)]
pub enum X<'a> {
    Type(&'a Type),
    Func(&'a Func),
    Enum(&'a Enum),
}

impl<'a> X<'a> {
    pub(crate) fn name(self) -> &'a Name {
        match self {
            X::Type(x) => &x.combinator.name,
            X::Func(x) => &x.combinator.name,
            X::Enum(x) => &x.name,
        }
    }
}
