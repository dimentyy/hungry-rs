use std::fmt;

use chumsky::prelude::*;

use crate::read::{Arg, Error, Ident, OptArg, ParserExtras, Typ};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Combinator<'a> {
    pub ident: Ident<'a>,
    pub name: Option<u32>,
    pub opts: Vec<OptArg<'a>>,
    pub args: Vec<Arg<'a>>,
    pub result: Typ<'a>,
}

impl fmt::Display for Combinator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ident.fmt(f)?;

        if !f.alternate()
            && let Some(name) = self.name
        {
            f.write_str("#")?;
            f.write_fmt(format_args!("{name:08x}"))?;
        }

        for opt in self.opts.iter() {
            f.write_str(" ")?;
            opt.fmt(f)?;
        }

        for arg in self.args.iter() {
            f.write_str(" ")?;
            arg.fmt(f)?;
        }

        f.write_str(" = ")?;

        self.result.fmt(f)?;

        if !f.alternate() {
            f.write_str(";")?;
        }

        Ok(())
    }
}

impl<'src> Combinator<'src> {
    pub(super) fn parser() -> impl ParserExtras<'src, Self> {
        let ident = Ident::parser().try_map(Ident::try_map_lowercase);

        let name = any()
            .filter(char::is_ascii_hexdigit)
            .repeated()
            .at_least(1)
            .to_slice()
            .try_map(|name, span| {
                u32::from_str_radix(name, 16).map_err(|e| Error::custom(span, e))
            });

        let opts = OptArg::parser().padded().repeated().collect();
        let args = Arg::parser().padded().repeated().collect();

        let result = Typ::parser(Ident::parser().try_map(Ident::try_map_uppercase));

        ident
            .then(just('#').ignore_then(name).or_not())
            .then(opts)
            .then(args)
            .then_ignore(just('=').padded())
            .then(result.padded())
            .then_ignore(just(';'))
            .map(|((((ident, name), opts), args), result)| Combinator {
                ident,
                name,
                opts,
                args,
                result,
            })
    }

    pub(crate) fn infer_name(&self) -> u32 {
        crc32fast::hash(format!("{self:#}").as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! args {
        ( $( $ident:literal : $typ:literal ),+ $( , )? ) => {
            vec![$( crate::read::Arg {
                ident: $ident,
                typ: crate::read::ArgTyp::Typ {
                    excl_mark: false,
                    typ: Typ {
                        ident: Ident { space: None, name: $typ },
                        params: vec![]
                    },
                    flag: None,
                },
            } ),+]
        };
    }

    /// https://core.telegram.org/mtproto/TL#example
    #[test]
    fn test_combinator_parser() {
        const TEST: &str = "user id:int first_name:string last_name:string = User;";

        let combinator = Combinator::parser().parse(TEST).unwrap();

        assert_eq!(combinator.infer_name(), 0xd23c81a3);

        assert_eq!(
            combinator,
            Combinator {
                ident: Ident {
                    space: None,
                    name: "user"
                },
                name: None,
                opts: vec![],
                args: args![
                    "id": "int",
                    "first_name": "string",
                    "last_name": "string",
                ],
                result: Typ {
                    ident: Ident {
                        space: None,
                        name: "User",
                    },
                    params: vec![]
                },
            }
        )
    }
}
