use std::io::{Result, Write};

use crate::code::{X, write_escaped, write_generics};
use crate::meta::Combinator;
use crate::{Cfg, F};

pub(super) fn write_identifiable(f: &mut F, cfg: &Cfg, x: &Combinator) -> Result<()> {
    f.write_all(b"\nimpl")?;
    write_generics(f, cfg, &x.generic_args, false)?;
    f.write_all(b" crate::Identifiable for ")?;
    write_escaped(f, &x.name.actual)?;
    write_generics(f, cfg, &x.generic_args, true)?;
    f.write_all(b" {\n    const CONSTRUCTOR_ID: u32 = 0x")?;

    write!(f, "{:08x}", x.explicit_id.unwrap_or(x.inferred_id))?;

    f.write_all(b";\n}\n")
}
