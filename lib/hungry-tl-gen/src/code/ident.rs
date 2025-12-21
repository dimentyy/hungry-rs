use crate::code::push_escaped;
use crate::meta::Ident;

pub(super) fn push_ident(s: &mut String, module: &str, ident: &Ident) {
    s.push_str("_");
    s.push_str(&module);
    s.push_str("::");

    if let Some(space) = &ident.space {
        push_escaped(s, space);
        s.push_str("::");
    }

    push_escaped(s, &ident.actual)
}
