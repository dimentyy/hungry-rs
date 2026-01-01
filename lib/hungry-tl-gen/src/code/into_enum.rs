use crate::Cfg;
use crate::code::{push_enum_variant, push_escaped, push_ident};
use crate::meta::{Data, Type};

pub(super) fn push_into_enum(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) {
    s.push_str("\nimpl From<");
    push_escaped(s, &x.combinator.ident.actual);
    s.push_str("> for ");
    push_ident(s, "enums", &data.enums[x.enum_index].ident);
    s.push_str(" {\n    #[inline]\n    fn from(value: ");
    push_escaped(s, &x.combinator.ident.actual);
    s.push_str(") -> Self {\n        ");
    s.push_str("Self::");
    push_enum_variant(cfg, s, x);
    if x.recursive {
        s.push_str("(Box::new(value))\n    }\n}\n")
    } else {
        s.push_str("(value)\n    }\n}\n")
    }
}
