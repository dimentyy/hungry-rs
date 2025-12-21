use std::fmt;

use chumsky::prelude::*;

use crate::read::ParserExtras;

const LINE_START: &str = "//";
const LINE_END: &str = "\n";

const BLOCK_START: &str = "/*";
const BLOCK_END: &str = "*/";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comment<'a> {
    pub variant: CommentVariant,
    pub content: &'a str,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CommentVariant {
    Line,
    Block,
}

impl CommentVariant {
    pub const fn start(&self) -> &'static str {
        match self {
            CommentVariant::Line => LINE_START,
            CommentVariant::Block => BLOCK_START,
        }
    }

    pub const fn end(&self) -> &'static str {
        match self {
            CommentVariant::Line => LINE_END,
            CommentVariant::Block => BLOCK_END,
        }
    }
}

impl<'src> Comment<'src> {
    fn parser_impl(variant: CommentVariant) -> impl ParserExtras<'src, Self> {
        let start = just(variant.start());

        let end = just(variant.end());

        let content = any().and_is(end.not()).repeated().to_slice();

        start
            .ignore_then(content)
            .then_ignore(end)
            .map(move |content| Comment { variant, content })
    }

    pub(super) fn parser() -> impl ParserExtras<'src, Self> {
        choice((
            Self::parser_impl(CommentVariant::Line),
            Self::parser_impl(CommentVariant::Block),
        ))
    }
}

impl fmt::Display for Comment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.variant.start())?;
        f.write_str(self.content)?;
        f.write_str(self.variant.end())
    }
}
