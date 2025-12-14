use std::collections::HashMap;
use std::io::{Result, Write};

use crate::code::{X, write_enum_variant, write_escaped, write_generics, write_name};
use crate::meta::{Arg, ArgTyp, Combinator, Data, Enum, Flag};
use crate::{Cfg, F};

fn write_structure_arg_len(f: &mut F, cfg: &Cfg, data: &Data, x: &Arg) -> Result<()> {
    match &x.typ {
        ArgTyp::Flags { .. } => f.write_all(b"4"),
        ArgTyp::Typ { typ, flag } => {
            if flag.is_some() {
                f.write_all(b"if let Some(x) = &")?;
            }
            f.write_all(b"self.")?;
            write_escaped(f, &x.name)?;
            if flag.is_some() {
                f.write_all(b" { x")?;
            }
            f.write_all(b".serialized_len()")?;
            if flag.is_some() {
                f.write_all(b" } else { 0 }")?;
            }

            Ok(())
        }
        ArgTyp::True { .. } => Ok(()),
    }
}

fn write_structure_len(
    f: &mut F,
    cfg: &Cfg,
    data: &Data,
    func: bool,
    x: &Combinator,
) -> Result<()> {
    if x.args.is_empty() {
        return f.write_all(if func { b"4" } else { b"0" });
    }

    let mut iter = x.args.iter();

    if func {
        f.write_all(b"4")?;
    } else {
        write_structure_arg_len(f, cfg, data, iter.next().unwrap())?;
    }

    for arg in iter {
        if matches!(arg.typ, ArgTyp::True { .. }) {
            continue;
        }
        f.write_all(b"\n            + ")?;
        write_structure_arg_len(f, cfg, data, arg)?;
    }

    Ok(())
}

fn write_enum_len(f: &mut F, cfg: &Cfg, data: &Data, x: &Enum) -> Result<()> {
    f.write_all(b"4 + match self {\n")?;

    for variant in &x.variants {
        let x = &data.types[*variant];

        f.write_all(b"            Self::")?;
        write_enum_variant(f, cfg, x)?;
        f.write_all(b"(x) => x.serialized_len(),\n")?;
    }

    f.write_all(b"        }")
}

fn write_flag_arg(f: &mut F, cfg: &Cfg, x: &Combinator, i: usize) -> Result<()> {
    let arg = &x.args[i];

    let (bit, opt) = match &arg.typ {
        ArgTyp::Typ {
            flag: Some(Flag { bit, .. }),
            ..
        } => (*bit, true),
        ArgTyp::True {
            flag: Flag { bit, .. },
        } => (*bit, false),
        _ => unreachable!(),
    };

    f.write_all(if bit > 0 { b"(self." } else { b"self." })?;
    write_escaped(f, &arg.name)?;
    if opt {
        f.write_all(b".is_some()")?;
    }
    f.write_all(b" as u32")?;
    if bit > 0 {
        f.write_all(b") << ")?;
        write!(f, "{bit}")?;
    }
    Ok(())
}

fn write_structure_ser(
    f: &mut F,
    cfg: &Cfg,
    data: &Data,
    func: bool,
    x: &Combinator,
) -> Result<()> {
    if x.args.is_empty() && !func {
        return f.write_all(b"buf");
    }

    f.write_all(b"unsafe {\n")?;

    if func {
        f.write_all(b"            buf = Self::CONSTRUCTOR_ID.serialize_unchecked(buf);\n")?;
    }

    for (i, arg) in x.args.iter().enumerate() {
        let (typ, optional) = match &arg.typ {
            ArgTyp::Flags { args } => {
                f.write_all(b"            buf = ")?;
                if args.is_empty() {
                    f.write_all(b"0u32")?;
                } else {
                    f.write_all(b"(")?;
                    for arg in &args[..args.len() - 1] {
                        write_flag_arg(f, cfg, x, *arg)?;

                        f.write_all(b" | ")?;
                    }

                    write_flag_arg(f, cfg, x, *args.last().unwrap())?;

                    f.write_all(b")")?;
                }
                f.write_all(b".serialize_unchecked(buf);\n")?;

                continue;
            }
            ArgTyp::Typ { typ, flag } => (typ, flag.is_some()),
            ArgTyp::True { .. } => continue,
        };
        if optional {
            f.write_all(b"            if let Some(x) = &self.")?;
            write_escaped(f, &arg.name)?;
            f.write_all(b" { buf = x.serialize_unchecked(buf); }\n")?;
        } else {
            f.write_all(b"            buf = self.")?;
            write_escaped(f, &arg.name)?;
            f.write_all(b".serialize_unchecked(buf);\n")?;
        }
    }

    f.write_all(b"            buf\n        }")
}

fn write_enum_ser(f: &mut F, cfg: &Cfg, data: &Data, x: &Enum) -> Result<()> {
    f.write_all(b"unsafe {\n            match self {\n")?;

    for variant in &x.variants {
        let x = &data.types[*variant];

        f.write_all(b"                Self::")?;
        write_enum_variant(f, cfg, x)?;
        f.write_all(b"(x) => {\n                    buf = ")?;
        write_name(f, "types", &x.combinator.name)?;
        f.write_all(b"::CONSTRUCTOR_ID.serialize_unchecked(buf);\n                    x.serialize_unchecked(buf)\n                }\n")?;
    }

    f.write_all(b"            }\n        }")
}

pub(super) fn write_serialized_len(f: &mut F, cfg: &Cfg, data: &Data, x: X) -> Result<()> {
    f.write_all(b"\nimpl")?;
    match x {
        X::Func(x) => write_generics(f, cfg, &x.combinator.generic_args, false)?,
        _ => {}
    }
    f.write_all(b" crate::SerializedLen for ")?;
    f.write_all(x.name().actual.as_bytes())?;
    match x {
        X::Func(x) => write_generics(f, cfg, &x.combinator.generic_args, true)?,
        _ => {}
    }
    f.write_all(b" {\n    fn serialized_len(&self) -> usize {\n        ")?;
    match x {
        X::Type(x) => write_structure_len(f, cfg, data, false, &x.combinator)?,
        X::Func(x) => write_structure_len(f, cfg, data, true, &x.combinator)?,
        X::Enum(x) => write_enum_len(f, cfg, data, x)?,
    }
    f.write_all(b"\n    }\n}\n")
}

pub(super) fn write_serialize(f: &mut F, cfg: &Cfg, data: &Data, x: X) -> Result<()> {
    f.write_all(b"\nimpl")?;
    match x {
        X::Func(x) => write_generics(f, cfg, &x.combinator.generic_args, false)?,
        _ => {}
    }
    f.write_all(b" crate::ser::SerializeUnchecked for ")?;
    f.write_all(x.name().actual.as_bytes())?;
    match x {
        X::Func(x) => write_generics(f, cfg, &x.combinator.generic_args, true)?,
        _ => {}
    }
    f.write_all(
        b" {\n    unsafe fn serialize_unchecked(&self, mut buf: *mut u8) -> *mut u8 {\n        ",
    )?;
    match x {
        X::Type(x) => write_structure_ser(f, cfg, data, false, &x.combinator)?,
        X::Func(x) => write_structure_ser(f, cfg, data, true, &x.combinator)?,
        X::Enum(x) => write_enum_ser(f, cfg, data, x)?,
    }
    f.write_all(b"\n    }\n}\n")
}
