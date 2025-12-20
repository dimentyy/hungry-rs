mod arg;
mod combinator;
mod comments;
mod opt;
mod typ;
pub mod flag;

use chumsky::prelude::*;

use crate::Category;

pub use arg::{Arg, ArgTyp};
pub use combinator::Combinator;
pub use comments::{Comment, CommentVariant};
pub use opt::{OptArg, OptArgTyp};
pub use typ::Typ;
pub use flag::Flag;

pub type Error<'a> = Rich<'a, char>;

pub(crate) type Extra<'a> = extra::Err<Error<'a>>;

pub(crate) trait ParserExtras<'src, T>: Parser<'src, &'src str, T, Extra<'src>> {}
impl<'src, T, A: Parser<'src, &'src str, T, Extra<'src>>> ParserExtras<'src, T> for A {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item<'a> {
    Comment(Comment<'a>),
    Combinator(Combinator<'a>),
    Separator(Category),
}

pub(crate) fn parse(schema: &'_ str) -> ParseResult<Vec<Item<'_>>, Error<'_>> {
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
