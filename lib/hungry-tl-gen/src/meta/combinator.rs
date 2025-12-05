use indexmap::IndexMap;

use crate::meta::{Error, Name, Typ};
use crate::read;

#[derive(Debug)]
pub struct Flag {
    pub arg: usize,
    pub bit: usize,
}

impl Flag {
    pub(crate) fn find(
        args: &mut IndexMap<&str, Arg>,
        index: usize,
        flag: &read::Flag,
    ) -> Result<Self, Error> {
        let Some(arg) = args.get_index_of(flag.ident) else {
            unimplemented!()
        };

        match &mut args.get_index_mut(arg).unwrap().1.typ {
            ArgTyp::Flags { args } => {
                args.push(index);
                Ok(Self {
                    arg,
                    bit: flag.bit.unwrap_or(0),
                })
            }
            ArgTyp::Typ { .. } => unimplemented!(),
            ArgTyp::True { .. } => unimplemented!(),
        }
    }
}

#[derive(Debug)]
pub enum ArgTyp {
    Flags { args: Vec<usize> },
    Typ { typ: Typ, flag: Option<Flag> },
    True { flag: Flag },
}

#[derive(Debug)]
pub struct Arg {
    pub name: String,
    pub typ: ArgTyp,
}

#[derive(Debug)]
pub struct GenericArg {
    pub name: String,
}

#[derive(Debug)]
pub struct Combinator {
    pub name: Name,
    pub explicit_id: Option<u32>,
    pub inferred_id: u32,
    pub args: Vec<Arg>,
    pub generic_args: Vec<GenericArg>,
}
