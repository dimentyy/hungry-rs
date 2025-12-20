mod combinator;
mod data;
mod items;
mod temp;
mod typ;

use crate::read;

pub(crate) use combinator::{Arg, ArgTyp, Combinator, Flag, GenericArg};
pub(crate) use data::Data;
pub(crate) use items::{Enum, Func, Type};
pub(crate) use temp::Temp;
pub(crate) use typ::Typ;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum TypeOrEnum {
    Type(usize),
    Enum(usize),
}

pub(crate) fn validate<'a>(parsed: &[&'a [read::Item<'a>]]) -> Data<'a> {
    let temp = Temp::validate(parsed);

    let data = Data::validate(temp);

    data
}
