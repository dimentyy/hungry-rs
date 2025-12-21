use crate::meta::{Combinator, Deserialization, Ident, Typ};
use crate::read;

#[derive(Debug)]
pub(crate) struct Type<'a> {
    pub(crate) combinator: Combinator<'a>,
    pub(crate) enum_index: usize,
    pub(crate) recursive: bool,
}

#[derive(Debug)]
pub(crate) struct Func<'a> {
    pub(crate) combinator: Combinator<'a>,
    pub(crate) response: Typ,
}

#[derive(Debug)]
pub(crate) struct Enum<'a> {
    pub(crate) parsed: &'a read::Ident<'a>,
    pub(crate) ident: Ident,
    pub(crate) variants: Vec<usize>,
    pub(crate) de: Deserialization,
}
