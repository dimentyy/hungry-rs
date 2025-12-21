mod const_ser_len;
mod de;
mod debug;
mod enum_body;
mod function;
mod generic;
mod ident;
mod identifiable;
mod into_enum;
mod ser;
mod struct_body;
mod typ;

use std::io::{Result, Write};

use indexmap::IndexMap;

use crate::Cfg;
use crate::meta::{Data, Enum, Func, Ident, Temp, Type};

use crate::code::de::push_type_de;
use const_ser_len::push_const_ser_len;
use de::push_enum_de;
use debug::{push_enum_debug, push_struct_debug};
use enum_body::{push_enum_body, push_enum_variant};
use function::push_function;
use generic::push_function_generics;
use ident::push_ident;
use identifiable::push_identifiable;
use into_enum::push_into_enum;
use ser::{push_enum_ser, push_enum_ser_len, push_struct_ser, push_struct_ser_len};
use struct_body::push_struct_body;
use typ::push_typ;

macro_rules! write_module {
    ( $cfg:expr,  $s:expr , $module:literal : for $x:ident in $iter:expr => $ident:expr; $func:expr; ) => {{
        let mut root = Vec::<&Ident>::new();
        let mut mods = IndexMap::<&str, Vec<&Ident>>::new();

        for $x in $iter {
            let ident = $ident;

            if let Some(ref space) = ident.space {
                mods.entry(space).or_default().push(ident);
            } else {
                root.push(ident)
            }

            $func;
        }

        write_spaces($cfg, $s, $module, &root, &mods)?;

        push_module($cfg, $s, $module, &root, &mods)?;

        $cfg.mod_file($module)?
    }};
}

pub(crate) fn generate(cfg: &Cfg, data: &Data) -> Result<()> {
    let mut s = String::with_capacity(1024 * 1024);

    let types = &data.types[data.types_split[cfg.current]..data.types_split[cfg.current + 1]];

    let mut f = write_module!(
        cfg, &mut s, "types": for x in types => &x.combinator.ident;
        write_type(cfg, data, &mut s, x)?;
    );

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()?;

    let funcs = &data.funcs[data.funcs_split[cfg.current]..data.funcs_split[cfg.current + 1]];

    let mut f = write_module!(
        cfg, &mut s, "funcs": for x in funcs => &x.combinator.ident;
        write_func(cfg, data, &mut s, x)?;
    );

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()?;

    let enums = &data.enums[data.enums_split[cfg.current]..data.enums_split[cfg.current + 1]];

    let mut f = write_module!(
        cfg, &mut s, "enums": for x in enums => &x.ident;
        write_enum(cfg, data, &mut s, x)?;
    );

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()?;

    let mut f = cfg.mod_file("mod")?;

    s.push_str("pub mod types;\npub mod funcs;\npub mod enums;\n");

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}

fn write_spaces(
    cfg: &Cfg,
    s: &mut String,
    module: &str,
    root: &Vec<&Ident>,
    mods: &IndexMap<&str, Vec<&Ident>>,
) -> Result<()> {
    if !root.is_empty() {
        write_space(cfg, s, module, Cfg::ROOT, root)?;
    }

    if !mods.is_empty() {
        for (space, names) in mods {
            write_space(cfg, s, module, space, names)?;
        }
    }

    Ok(())
}

fn push_module(
    _cfg: &Cfg,
    s: &mut String,
    module: &str,
    root: &Vec<&Ident>,
    mods: &IndexMap<&str, Vec<&Ident>>,
) -> Result<()> {
    if !root.is_empty() {
        s.push_str("mod ");
        s.push_str(Cfg::ROOT);
        s.push_str(";\n\n");
    }

    if !mods.is_empty() {
        for space in mods.keys() {
            s.push_str("pub mod ");
            push_escaped(s, space);
            s.push_str(";\n");
        }
    }

    if !root.is_empty() {
        s.push_str("pub use ");
        s.push_str(Cfg::ROOT);
        s.push_str("::*;\n\n");
    }

    Ok(())
}

fn write_space(
    cfg: &Cfg,
    s: &mut String,
    module: &str,
    space: &str,
    idents: &Vec<&Ident>,
) -> Result<()> {
    let f = &mut cfg.space_file(module, space)?;

    for ident in idents {
        s.push_str("mod ");
        push_escaped(s, &ident.file);
        s.push_str(";\n");
    }

    s.push_str("\n");

    for ident in idents {
        s.push_str("pub use ");
        push_escaped(s, &ident.file);
        s.push_str("::");
        push_escaped(s, &ident.actual);
        s.push_str(";\n");
    }

    s.push_str("\n");

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}

fn write_type(cfg: &Cfg, data: &Data, s: &mut String, x: &Type) -> Result<()> {
    let f = &mut cfg.item_file("types", &x.combinator.ident)?;

    push_imports(cfg, s);

    push_struct_body(cfg, data, s, &x.combinator);
    if cfg.impl_debug {
        push_struct_debug(cfg, data, s, &x.combinator);
    }
    if cfg.impl_into_enum {
        push_into_enum(cfg, data, s, x);
    }
    push_identifiable(s, &x.combinator);
    if let Some(len) = x.combinator.de.const_len() {
        push_const_ser_len(s, &x.combinator.ident.actual, len);
    } else {
        push_struct_ser_len(cfg, data, s, &x.combinator);
    }
    push_struct_ser(cfg, data, s, &x.combinator);
    push_type_de(cfg, data, s, x);

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}

fn write_func(cfg: &Cfg, data: &Data, s: &mut String, x: &Func) -> Result<()> {
    let f = &mut cfg.item_file("funcs", &x.combinator.ident)?;

    push_imports(cfg, s);

    push_struct_body(cfg, data, s, &x.combinator);
    if cfg.impl_debug {
        push_struct_debug(cfg, data, s, &x.combinator);
    }
    push_identifiable(s, &x.combinator);
    push_function(cfg, data, s, x);
    if let Some(len) = x.combinator.de.const_len() {
        push_const_ser_len(s, &x.combinator.ident.actual, len);
    } else {
        push_struct_ser_len(cfg, data, s, &x.combinator);
    }
    push_struct_ser(cfg, data, s, &x.combinator);

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}

fn write_enum(cfg: &Cfg, data: &Data, s: &mut String, x: &Enum) -> Result<()> {
    let f = &mut cfg.item_file("enums", &x.ident)?;

    push_imports(cfg, s);

    push_enum_body(cfg, data, s, x);
    if cfg.impl_debug {
        push_enum_debug(cfg, data, s, x);
    }
    if let Some(len) = x.de.const_len() {
        push_const_ser_len(s, &x.ident.actual, len);
    } else {
        push_enum_ser_len(cfg, data, s, x);
    }
    push_enum_ser(cfg, data, s, x);
    push_enum_de(cfg, data, s, x);

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}

fn push_imports(cfg: &Cfg, s: &mut String) {
    s.push_str("use crate::{");
    s.push_str(&cfg.schemas[cfg.current]);
    s.push_str("::{types as _types, enums as _enums}, Identifiable as _, de::DeserializeUnchecked as _, de::DeserializeInfallible as _};\n");
}

pub(crate) fn push_escaped(s: &mut String, ident: &str) {
    match ident {
        "self" => s.push_str("is_"),
        "loop" | "type" | "static" | "final" => s.push_str("r#"),
        _ => {}
    }

    s.push_str(ident)
}
