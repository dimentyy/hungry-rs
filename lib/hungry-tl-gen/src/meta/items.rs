use crate::{read, Ident};
use crate::meta::{Combinator, Typ};

#[derive(Debug)]
pub(crate) struct Type<'a> {
    pub(crate) parsed: &'a read::Combinator<'a>,
    pub(crate) combinator: Combinator,
    pub(crate) enum_index: usize,
    pub(crate) recursive: bool,
}

#[derive(Debug)]
pub(crate) struct Func<'a> {
    pub(crate) parsed: &'a read::Combinator<'a>,
    pub(crate) combinator: Combinator,
    pub(crate) response: Typ, 
}

#[derive(Debug)]
pub(crate) struct Enum<'a> {
    pub(crate) parsed: &'a Ident<&'a str>,
    pub(crate) ident: Ident,
    pub(crate) variants: Vec<usize>,
}
