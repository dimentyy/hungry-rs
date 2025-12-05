use std::io::{Result, Write};

use crate::code::{write_escaped, write_typ};
use crate::meta::{Data, Func};
use crate::{Cfg, F};

pub(super) fn write_function(f: &mut F, cfg: &Cfg, data: &Data, x: &Func) -> Result<()> {
    f.write_all(b"\nimpl crate::Function for ")?;
    write_escaped(f, &x.combinator.name.actual)?;
    f.write_all(b" {\n    type Response = ")?;

    write_typ(f, cfg, data, &x.combinator.generic_args, &x.response, false)?;

    f.write_all(b";\n}\n")
}
