use crate::code::{push_escaped, push_imports};
use crate::config::Cfg;
use crate::meta::{Data, Enum};
use std::io::{Result, Write};

pub(super) fn write_object(cfg: &Cfg, data: &Data, enums: &[Enum], s: &mut String) -> Result<()> {
    let mut f = cfg.mod_file("object")?;

    push_imports(cfg, s);

    s.push_str(&cfg.derive);
    s.push_str("\npub enum Object {\n    Bool(bool),\n");

    s.push_str("}");

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}
