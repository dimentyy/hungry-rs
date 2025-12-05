mod arg;
mod combinator;
mod comments;
mod error;
mod ident;
mod opts;
mod typ;

use chumsky::prelude::*;

pub use arg::{Arg, ArgTyp, Flag};
pub use combinator::Combinator;
pub use comments::{Comment, CommentVariant};
pub use error::Error;
pub use ident::Ident;
pub use opts::{OptArgs, OptArgsTyp};
pub use typ::Typ;

use crate::Cfg;
use crate::category::Category;

pub(crate) type Extra<'a> = extra::Err<Error<'a>>;

pub(crate) trait ParserExtras<'src, T>: Parser<'src, &'src str, T, Extra<'src>> {}
impl<'src, T, A: Parser<'src, &'src str, T, Extra<'src>>> ParserExtras<'src, T> for A {}

#[derive(Debug)]
pub enum Item<'a> {
    Comment(Comment<'a>),
    Combinator(Combinator<'a>),
    Separator(Category),
}

pub(crate) fn parse<'a>(_config: &Cfg, schema: &'a str) -> ParseResult<Vec<Item<'a>>, Error<'a>> {
    choice((
        Comment::parser().map(Item::Comment),
        Combinator::parser().map(Item::Combinator),
        Category::separator_parser().map(Item::Separator),
    ))
    .padded()
    .repeated()
    .collect()
    .parse(schema)
}
