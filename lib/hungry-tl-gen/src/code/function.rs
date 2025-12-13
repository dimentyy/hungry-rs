use std::io::{Result, Write};

use crate::code::{write_escaped, write_generics, write_typ};
use crate::meta::{Data, Func, Typ};
use crate::{Cfg, F};

pub(super) fn write_function(f: &mut F, cfg: &Cfg, data: &Data, x: &Func) -> Result<()> {
    f.write_all(b"\nimpl")?;
    write_generics(f, cfg, &x.combinator.generic_args, false)?;
    f.write_all(b" crate::Function for ")?;
    write_escaped(f, &x.combinator.name.actual)?;
    write_generics(f, cfg, &x.combinator.generic_args, true)?;
    f.write_all(b" {\n    type Response = ")?;

    write_typ(f, cfg, data, &x.combinator.generic_args, &x.response, false)?;

    if matches!(x.response, Typ::Generic { .. }) {
        f.write_all(b"::Response")?;
    }

    f.write_all(b";\n}\n")
}
