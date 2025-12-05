use std::io::{Result, Write};

use crate::code::{write_derive_macros, write_escaped, write_typ};
use crate::meta::{Data, Enum, Typ, Type};
use crate::{Cfg, F};

pub(super) fn write_enum_variant(f: &mut F, cfg: &Cfg, x: &Type) -> Result<()> {
    write_escaped(f, &x.combinator.name.actual)
}

pub(super) fn write_enum_body(f: &mut F, cfg: &Cfg, data: &Data, x: &Enum) -> Result<()> {
    write_derive_macros(f, cfg)?;
    f.write_all(b"pub enum ")?;
    write_escaped(f, &x.name.actual)?;
    f.write_all(b" {\n");

    for variant in &x.variants {
        let x = &data.types[*variant];

        let name = &x.combinator.name;

        f.write_all(b"    ")?;
        write_enum_variant(f, cfg, x)?;
        f.write_all(b"(")?;

        if x.recursive {
            f.write_all(b"Box<")?;
        }

        let typ = Typ::Type {
            index: *variant,
            params: Vec::new(),
        };
        write_typ(f, cfg, data, &[], &typ, false);

        if x.recursive {
            f.write_all(b">")?;
        }

        f.write_all(b"),\n")?;
    }

    f.write_all(b"}\n")
}
