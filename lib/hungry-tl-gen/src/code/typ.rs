use crate::Cfg;
use crate::code::push_ident;
use crate::meta::{Data, GenericArg, Typ};

pub(super) fn push_typ(
    _cfg: &Cfg,
    data: &Data,
    s: &mut String,
    generic_args: &[GenericArg],
    typ: &Typ,
    turbofish: bool,
) {
    let end = match typ {
        Typ::Type { index } => {
            let x = &data.types[*index];

            return push_ident(s, "types", &x.combinator.ident);
        }
        Typ::Enum { index } => {
            let x = &data.enums[*index];

            return push_ident(s, "enums", &x.ident);
        }
        Typ::Int => "i32",
        Typ::Long => "i64",
        Typ::Double => "f64",
        Typ::Bytes => {
            if turbofish {
                "Vec::<u8>"
            } else {
                "Vec<u8>"
            }
        }
        Typ::String => "String",
        Typ::Bool => "bool",
        Typ::BareVector(typ) => {
            s.push_str(if turbofish {
                "crate::BareVec::<"
            } else {
                "crate::BareVec<"
            });
            push_typ(_cfg, data, s, generic_args, typ, false);
            ">"
        }
        Typ::Vector(typ) => {
            s.push_str(if turbofish { "Vec::<" } else { "Vec<" });
            push_typ(_cfg, data, s, generic_args, typ, false);
            ">"
        }
        Typ::Int128 => "crate::Int128",
        Typ::Int256 => "crate::Int256",
        Typ::Generic { index } => &generic_args[*index].ident,
    };

    s.push_str(end);
}
