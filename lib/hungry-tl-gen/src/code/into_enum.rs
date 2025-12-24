use crate::Cfg;
use crate::code::{push_enum_variant, push_escaped, push_ident};
use crate::meta::{Data, Type};

pub(super) fn push_into_enum(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) {
    s.push_str("\nimpl crate::IntoEnum for ");
    push_escaped(s, &x.combinator.ident.actual);
    s.push_str(" {\n    type Enum = ");
    push_ident(s, "enums", &data.enums[x.enum_index].ident);
    s.push_str(";\n\n    fn into_enum(self) -> Self::Enum {\n        ");
    push_ident(s, "enums", &data.enums[x.enum_index].ident);
    s.push_str("::");
    push_enum_variant(cfg, s, x);
    if x.recursive {
        s.push_str("(Box::new(self))\n    }\n}\n")
    } else {
        s.push_str("(self)\n    }\n}\n")
    }
}
