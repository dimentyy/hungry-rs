use std::fmt;

use chumsky::prelude::*;
use chumsky::recursive::{Direct, Recursive};

use crate::read::{Extra, Ident, ParserExtras};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Typ<'a> {
    pub ident: Ident<'a>,
    pub params: Vec<Typ<'a>>,
}

impl fmt::Display for Typ<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ident.fmt(f)?;

        let mut params = self.params.iter();
        if let Some(param) = params.next() {
            f.write_str(if f.alternate() { " " } else { "<" })?;

            param.fmt(f)?;

            let sep = if f.alternate() { " " } else { ", " };

            for param in params {
                f.write_str(sep)?;

                param.fmt(f)?;
            }

            if !f.alternate() {
                f.write_str(">")?;
            }
        }

        Ok(())
    }
}

impl<'src> Typ<'src> {
    fn parser_impl(
        parser: Recursive<Direct<'src, 'src, &'src str, Self, Extra<'src>>>,
        ident: impl ParserExtras<'src, Ident<'src>>,
    ) -> impl ParserExtras<'src, Self> {
        let params = parser
            .padded()
            .separated_by(just(','))
            .at_least(1)
            .collect()
            .delimited_by(just('<'), just('>'))
            .or_not()
            .map(Option::unwrap_or_default);

        ident
            .then(params)
            .map(|(ident, params)| Typ { ident, params })
    }

    pub(super) fn parser(
        ident: impl ParserExtras<'src, Ident<'src>>,
    ) -> impl ParserExtras<'src, Self> {
        let parser = recursive(|parser| Self::parser_impl(parser, Ident::parser()).boxed());

        Self::parser_impl(parser, ident)
    }
}
