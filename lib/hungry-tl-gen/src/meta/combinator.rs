use crate::Ident;
use crate::meta::Typ;

#[derive(Debug)]
pub struct Flag {
    pub arg: usize,
    pub bit: usize,
}

#[derive(Debug)]
pub enum ArgTyp {
    Flags { args: Vec<usize> },
    Typ { typ: Typ, flag: Option<Flag> },
    True { flag: Flag },
}

#[derive(Debug)]
pub struct Arg {
    pub ident: String,
    pub typ: ArgTyp,
}

#[derive(Debug)]
pub struct GenericArg {
    pub ident: String,
}

#[derive(Debug)]
pub struct Combinator {
    pub ident: Ident,
    pub explicit_id: Option<u32>,
    pub inferred_id: u32,
    pub args: Vec<Arg>,
    pub generic_args: Vec<GenericArg>,
}
