// PERFECT!

use std::fmt;

use chumsky::prelude::*;

use crate::read::ParserExtras;

const DELIMITER: &str = "---";

const TYPES: &str = "types";
const FUNCTIONS: &str = "functions";

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum Category {
    #[default]
    Types,
    Funcs,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(DELIMITER)?;
        f.write_str(match self {
            Category::Types => TYPES,
            Category::Funcs => FUNCTIONS,
        })?;
        f.write_str(DELIMITER)
    }
}

impl Category {
    pub(super) fn separator_parser<'src>() -> impl ParserExtras<'src, Self> {
        let delimiter = just(DELIMITER);

        let category = choice((
            just(TYPES).to(Category::Types),
            just(FUNCTIONS).to(Category::Funcs),
        ));

        delimiter
            .ignore_then(category.padded())
            .then_ignore(delimiter)
    }
}
