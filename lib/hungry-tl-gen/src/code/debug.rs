use crate::Cfg;
use crate::code::{push_enum_variant, push_escaped};
use crate::meta::{Data, Enum};

pub(super) fn push_enum_debug(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) {
    s.push_str("\nimpl std::fmt::Debug for ");
    push_escaped(s, &x.ident.actual);
    s.push_str(" {\n    ");
    if x.variants.len() == 1 {
        s.push_str("#[inline]\n    ")
    }
    s.push_str("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        match self {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        s.push_str("            Self::");
        push_enum_variant(cfg, s, x);
        s.push_str("(x) => x.fmt(f),\n");
    }

    s.push_str("        }\n    }\n}\n");
}
