mod combinator;
mod data;
mod ident;
mod items;
mod temp;
mod typ;

use crate::read;

pub(crate) use combinator::{Arg, ArgTyp, Combinator, Flag, GenericArg};
pub(crate) use data::Data;
pub(crate) use ident::Ident;
pub(crate) use items::{Enum, Func, Type};
pub(crate) use temp::Temp;
pub(crate) use typ::Typ;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Deserialization {
    Infallible(usize),
    Unchecked(usize),
    Checked,
}

impl Deserialization {
    pub fn const_len(&self) -> Option<usize> {
        match self {
            Deserialization::Infallible(len) => Some(*len),
            Deserialization::Unchecked(len) => Some(*len),
            Deserialization::Checked => None,
        }
    }
}

pub(crate) fn validate<'a>(parsed: &'a [Vec<read::Item<'a>>]) -> Data<'a> {
    let temp = Temp::validate(parsed);

    let data = Data::validate(temp);

    data
}
