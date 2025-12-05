use std::io::{Result, Write};

use crate::code::write_escaped;
use crate::meta::{Combinator, Name};
use crate::{Cfg, F};

pub(super) fn write_serialized_len(f: &mut F, name: &str, size: usize) -> Result<()> {
    f.write_all(b"\nimpl crate::SerializedLen for ")?;
    write_escaped(f, name)?;
    f.write_all(b" {\n    const SERIALIZED_LEN: usize = ")?;
    write!(f, "{size}")?;
    f.write_all(b";\n}\n")
}
