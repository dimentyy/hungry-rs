use indexmap::IndexMap;
use indexmap::map::Entry;

use crate::{Category, Ident, read};

#[derive(Debug)]
pub(crate) struct TempType<'a> {
    pub(crate) combinator: &'a read::Combinator<'a>,
    pub(crate) enum_index: usize,
}

#[derive(Debug)]
pub(crate) struct TempFunc<'a> {
    pub(crate) combinator: &'a read::Combinator<'a>,
}

#[derive(Debug)]
pub(crate) struct TempEnum {
    pub(crate) bare_types: Vec<usize>,
}

pub(crate) struct Temp<'a> {
    pub(crate) types: IndexMap<&'a Ident<&'a str>, TempType<'a>>,
    pub(crate) funcs: IndexMap<&'a Ident<&'a str>, TempFunc<'a>>,
    pub(crate) enums: IndexMap<&'a Ident<&'a str>, TempEnum>,

    pub(crate) types_split: Vec<usize>,
    pub(crate) funcs_split: Vec<usize>,
    pub(crate) enums_split: Vec<usize>,
}

impl<'a> Temp<'a> {
    /// TODO: errors.
    pub(super) fn validate(parsed: &[&'a [read::Item<'a>]]) -> Self {
        let mut section = Category::default();

        let mut types = IndexMap::new();
        let mut funcs = IndexMap::new();
        let mut enums = IndexMap::<&'a Ident<&'a str>, TempEnum>::new();

        let mut types_split = Vec::with_capacity(parsed.len());
        let mut funcs_split = Vec::with_capacity(parsed.len());
        let mut enums_split = Vec::with_capacity(parsed.len());

        for schema in parsed {
            types_split.push(types.len());
            funcs_split.push(funcs.len());
            enums_split.push(enums.len());

            for item in *schema {
                let combinator @ read::Combinator { ident, result, .. } = match item {
                    read::Item::Comment(_) => {
                        // TODO: comments.
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
                        let type_index = types.len();

                        let enum_entry = enums.entry(&result.ident);
                        let enum_index = enum_entry.index();

                        let temp = TempType {
                            combinator,
                            enum_index,
                        };

                        if let Some(x) = types.insert(ident, temp) {
                            panic!("type declared twice: {x:?}, {:?}", types.get(ident));
                        }

                        match enum_entry {
                            Entry::Occupied(mut entry) => {
                                if entry.index() < *enums_split.last().unwrap() {
                                    panic!()
                                }

                                entry.get_mut().bare_types.push(type_index);
                            }
                            Entry::Vacant(entry) => {
                                entry.insert({
                                    let mut bare_types = Vec::with_capacity(1);
                                    bare_types.push(type_index);

                                    TempEnum { bare_types }
                                });
                            }
                        }
                    }
                    Category::Funcs => {
                        let temp = TempFunc { combinator };

                        if let Some(x) = funcs.insert(ident, temp) {
                            panic!("func declared twice: {x:?}, {:?}", funcs.get(ident));
                        }
                    }
                }
            }
        }

        Self {
            types,
            funcs,
            enums,

            types_split,
            funcs_split,
            enums_split,
        }
    }
}
