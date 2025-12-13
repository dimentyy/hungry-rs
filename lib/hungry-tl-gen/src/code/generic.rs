use std::io::{Result, Write};

use crate::meta::{Data, GenericArg};
use crate::{Cfg, F};

pub(super) fn write_generics(
    f: &mut F,
    _cfg: &Cfg,
    generic_args: &[GenericArg],
    min: bool,
) -> Result<()> {
    let mut iter = generic_args.iter();

    if let Some(arg) = iter.next() {
        f.write_all(b"<")?;

        f.write_all(arg.name.as_bytes())?;
        if !min {
            f.write_all(b": crate::Function")?;
        }

        for arg in iter {
            f.write_all(b", ")?;
            f.write_all(arg.name.as_bytes())?;
            if !min {
                f.write_all(b": crate::Function")?;
            }
        }

        f.write_all(b">")?;
    }

    Ok(())
}
