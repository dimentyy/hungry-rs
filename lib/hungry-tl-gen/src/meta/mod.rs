mod combinator;
mod data;
mod error;
mod name;
mod temp;
mod typ;

use crate::read;

use temp::Temp;

pub(crate) use data::{Data, Enum, Func, Type};

pub use combinator::{Arg, ArgTyp, Combinator, Flag, GenericArg};
pub use error::Error;
pub use name::Name;
pub use typ::Typ;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TypeOrEnum {
    Type(usize),
    Enum(usize),
}

pub(crate) fn validate<'a>(parsed: &'a [read::Item<'a>]) -> Result<Data, Error> {
    let mut temp = Temp::build(parsed)?;

    let mut data = Data::validate(temp)?;

    Ok(data)
}
