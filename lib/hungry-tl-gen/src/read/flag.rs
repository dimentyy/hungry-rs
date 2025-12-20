use std::fmt;

use chumsky::prelude::*;

use crate::Ident;
use crate::read::{Error, ParserExtras};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Flag<'a> {
    pub ident: &'a str,
    pub bit: Option<usize>,
}

impl fmt::Display for Flag<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.ident.as_ref())?;
        if let Some(bit) = self.bit {
            f.write_str(".")?;
            bit.fmt(f)?;
        }
        f.write_str("?")
    }
}

impl<'src> Flag<'src> {
    pub(crate) fn parser() -> impl ParserExtras<'src, Self> {
        let ident = Ident::string_parser();

        let bit = any()
            .filter(char::is_ascii_digit)
            .repeated()
            .at_least(1)
            .to_slice()
            .try_map(|bit: &str, span| {
                usize::from_str_radix(bit, 10).map_err(|err| Error::custom(span, err))
            });

        ident
            .then(just('.').ignore_then(bit).or_not())
            .then_ignore(just('?'))
            .map(|(ident, bit)| Self { ident, bit })
    }
}
