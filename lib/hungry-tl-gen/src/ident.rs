use std::fmt;

use chumsky::prelude::*;

use crate::read::{Error, ParserExtras};
use crate::rust;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ident<S: AsRef<str> = String> {
    pub space: Option<S>,
    pub name: S,
}

impl<S: AsRef<str>> fmt::Display for Ident<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref space) = self.space {
            f.write_str(space.as_ref())?;
            f.write_str(".")?;
        }

        f.write_str(self.name.as_ref())
    }
}

impl<S: AsRef<str>> Ident<S> {
    pub fn as_ref(&self) -> Ident<&str> {
        Ident {
            space: self.space.as_ref().map(AsRef::as_ref),
            name: self.name.as_ref(),
        }
    }

    pub fn to_rust(&self) -> Ident<String> {
        Ident {
            space: self.space.as_ref().map(AsRef::as_ref).map(rust::snake_case),
            name: rust::pascal_case(self.name.as_ref()),
        }
    }
}

impl<'src> Ident<&'src str> {
    pub const TRUE: Self = Self {
        space: None,
        name: "true",
    };
    
    pub(crate) fn string_parser() -> impl ParserExtras<'src, &'src str> + Copy {
        let first = any().filter(char::is_ascii_alphabetic);

        let other = any().filter(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'));

        first.then(other.repeated()).to_slice()
    }

    pub(crate) fn parser() -> impl ParserExtras<'src, Self> {
        let ident = Self::string_parser();

        ident
            .then(just('.').ignore_then(ident).or_not())
            .try_map(Self::parser_try_map)
    }

    fn parser_try_map(
        (l, r): (&'src str, Option<&'src str>),
        span: SimpleSpan,
    ) -> Result<Self, Error<'src>> {
        if let Some(name) = r {
            if !l.chars().next().unwrap().is_ascii_lowercase() {
                return Err(Error::custom(
                    span,
                    "identifier space must begin with lowercase ASCII character",
                ));
            }

            Ok(Self {
                space: Some(l),
                name,
            })
        } else {
            Ok(Self {
                space: None,
                name: l,
            })
        }
    }

    pub(crate) fn try_map_uppercase(self, span: SimpleSpan) -> Result<Self, Error<'src>> {
        if !self.name.starts_with(|c: char| c.is_ascii_uppercase()) {
            return Err(Error::custom(
                span,
                "identifier name must begin with uppercase ASCII character",
            ));
        }

        Ok(self)
    }

    pub(crate) fn try_map_lowercase(self, span: SimpleSpan) -> Result<Self, Error<'src>> {
        if !self.name.starts_with(|c: char| c.is_ascii_lowercase()) {
            return Err(Error::custom(
                span,
                "identifier name must begin with lowercase ASCII character",
            ));
        }

        Ok(self)
    }
}
