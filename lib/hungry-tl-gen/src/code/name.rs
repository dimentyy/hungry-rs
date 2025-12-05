use std::io::{Result, Write};

use crate::F;
use crate::code::write_escaped;
use crate::meta::Name;

pub(super) fn write_name(f: &mut F, module: &str, name: &Name) -> Result<()> {
    f.write_all(b"_")?;
    f.write_all(module.as_bytes())?;
    f.write_all(b"::")?;

    if let Some(space) = &name.space {
        write_escaped(f, space)?;
        f.write_all(b"::")?;
    }

    write_escaped(f, &name.actual)
}
