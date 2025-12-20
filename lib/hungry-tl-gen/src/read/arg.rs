use std::fmt;

use chumsky::prelude::*;

use crate::ident::Ident;
use crate::read::{Extra, Typ, Flag};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arg<'a> {
    pub ident: &'a str,
    pub typ: ArgTyp<'a>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArgTyp<'a> {
    Typ {
        excl_mark: bool,
        typ: Typ<'a>,
        flag: Option<Flag<'a>>,
    },
    Nat,
}

impl fmt::Display for ArgTyp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgTyp::Typ {
                flag,
                typ,
                excl_mark,
            } => {
                if *excl_mark {
                    f.write_str("!")?;
                }
                if let Some(flag) = flag {
                    flag.fmt(f)?;
                }
                typ.fmt(f)
            }
            ArgTyp::Nat => f.write_str("#"),
        }
    }
}

impl<'src> ArgTyp<'src> {
    pub(super) fn parser() -> impl Parser<'src, &'src str, Self, Extra<'src>> {
        let nat = just('#').to(ArgTyp::Nat);

        let excl_mark = just('!').or_not().map(|x| x.is_some());

        let flag = Flag::parser();
        let typ = Typ::parser(Ident::parser());

        let typ = excl_mark
            .then(flag.or_not())
            .then(typ)
            .map(|((excl_mark, flag), typ)| ArgTyp::Typ {
                excl_mark,
                flag,
                typ,
            });

        choice((nat, typ))
    }
}

impl fmt::Display for Arg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.ident)?;
        f.write_str(":")?;
        self.typ.fmt(f)
    }
}

impl<'src> Arg<'src> {
    pub(super) fn parser() -> impl Parser<'src, &'src str, Self, Extra<'src>> {
        let ident = Ident::string_parser();

        ident
            .then_ignore(just(':'))
            .then(ArgTyp::parser())
            .map(|(ident, typ)| Arg { ident, typ })
    }
}
