use std::io::{Result, Write};

use crate::meta::{Combinator, Name};
use crate::{Cfg, F, read};

pub(super) fn write_name_for_id<
    'a,
    I: Iterator<Item = (&'a Combinator, &'a read::Combinator<'a>)>,
>(
    f: &mut F,
    combinators: I,
) -> Result<()> {
    f.write_all(b"\npub fn name(id: u32) -> Option<&'static str> {\n    Some(match id {\n")?;
    for (meta, read) in combinators {
        write!(
            f,
            "        {:#010x} => \"",
            meta.explicit_id.unwrap_or(meta.inferred_id)
        )?;
        if let Some(space) = read.ident.space {
            f.write_all(space.as_bytes())?;
            f.write_all(b".")?;
        }
        f.write_all(read.ident.name.as_bytes())?;
        f.write_all(b"\",\n")?;
    }
    f.write_all(b"        _ => return None,\n    })\n}\n")
}
