use crate::F;
use crate::code::{write_enum_variant, write_escaped, write_name};
use crate::config::Cfg;
use crate::meta::{Data, Type};
use std::io::{Result, Write};

pub(super) fn write_into_enum(f: &mut F, cfg: &Cfg, data: &Data, x: &Type) -> Result<()> {
    f.write_all(b"\nimpl crate::IntoEnum for ")?;
    write_escaped(f, &x.combinator.name.actual)?;
    f.write_all(b" {\n    type Enum = ")?;
    write_name(f, "enums", &data.enums[x.enum_index].name)?;
    f.write_all(b";\n\n    fn into_enum(self) -> Self::Enum {\n        ")?;
    write_name(f, "enums", &data.enums[x.enum_index].name)?;
    f.write_all(b"::")?;
    write_enum_variant(f, cfg, x)?;
    if x.recursive {
        f.write_all(b"(Box::new(self))\n    }\n}\n")
    } else {
        f.write_all(b"(self)\n    }\n}\n")
    }
}
