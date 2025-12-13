use std::fmt;

use chumsky::prelude::*;

use crate::read::{Arg, Error, Ident, OptArgs, ParserExtras, Typ};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Combinator<'a> {
    pub ident: Ident<'a>,
    pub name: Option<u32>,
    pub opts: Vec<OptArgs<'a>>,
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
            f.write_str(&format!("{:08x}", name))?;
        }

        for arg in &self.args {
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

        let name = just('#')
            .ignore_then(text::digits(16).at_most(8).to_slice())
            .try_map(|s, span| u32::from_str_radix(s, 16).map_err(|e| Error::custom(span, e)));

        let opts = OptArgs::parser().padded().repeated().collect();
        let args = Arg::parser().padded().repeated().collect();

        let result = Typ::parser(Ident::parser().try_map(Ident::try_map_uppercase));

        ident
            .then(name.or_not())
            .then(opts)
            .then(args)
            .then_ignore(just('=').padded())
            .then(result)
            .then_ignore(just(';').padded())
            .map(|((((ident, name), opts), args), result)| Combinator {
                ident,
                name,
                opts,
                args,
                result,
            })
    }

    pub(crate) fn infer_name(&self) -> u32 {
        crc32fast::hash(format!("{:#}", self).as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// https://core.telegram.org/mtproto/TL#example
    #[test]
    fn test_calc_name() {
        const TEST: &str = "user id:int first_name:string last_name:string = User;";

        let inferred = Combinator::parser().parse(TEST).unwrap().infer_name();

        assert_eq!(inferred, 0xd23c81a3);
    }
}
