use std::collections::HashMap;
use std::io::{Result, Write};

use crate::code::generic::write_generics;
use crate::code::{X, write_enum_variant, write_escaped, write_name};
use crate::meta::{Arg, ArgTyp, Combinator, Data, Enum, Flag, Typ};
use crate::{Cfg, F};

fn write_after_f(f: &mut F) -> Result<()> {
    f.write_all(b"f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        ")
}

fn write_struct_debug(f: &mut F, cfg: &Cfg, data: &Data, x: &Combinator) -> Result<()> {
    if x.args.is_empty() {
        f.write_all(b"_")?;
    }

    write_after_f(f)?;

    if x.args.is_empty() {
        return f.write_all(b"Ok(())\n");
    }

    f.write_all(b"f.debug_struct(\"")?;
    f.write_all(x.name.actual.as_bytes())?;
    f.write_all(b"\")\n")?;

    for (i, arg) in x.args.iter().enumerate() {
        let (typ, optional) = match &arg.typ {
            ArgTyp::Flags { args } => continue,
            ArgTyp::Typ { typ, flag } => (typ, flag.is_some()),
            ArgTyp::True { flag } => (&Typ::Bool, false),
        };

        f.write_all(b"            .field(\"")?;
        write_escaped(f, &arg.name)?;
        f.write_all(b"\", ")?;

        if optional {
            match typ {
                Typ::Int128 | Typ::Int256 => {
                    f.write_all(b"&if let Some(x) = &self.")?;
                    write_escaped(f, &arg.name)?;
                    f.write_all(b" { Some(crate::hex::HexIntFmt(x)) } else { None })\n")?;
                }
                Typ::Bytes => {
                    f.write_all(b"&if let Some(x) = &self.")?;
                    write_escaped(f, &arg.name)?;
                    f.write_all(b" { Some(crate::hex::HexBytesFmt(x)) } else { None })\n")?;
                }
                _ => {
                    f.write_all(b"&self.")?;
                    write_escaped(f, &arg.name)?;
                    f.write_all(b")\n")?;
                }
            }
        } else {
            match typ {
                Typ::Int128 | Typ::Int256 => {
                    f.write_all(b"&crate::hex::HexIntFmt(&self.")?;
                    write_escaped(f, &arg.name)?;
                    f.write_all(b"))\n")?;
                }
                Typ::Bytes => {
                    f.write_all(b"&crate::hex::HexBytesFmt(&self.")?;
                    write_escaped(f, &arg.name)?;
                    f.write_all(b"))\n")?;
                }
                _ => {
                    f.write_all(b"&self.")?;
                    write_escaped(f, &arg.name)?;
                    f.write_all(b")\n")?;
                }
            }
        }
    }

    f.write_all(b"            .finish()\n")
}

fn write_enum_debug(f: &mut F, cfg: &Cfg, data: &Data, x: &Enum) -> Result<()> {
    write_after_f(f)?;
    f.write_all(b"match self {\n")?;

    for variant in &x.variants {
        let x = &data.types[*variant];

        f.write_all(b"            Self::")?;
        write_enum_variant(f, cfg, x)?;
        f.write_all(b"(x) => x.fmt(f),\n")?;
    }

    f.write_all(b"        }\n")
}

pub(super) fn write_debug(f: &mut F, cfg: &Cfg, data: &Data, x: X) -> Result<()> {
    f.write_all(b"\nimpl")?;
    match x {
        X::Func(x) => write_generics(f, cfg, &x.combinator.generic_args, false)?,
        _ => {}
    }
    f.write_all(b" std::fmt::Debug for ")?;
    write_escaped(f, &x.name().actual)?;
    match x {
        X::Func(x) => write_generics(f, cfg, &x.combinator.generic_args, true)?,
        _ => {}
    }
    f.write_all(b" {\n    fn fmt(&self, ")?;
    match x {
        X::Type(x) => write_struct_debug(f, cfg, data, &x.combinator)?,
        X::Func(x) => write_struct_debug(f, cfg, data, &x.combinator)?,
        X::Enum(x) => write_enum_debug(f, cfg, data, x)?,
    }
    f.write_all(b"    }\n}\n")
}
