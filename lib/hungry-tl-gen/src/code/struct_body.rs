use std::io::{Result, Write};

use crate::code::{write_derive_macros, write_escaped, write_typ};
use crate::meta::{Arg, ArgTyp, Combinator, Data, GenericArg, Typ};
use crate::{Cfg, F};

fn write_arg(
    f: &mut F,
    cfg: &Cfg,
    data: &Data,
    generic_args: &Vec<GenericArg>,
    arg: &Arg,
) -> Result<()> {
    let (typ, optional) = match &arg.typ {
        ArgTyp::Flags { .. } => return Ok(()),
        ArgTyp::Typ { typ, flag } => (typ, flag.is_some()),
        ArgTyp::True { .. } => (&Typ::Bool, false),
    };

    f.write_all(b"    pub ")?;
    write_escaped(f, &arg.name)?;
    f.write_all(b": ")?;
    if optional {
        f.write_all(b"Option<")?;
    }
    write_typ(f, cfg, data, generic_args, typ, false)?;
    if optional {
        f.write_all(b">")?;
    }
    f.write_all(b",\n")
}

pub(super) fn write_struct_body(f: &mut F, cfg: &Cfg, data: &Data, x: &Combinator) -> Result<()> {
    write_derive_macros(f, cfg)?;
    f.write_all(b"pub struct ")?;
    write_escaped(f, &x.name.actual)?;

    let mut iter = x.generic_args.iter();

    if let Some(arg) = iter.next() {
        f.write_all(b"<")?;
        f.write_all(arg.name.as_bytes())?;
        f.write_all(b": crate::Function")?;

        for arg in iter {
            f.write_all(b", ")?;
            f.write_all(arg.name.as_bytes())?;
            f.write_all(b": crate::Function")?;
        }

        f.write_all(b">")?;
    }

    if x.args.is_empty() {
        return f.write_all(b" {}\n");
    };

    f.write(b" {\n")?;

    for arg in &x.args {
        write_arg(f, cfg, data, &x.generic_args, arg)?;
    }

    f.write_all(b"}\n")
}
