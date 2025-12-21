use crate::Cfg;
use crate::code::{push_escaped, push_typ};
use crate::meta::{Data, Enum, Typ, Type};

pub(super) fn push_enum_variant(cfg: &Cfg, s: &mut String, x: &Type) {
    push_escaped(s, &x.combinator.ident.actual);
}

pub(super) fn push_enum_body(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    s.push_str(&cfg.derive);
    s.push_str("\npub enum ");
    push_escaped(s, &x.ident.actual);
    s.push_str(" {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("    ");
        push_enum_variant(cfg, s, x);
        s.push_str("(");

        if x.recursive {
            s.push_str("Box<");
        }

        let typ = Typ::Type { index: *variant };
        push_typ(cfg, data, s, &[], &typ, false);

        if x.recursive {
            s.push_str(">");
        }

        s.push_str("),\n");
    }

    s.push_str("}\n");
}
