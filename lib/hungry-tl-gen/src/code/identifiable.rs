use crate::code::{push_escaped, push_function_generics};
use crate::meta::Combinator;

pub(super) fn push_identifiable(s: &mut String, x: &Combinator) {
    s.push_str("\nimpl");
    push_function_generics(s, &x.generic_args, true);
    s.push_str(" crate::Identifiable for ");
    push_escaped(s, &x.ident.actual);
    push_function_generics(s, &x.generic_args, false);
    s.push_str(" {\n    const CONSTRUCTOR_ID: u32 = 0x");

    let id = x.explicit_id.unwrap_or(x.inferred_id);
    std::fmt::write(s, format_args!("{id:08x}")).unwrap();

    s.push_str(";\n}\n");
}
