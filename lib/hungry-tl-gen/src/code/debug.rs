use crate::Cfg;
use crate::code::{push_enum_variant, push_escaped, push_function_generics, push_ident};
use crate::meta::{Arg, ArgTyp, Combinator, Data, Enum, Flag, Typ};

pub(super) fn push_struct_debug(cfg: &Cfg, data: &Data, s: &mut String, x: &Combinator) {
    s.push_str("\nimpl");

    push_function_generics(s, &x.generic_args, true);
    s.push_str(" std::fmt::Debug for ");
    push_escaped(s, &x.ident.actual);
    push_function_generics(s, &x.generic_args, false);
    s.push_str(" {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        f.debug_struct(\"");
    push_escaped(s, &x.ident.actual);
    s.push_str("\")\n");

    for (i, arg) in x.args.iter().enumerate() {
        let (typ, optional) = match &arg.typ {
            ArgTyp::Flags { args } => continue,
            ArgTyp::Typ { typ, flag } => (typ, flag.is_some()),
            ArgTyp::True { flag } => (&Typ::Bool, false),
        };

        s.push_str("            .field(\"");
        push_escaped(s, &arg.ident);
        s.push_str("\", ");

        if optional {
            match typ {
                Typ::Int128 | Typ::Int256 => {
                    s.push_str("&self.");
                    push_escaped(s, &arg.ident);
                    s.push_str(".as_ref().map(crate::hex::HexIntFmt))\n");
                }
                Typ::Bytes => {
                    s.push_str("&self.");
                    push_escaped(s, &arg.ident);
                    s.push_str(".as_ref().map(crate::hex::HexBytesFmt))\n");
                }
                _ => {
                    s.push_str("&self.");
                    push_escaped(s, &arg.ident);
                    s.push_str(")\n");
                }
            }
        } else {
            match typ {
                Typ::Int128 | Typ::Int256 => {
                    s.push_str("&crate::hex::HexIntFmt(&self.");
                    push_escaped(s, &arg.ident);
                    s.push_str("))\n");
                }
                Typ::Bytes => {
                    s.push_str("&crate::hex::HexBytesFmt(&self.");
                    push_escaped(s, &arg.ident);
                    s.push_str("))\n");
                }
                _ => {
                    s.push_str("&self.");
                    push_escaped(s, &arg.ident);
                    s.push_str(")\n");
                }
            }
        }
    }

    s.push_str("            .finish()\n    }\n}\n");
}

pub(super) fn push_enum_debug(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    s.push_str("\nimpl std::fmt::Debug for ");
    push_escaped(s, &x.ident.actual);
    s.push_str(" {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        match self {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("            Self::");
        push_enum_variant(cfg, s, x);
        s.push_str("(x) => x.fmt(f),\n");
    }

    s.push_str("        }\n    }\n}\n");
}
