use crate::code::push_escaped;

pub(super) fn push_const_ser_len(s: &mut String, name: &str, size: usize) {
    s.push_str("\nimpl crate::ConstSerializedLen for ");
    push_escaped(s, name);
    s.push_str(" {\n    const SERIALIZED_LEN: usize = ");
    std::fmt::write(s, format_args!("{size}")).unwrap();
    s.push_str(";\n}\n");
}
