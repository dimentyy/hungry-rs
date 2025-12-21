use crate::Cfg;
use crate::code::{push_escaped, push_function_generics, push_typ};
use crate::meta::{Data, Func, Typ};

pub(super) fn push_function(cfg: &Cfg, data: &Data, s: &mut String, x: &Func) {
    s.push_str("\nimpl");
    push_function_generics(s, &x.combinator.generic_args, true);
    s.push_str(" crate::Function for ");
    push_escaped(s, &x.combinator.ident.actual);
    push_function_generics(s, &x.combinator.generic_args, false);
    s.push_str(" {\n    type Response = ");

    push_typ(cfg, data, s, &x.combinator.generic_args, &x.response, false);

    if matches!(x.response, Typ::Generic { .. }) {
        s.push_str("::Response");
    }

    s.push_str(";\n}\n");
}
