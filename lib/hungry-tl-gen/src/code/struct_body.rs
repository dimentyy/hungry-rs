use crate::Cfg;
use crate::code::{push_escaped, push_typ};
use crate::meta::{Arg, ArgTyp, Combinator, Data, GenericArg, Typ};

fn write_arg(cfg: &Cfg, data: &Data, s: &mut String, generic_args: &[GenericArg], arg: &Arg) {
    let (typ, optional) = match &arg.typ {
        ArgTyp::Flags { .. } => return,
        ArgTyp::Typ { typ, flag } => (typ, flag.is_some()),
        ArgTyp::True { .. } => (&Typ::Bool, false),
    };

    s.push_str("    pub ");
    push_escaped(s, &arg.ident);
    s.push_str(": ");
    if optional {
        s.push_str("Option<");
    }
    push_typ(cfg, data, s, generic_args, typ, false);
    if optional {
        s.push_str(">");
    }
    s.push_str(",\n")
}

pub(super) fn push_struct_body(cfg: &Cfg, data: &Data, s: &mut String, x: &Combinator) {
    s.push_str("\n/// ```tl\n/// ");
    std::fmt::write(s, format_args!("{}", x.parsed)).unwrap();
    s.push_str("\n/// ```");
    s.push_str(&cfg.derive);
    s.push_str("\npub struct ");
    push_escaped(s, &x.ident.actual);

    let mut iter = x.generic_args.iter();

    if let Some(arg) = iter.next() {
        s.push_str("<");
        s.push_str(&arg.ident);
        s.push_str(": crate::Function");

        for arg in iter {
            s.push_str(", ");
            s.push_str(&arg.ident);
            s.push_str(": crate::Function");
        }

        s.push_str(">");
    }

    if x.args.is_empty() {
        return s.push_str(" {}\n");
    };

    s.push_str(" {\n");

    for arg in &x.args {
        write_arg(cfg, data, s, &x.generic_args, arg);
    }

    s.push_str("}\n");
}
