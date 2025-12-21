use std::io::{Result, Write};

use crate::Cfg;
use crate::code::{push_escaped, push_ident, push_typ};
use crate::meta::{ArgTyp, Combinator, Data, Deserialization, Enum, Typ, Type};

fn push_empty(s: &mut String, name: &str) {
    s.push_str("\nimpl crate::de::DeserializeInfallible for ");
    s.push_str(&name);
    s.push_str(" {\n    unsafe fn deserialize_infallible(_buf: std::ptr::NonNull<u8>) -> Self {\n        Self {}\n    }\n}\n", )
}

fn push_checked_de(s: &mut String, name: &str) {
    s.push_str("\nimpl crate::de::Deserialize for ");
    s.push_str(&name);
    s.push_str(
        " {\n    fn deserialize(buf: &mut crate::de::Buf) -> Result<Self, crate::de::Error> {\n        ",
    )
}

fn push_unchecked_de(s: &mut String, name: &str) {
    s.push_str("\nimpl crate::de::DeserializeUnchecked for ");
    s.push_str(&name);
    s.push_str(
        " {\n    unsafe fn deserialize_unchecked(buf: std::ptr::NonNull<u8>) -> Result<Self, crate::de::UnexpectedConstructorError> {\n        unsafe {\n",
    )
}

fn push_infallible_de(s: &mut String, name: &str) {
    s.push_str("\nimpl crate::de::DeserializeInfallible for ");
    s.push_str(&name);
    s.push_str(
        " {\n    unsafe fn deserialize_infallible(buf: std::ptr::NonNull<u8>) -> Self {\n        unsafe {\n",
    )
}

fn push_enum_checked_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    push_checked_de(s, &x.ident.actual);

    s.push_str("match u32::deserialize(buf)? {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("            ");
        push_ident(s, "types", &x.combinator.ident);
        s.push_str("::CONSTRUCTOR_ID => Ok(Self::");
        s.push_str(&x.combinator.ident.actual);
        s.push_str(if x.recursive { "(Box::new(" } else { "(" });
        push_typ(cfg, data, s, &[], &Typ::Type { index: *variant }, true);
        s.push_str(if x.recursive {
            "::deserialize(buf)?))),\n"
        } else {
            "::deserialize(buf)?)),\n"
        });
    }

    s.push_str("            _ => Err(crate::de::Error::unexpected_constructor()),\n        }");
    s.push_str("\n    }\n}\n");
}

fn push_enum_unchecked_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    push_unchecked_de(s, &x.ident.actual);

    s.push_str("            match u32::deserialize_infallible(buf) {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("                ");
        push_ident(s, "types", &x.combinator.ident);
        s.push_str("::CONSTRUCTOR_ID => Ok(Self::");
        s.push_str(&x.combinator.ident.actual);
        s.push_str("(");
        push_typ(cfg, data, s, &[], &Typ::Type { index: *variant }, true);
        s.push_str("::deserialize_");
        s.push_str(
            if matches!(x.combinator.de, Deserialization::Infallible(_)) {
                "infallible(buf.add(4))"
            } else {
                "unchecked(buf.add(4))?"
            },
        );
        s.push_str(")),\n");
    }

    s.push_str("                _ => Err(crate::de::UnexpectedConstructorError {}),\n            }\n        }\n    }\n}\n");
}

pub(super) fn push_enum_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    if matches!(x.de, Deserialization::Unchecked(_)) {
        push_enum_unchecked_de(cfg, data, s, x);
    } else {
        push_enum_checked_de(cfg, data, s, x);
    }
}

fn push_struct_finish(x: &Combinator, s: &mut String, ok: bool, indent: &str) {
    s.push_str(indent);
    s.push_str(if ok { "Ok(Self {" } else { "Self {" });

    for arg in &x.args {
        match &arg.typ {
            ArgTyp::Flags { .. } => continue,
            ArgTyp::Typ { .. } => {}
            ArgTyp::True { .. } => {}
        }
        s.push_str(indent);
        s.push_str("    ");
        push_escaped(s, &arg.ident);
        s.push_str(",");
    }
    s.push_str(indent);
    s.push_str(if ok { "})" } else { "}" });
}

fn push_type_checked_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) {
    push_checked_de(s, &x.combinator.ident.actual);

    for arg in &x.combinator.args {
        s.push_str("        let ");
        if matches!(&arg.typ, ArgTyp::Flags { args } if args.is_empty()) {
            s.push_str("_");
        }
        push_escaped(s, &arg.ident);
        s.push_str(" = ");

        match &arg.typ {
            ArgTyp::Flags { .. } => {
                s.push_str("u32::deserialize(buf)?;\n");
            }
            ArgTyp::Typ { typ, flag } => {
                if let Some(flag) = flag {
                    s.push_str("if ");
                    let arg = &x.combinator.args[flag.arg];
                    push_escaped(s, &arg.ident);
                    s.push_str(" & (1 << ");
                    std::fmt::write(s, format_args!("{}", flag.bit)).unwrap();
                    s.push_str(") != 0 { Some(");
                    push_typ(cfg, data, s, &x.combinator.generic_args, typ, true);
                    s.push_str("::deserialize(buf)?) } else { None };\n");
                } else {
                    push_typ(cfg, data, s, &x.combinator.generic_args, typ, true);
                    s.push_str("::deserialize(buf)?;\n");
                }
            }
            ArgTyp::True { flag } => {
                let flag_arg = &x.combinator.args[flag.arg];
                assert!(matches!(&flag_arg.typ, ArgTyp::Flags { .. }));
                s.push_str(&flag_arg.ident);
                s.push_str(" & (1 << ");
                std::fmt::write(s, format_args!("{}", flag.bit)).unwrap();
                s.push_str(") != 0;\n");
            }
        };
    }

    push_struct_finish(&x.combinator, s, true, "\n        ");
    s.push_str("\n    }\n}\n");
}

fn push_type_unchecked_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) {
    push_unchecked_de(s, &x.combinator.ident.actual);

    let mut offset = 0;

    for arg in &x.combinator.args {
        s.push_str("            let ");
        if matches!(&arg.typ, ArgTyp::Flags { args } if args.is_empty()) {
            s.push_str("_");
        }
        push_escaped(s, &arg.ident);
        s.push_str(" = ");

        match &arg.typ {
            ArgTyp::Flags { .. } => {
                offset += 4;
                s.push_str("u32::deserialize_infallible(buf);\n");
            }
            ArgTyp::Typ { typ, flag } => {
                let de = dbg!(typ).ready_de(data);
                let func = if matches!(de, Deserialization::Infallible(_)) {
                    "_infallible"
                } else {
                    "_unchecked"
                };

                let fin = if matches!(de, Deserialization::Infallible(_)) {
                    ")"
                } else {
                    ")?"
                };

                if let Some(flag) = flag {
                    s.push_str("if ");
                    let arg = &x.combinator.args[flag.arg];
                    push_escaped(s, &arg.ident);
                    s.push_str(" & (1 << ");
                    std::fmt::write(s, format_args!("{}", flag.bit)).unwrap();
                    s.push_str(") != 0 { Some(");
                    push_typ(cfg, data, s, &x.combinator.generic_args, typ, true);
                    s.push_str("::deserialize");
                    s.push_str(func);
                    s.push_str("(buf");
                    if offset > 0 {
                        s.push_str(".add(");
                        std::fmt::write(s, format_args!("{offset}")).unwrap();
                        s.push_str(")");
                    }
                    s.push_str(fin);
                    s.push_str(") } else { None };\n");
                } else {
                    push_typ(cfg, data, s, &x.combinator.generic_args, typ, true);
                    s.push_str("::deserialize");
                    s.push_str(func);
                    s.push_str("(buf");
                    if offset > 0 {
                        s.push_str(".add(");
                        std::fmt::write(s, format_args!("{offset}")).unwrap();
                        s.push_str(")");
                    }
                    s.push_str(fin);
                    s.push_str(";\n");
                }

                offset += de.const_len().unwrap();
            }
            ArgTyp::True { flag } => {
                let flag_arg = &x.combinator.args[flag.arg];
                assert!(matches!(&flag_arg.typ, ArgTyp::Flags { .. }));
                s.push_str(&flag_arg.ident);
                s.push_str(" & (1 << ");
                std::fmt::write(s, format_args!("{}", flag.bit)).unwrap();
                s.push_str(") != 0;\n");
            }
        };
    }

    push_struct_finish(&x.combinator, s, true, "\n            ");
    s.push_str("\n        }\n    }\n}\n");
}

fn push_type_infallible_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) {
    push_infallible_de(s, &x.combinator.ident.actual);

    let mut offset = 0;

    for arg in &x.combinator.args {
        s.push_str("            let ");
        if matches!(&arg.typ, ArgTyp::Flags { args } if args.is_empty()) {
            s.push_str("_");
        }
        push_escaped(s, &arg.ident);
        s.push_str(" = ");

        match &arg.typ {
            ArgTyp::Flags { .. } => {
                offset += 4;
                s.push_str("u32::deserialize_infallible(buf);\n");
            }
            ArgTyp::Typ { typ, flag } => {
                let de = typ.ready_de(data);

                if let Some(flag) = flag {
                    s.push_str("if ");
                    let arg = &x.combinator.args[flag.arg];
                    push_escaped(s, &arg.ident);
                    s.push_str(" & (1 << ");
                    std::fmt::write(s, format_args!("{}", flag.bit)).unwrap();
                    s.push_str(") != 0 { Some(");
                    push_typ(cfg, data, s, &x.combinator.generic_args, typ, true);
                    s.push_str("::deserialize_infallible(buf");
                    if offset > 0 {
                        s.push_str(".add(");
                        std::fmt::write(s, format_args!("{offset}")).unwrap();
                        s.push_str(")");
                    }
                    s.push_str(")) } else { None };\n");
                } else {
                    push_typ(cfg, data, s, &x.combinator.generic_args, typ, true);
                    s.push_str("::deserialize_infallible(buf");
                    if offset > 0 {
                        s.push_str(".add(");
                        std::fmt::write(s, format_args!("{offset}")).unwrap();
                        s.push_str(")");
                    }
                    s.push_str(");\n");
                }

                offset += de.const_len().unwrap();
            }
            ArgTyp::True { flag } => {
                let flag_arg = &x.combinator.args[flag.arg];
                assert!(matches!(&flag_arg.typ, ArgTyp::Flags { .. }));
                s.push_str(&flag_arg.ident);
                s.push_str(" & (1 << ");
                std::fmt::write(s, format_args!("{}", flag.bit)).unwrap();
                s.push_str(") != 0;\n");
            }
        };
    }

    push_struct_finish(&x.combinator, s, false, "\n            ");
    s.push_str("\n        }\n    }\n}\n");
}

pub(super) fn push_type_de(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) {
    if x.combinator.args.is_empty() {
        push_empty(s, &x.combinator.ident.actual);

        return;
    }

    match &x.combinator.de {
        Deserialization::Infallible(_) => push_type_infallible_de(cfg, data, s, x),
        Deserialization::Unchecked(_) => push_type_unchecked_de(cfg, data, s, dbg!(x)),
        Deserialization::Checked => push_type_checked_de(cfg, data, s, x),
    }
}
