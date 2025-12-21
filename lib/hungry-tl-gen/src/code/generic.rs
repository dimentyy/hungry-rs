use crate::meta::GenericArg;

pub(super) fn push_function_generics(s: &mut String, generic_args: &[GenericArg], parameterize: bool) {
    let mut iter = generic_args.iter();

    if let Some(arg) = iter.next() {
        s.push_str("<");

        s.push_str(&arg.ident);
        if parameterize {
            s.push_str(": crate::Function");
        }

        for arg in iter {
            s.push_str(", ");
            s.push_str(&arg.ident);
            if parameterize {
                s.push_str(": crate::Function");
            }
        }

        s.push_str(">");
    }
}
