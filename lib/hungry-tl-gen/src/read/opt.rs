use std::fmt;

use chumsky::prelude::*;

use crate::Ident;
use crate::read::ParserExtras;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OptArgTyp {
    Type,
}

impl OptArgTyp {
    fn parser<'src>() -> impl ParserExtras<'src, Self> {
        choice((just("Type").to(OptArgTyp::Type),))
    }
}

impl fmt::Display for OptArgTyp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OptArgTyp::Type => "Type",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptArg<'a> {
    pub ident: &'a str,
    pub typ: OptArgTyp,
}

impl fmt::Display for OptArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            f.write_str("{")?;
        }
        f.write_str(self.ident)?;
        f.write_str(":")?;
        self.typ.fmt(f)?;
        if !f.alternate() {
            f.write_str("}")?;
        }
        Ok(())
    }
}

impl<'src> OptArg<'src> {
    pub(super) fn parser() -> impl ParserExtras<'src, Self> {
        let ident = Ident::string_parser();
        let typ = OptArgTyp::parser();

        just('{')
            .ignore_then(ident.padded())
            .then_ignore(just(':'))
            .then(typ.padded())
            .then_ignore(just('}'))
            .map(|(ident, typ)| Self { ident, typ })
    }
}
