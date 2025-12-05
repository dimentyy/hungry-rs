use indexmap::{IndexMap, IndexSet};
use std::ops::Index;

use crate::meta::Error;
use crate::{Category, read};

pub(super) struct Temp<'a> {
    pub(super) types: IndexMap<&'a read::Ident<'a>, (&'a read::Combinator<'a>, usize)>,
    pub(super) funcs: IndexMap<&'a read::Ident<'a>, &'a read::Combinator<'a>>,
    pub(super) enums: IndexMap<&'a read::Ident<'a>, Vec<usize>>,
}

impl<'a> Temp<'a> {
    pub(super) fn build(parsed: &'a [read::Item<'a>]) -> Result<Self, Error> {
        let mut section = Category::default();

        let mut types = IndexMap::new();
        let mut funcs = IndexMap::new();
        let mut enums = IndexMap::new();

        for item in parsed {
            let combinator = match item {
                read::Item::Comment(comment) => {
                    continue;
                }
                read::Item::Combinator(combinator) => combinator,
                read::Item::Separator(category) => {
                    section = *category;
                    continue;
                }
            };

            match section {
                Category::Types => {
                    let index = types.len();

                    let mut entry = enums.entry(&combinator.result.ident);

                    let enum_index = entry.index();

                    entry.or_insert_with(|| Vec::with_capacity(1)).push(index);

                    if let Some(_) = types.insert(&combinator.ident, (combinator, enum_index)) {
                        unimplemented!()
                    }
                }
                Category::Funcs => {
                    if let Some(_) = funcs.insert(&combinator.ident, combinator) {
                        unimplemented!()
                    }
                }
            }
        }

        Ok(Self {
            types,
            funcs,
            enums,
        })
    }
}
