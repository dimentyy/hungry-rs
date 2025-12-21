use crate::Cfg;
use crate::code::{push_enum_variant, push_escaped, push_function_generics, push_ident};
use crate::meta::{Arg, ArgTyp, Combinator, Data, Enum, Flag};

fn write_structure_arg_len(cfg: &Cfg, data: &Data, s: &mut String, x: &Arg) {
    match &x.typ {
        ArgTyp::Flags { .. } => s.push_str("4"),
        ArgTyp::Typ { flag, .. } => {
            if flag.is_some() {
                s.push_str("if let Some(x) = &");
            }
            s.push_str("self.");
            push_escaped(s, &x.ident);
            if flag.is_some() {
                s.push_str(" { x");
            }
            s.push_str(".serialized_len()");
            if flag.is_some() {
                s.push_str(" } else { 0 }");
            }
        }
        ArgTyp::True { .. } => {}
    }
}

pub(super) fn push_struct_ser_len(cfg: &Cfg, data: &Data, s: &mut String, x: &Combinator) {
    s.push_str("\nimpl");
    push_function_generics(s, &x.generic_args, true);
    s.push_str(" crate::SerializedLen for ");
    push_escaped(s, &x.ident.actual);
    push_function_generics(s, &x.generic_args, false);
    s.push_str(" {\n    fn serialized_len(&self) -> usize {\n        ");

    if x.args.is_empty() {
        return s.push_str("0");
    }

    let mut iter = x.args.iter();

    write_structure_arg_len(cfg, data, s, iter.next().unwrap());

    for arg in iter {
        if matches!(arg.typ, ArgTyp::True { .. }) {
            continue;
        }
        s.push_str("\n            + ");
        write_structure_arg_len(cfg, data, s, arg);
    }

    s.push_str("\n    }\n}\n");
}

pub(super) fn push_enum_ser_len(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    s.push_str("\nimpl crate::SerializedLen for ");
    push_escaped(s, &x.ident.actual);
    s.push_str(" {\n    fn serialized_len(&self) -> usize {\n        4 + match self {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("            Self::");
        push_enum_variant(cfg, s, x);
        s.push_str("(x) => x.serialized_len(),\n");
    }

    s.push_str("        }\n    }\n}\n");
}

fn write_flag_arg(cfg: &Cfg, s: &mut String, x: &Combinator, i: usize) {
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

    s.push_str(if bit > 0 { "(self." } else { "self." });
    push_escaped(s, &arg.ident);
    if opt {
        s.push_str(".is_some()");
    }
    s.push_str(" as u32");
    if bit > 0 {
        s.push_str(") << ");
        std::fmt::write(s, format_args!("{bit}")).unwrap();
    }
}

pub(super) fn push_struct_ser(cfg: &Cfg, data: &Data, s: &mut String, x: &Combinator) {
    s.push_str("\nimpl");
    push_function_generics(s, &x.generic_args, true);
    s.push_str(" crate::ser::SerializeUnchecked for ");
    push_escaped(s, &x.ident.actual);
    push_function_generics(s, &x.generic_args, false);
    s.push_str(" {\n    unsafe fn serialize_unchecked(&self, mut buf: std::ptr::NonNull<u8>) -> std::ptr::NonNull<u8> {\n        ");

    if x.args.is_empty() {
        s.push_str("buf\n    }\n}\n");
        return;
    }

    s.push_str("unsafe {\n");

    for arg in &x.args {
        let (typ, optional) = match &arg.typ {
            ArgTyp::Flags { args } => {
                s.push_str("            buf = ");
                if args.is_empty() {
                    s.push_str("0u32");
                } else {
                    s.push_str("(");
                    for arg in &args[..args.len() - 1] {
                        write_flag_arg(cfg, s, x, *arg);

                        s.push_str(" | ");
                    }

                    write_flag_arg(cfg, s, x, *args.last().unwrap());

                    s.push_str(")");
                }
                s.push_str(".serialize_unchecked(buf);\n");

                continue;
            }
            ArgTyp::Typ { typ, flag } => (typ, flag.is_some()),
            ArgTyp::True { .. } => continue,
        };
        if optional {
            s.push_str("            if let Some(x) = &self.");
            push_escaped(s, &arg.ident);
            s.push_str(" { buf = x.serialize_unchecked(buf); }\n");
        } else {
            s.push_str("            buf = self.");
            push_escaped(s, &arg.ident);
            s.push_str(".serialize_unchecked(buf);\n");
        }
    }

    s.push_str("            buf\n        }\n    }\n}\n");
}

pub(super) fn push_enum_ser(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    s.push_str("\nimpl crate::ser::SerializeUnchecked for ");
    push_escaped(s, &x.ident.actual);
    s.push_str(" {\n    unsafe fn serialize_unchecked(&self, mut buf: std::ptr::NonNull<u8>) -> std::ptr::NonNull<u8> {\n        unsafe {\n            match self {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("                Self::");
        push_enum_variant(cfg, s, x);
        s.push_str("(x) => {\n                    buf = ");
        push_ident(s, "types", &x.combinator.ident);
        s.push_str("::CONSTRUCTOR_ID.serialize_unchecked(buf);\n                    x.serialize_unchecked(buf)\n                }\n");
    }

    s.push_str("            }\n        }\n    }\n}\n");
}
