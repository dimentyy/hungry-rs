use std::fmt;

use chumsky::prelude::*;

use crate::read::{Error, ParserExtras};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ident<'a> {
    pub space: Option<&'a str>,
    pub name: &'a str,
}

impl fmt::Display for Ident<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(space) = self.space {
            f.write_str(space)?;
            f.write_str(".")?;
        }

        f.write_str(self.name)
    }
}

impl<'src> Ident<'src> {
    pub(crate) const TRUE: Ident<'static> = Ident {
        space: None,
        name: "true",
    };

    pub(super) fn part_parser() -> impl ParserExtras<'src, &'src str> + Copy {
        let first = any().filter(char::is_ascii_alphabetic);

        let other = any().filter(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'));

        first.then(other.repeated()).to_slice()
    }

    pub(super) fn parser() -> impl ParserExtras<'src, Self> {
        let ident = Self::part_parser();

        ident
            .then(just('.').ignore_then(ident).or_not())
            .try_map(Self::parser_try_map)
    }

    fn parser_try_map(
        (l, r): (&'src str, Option<&'src str>),
        span: SimpleSpan,
    ) -> Result<Self, Error<'src>> {
        if let Some(r) = r {
            if !l.chars().next().unwrap().is_ascii_lowercase() {
                return Err(Error::custom(
                    span,
                    "identifier space must begin with lowercase ASCII character",
                ));
            }

            Ok(Self {
                space: Some(l),
                name: r,
            })
        } else {
            Ok(Self {
                space: None,
                name: l,
            })
        }
    }

    pub(super) fn try_map_uppercase(self, span: SimpleSpan) -> Result<Self, Error<'src>> {
        if !self.name.starts_with(|c: char| c.is_ascii_uppercase()) {
            return Err(Error::custom(
                span,
                "identifier name must begin with uppercase ASCII character",
            ));
        }

        Ok(self)
    }

    pub(super) fn try_map_lowercase(self, span: SimpleSpan) -> Result<Self, Error<'src>> {
        if !self.name.starts_with(|c: char| c.is_ascii_lowercase()) {
            return Err(Error::custom(
                span,
                "identifier name must begin with lowercase ASCII character",
            ));
        }

        Ok(self)
    }
}
