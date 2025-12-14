use std::io::{Result, Write};

use crate::code::{X, write_escaped, write_name, write_typ};
use crate::meta::{ArgTyp, Combinator, Data, Enum, Typ, Type};
use crate::{Cfg, F};

fn write_empty(f: &mut F, name: &str) -> Result<()> {
    f.write_all(b"\nimpl crate::de::DeserializeInfallible for ")?;
    f.write_all(name.as_bytes())?;
    f.write_all(
        b" {\n    unsafe fn deserialize_infallible(_buf: *const u8) -> Self {\n        Self {}",
    )
}

fn write_pre_de(f: &mut F, name: &str) -> Result<()> {
    f.write_all(b"\nimpl crate::de::Deserialize for ")?;
    f.write_all(name.as_bytes())?;
    f.write_all(
        b" {\n    fn deserialize(buf: &mut crate::de::Buf) -> Result<Self, crate::de::Error> {\n",
    )
}

fn write_enum_de(f: &mut F, cfg: &Cfg, data: &Data, x: &Enum) -> Result<()> {
    write_pre_de(f, &x.name.actual)?;

    f.write_all(b"        match u32::deserialize(buf)? {\n")?;

    for variant in &x.variants {
        let x = &data.types[*variant];

        f.write_all(b"            ")?;
        write_name(f, "types", &x.combinator.name)?;
        f.write_all(b"::CONSTRUCTOR_ID => Ok(Self::")?;
        f.write_all(x.combinator.name.actual.as_bytes())?;
        f.write_all(if x.recursive { b"(Box::new(" } else { b"(" })?;
        write_typ(
            f,
            cfg,
            data,
            &[],
            &Typ::Type {
                index: *variant,
                params: Vec::new(),
            },
            true,
        )?;
        f.write_all(if x.recursive {
            b"::deserialize(buf)?))),\n"
        } else {
            b"::deserialize(buf)?)),\n"
        })?;
    }

    f.write_all(b"            _ => Err(crate::de::Error::UnexpectedConstructor),\n        }")
}

fn write_struct_finish(f: &mut F, cfg: &Cfg, x: &Combinator, ok: bool) -> Result<()> {
    f.write_all(if ok {
        b"\n        Ok(Self {\n"
    } else {
        b"\n        Self {\n"
    })?;

    for arg in &x.args {
        match &arg.typ {
            ArgTyp::Flags { .. } => continue,
            ArgTyp::Typ { .. } => {}
            ArgTyp::True { .. } => {}
        }

        f.write_all(b"            ")?;
        write_escaped(f, &arg.name)?;
        f.write_all(b",\n")?;
    }

    f.write_all(if ok { b"        })" } else { b"        }" })
}

fn write_type_de(f: &mut F, cfg: &Cfg, data: &Data, x: &Type) -> Result<()> {
    if x.combinator.args.is_empty() {
        return write_empty(f, &x.combinator.name.actual);
    }

    write_pre_de(f, &x.combinator.name.actual)?;

    for arg in &x.combinator.args {
        f.write_all(b"        let ")?;
        if matches!(&arg.typ, ArgTyp::Flags { args } if args.is_empty()) {
            f.write_all(b"_")?;
        }
        write_escaped(f, &arg.name)?;
        f.write_all(b" = ")?;

        match &arg.typ {
            ArgTyp::Flags { .. } => {
                f.write_all(b"u32::deserialize(buf)?;\n")?;
            }
            ArgTyp::Typ { typ, flag } => {
                if let Some(flag) = flag {
                    f.write_all(b"if ")?;
                    let arg = &x.combinator.args[flag.arg];
                    write_escaped(f, &arg.name)?;
                    f.write_all(b" & (1 << ")?;
                    write!(f, "{}", flag.bit)?;
                    f.write_all(b") != 0 { Some(")?;
                    write_typ(f, cfg, data, &x.combinator.generic_args, typ, true)?;
                    f.write_all(b"::deserialize(buf)?) } else { None };\n")?;
                } else {
                    write_typ(f, cfg, data, &x.combinator.generic_args, typ, true)?;
                    f.write_all(b"::deserialize(buf)?;\n")?;
                }
            }
            ArgTyp::True { flag } => {
                let flag_arg = &x.combinator.args[flag.arg];
                dbg!(&x);
                assert!(matches!(&flag_arg.typ, ArgTyp::Flags { .. }));
                f.write_all(flag_arg.name.as_bytes())?;
                f.write_all(b" & (1 << ")?;
                write!(f, "{}", flag.bit)?;
                f.write_all(b") != 0;\n")?;
            }
        };
    }

    write_struct_finish(f, cfg, &x.combinator, true)
}

pub(super) fn write_deserializable(f: &mut F, cfg: &Cfg, data: &Data, x: X) -> Result<()> {
    match x {
        X::Type(x) => write_type_de(f, cfg, data, x)?,
        X::Func(x) => unimplemented!(),
        X::Enum(x) => write_enum_de(f, cfg, data, x)?,
    };

    f.write_all(b"\n    }\n}\n")
}
