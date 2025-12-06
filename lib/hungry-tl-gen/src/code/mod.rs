mod de;
mod enum_body;
mod function;
mod identifiable;
mod name;
mod ser;
mod serialized_len;
mod struct_body;
mod typ;
mod x;
mod debug;

use std::io::{Result, Write};

use indexmap::IndexMap;

use crate::meta::{Data, Enum, Func, Name, Type};
use crate::{Cfg, F};

use de::write_deserializable;
use enum_body::{write_enum_body, write_enum_variant};
use function::write_function;
use identifiable::write_identifiable;
use name::write_name;
use ser::write_serialize;
use serialized_len::write_serialized_len;
use struct_body::write_struct_body;
use typ::write_typ;
use x::X;
use debug::write_debug;

macro_rules! write_module {
    ( $cfg:expr, $module:literal: for $x:ident in $iter:expr => $name:expr; $func:expr; ) => {{
        let mut root = Vec::<&Name>::new();
        let mut mods = IndexMap::<&str, Vec<&Name>>::new();

        for $x in $iter {
            let name = $name;

            if let Some(ref space) = name.space {
                mods.entry(space).or_default().push(name);
            } else {
                root.push(name)
            }

            $func;
        }

        let f = &mut $cfg.mod_file($module)?;

        write_module(f, $cfg, $module, &root, &mods)?;

        f.flush()?;
    }};
}

pub(crate) fn generate(cfg: &Cfg, data: &Data) -> Result<()> {
    write_module!(
        cfg, "types": for x in &data.types => &x.combinator.name;
        write_type(cfg, data, x)?;
    );

    write_module!(
        cfg, "funcs": for x in &data.funcs => &x.combinator.name;
        write_func(cfg, data, x)?;
    );

    write_module!(
        cfg, "enums": for x in &data.enums => &x.name;
        write_enum(cfg, data, x)?;
    );

    let mut f = cfg.mod_file("mod")?;

    f.write_all(b"pub mod types;\npub mod funcs;\npub mod enums;\n")?;

    f.flush()
}

fn write_module(
    f: &mut F,
    cfg: &Cfg,
    module: &str,
    root: &Vec<&Name>,
    mods: &IndexMap<&str, Vec<&Name>>,
) -> Result<()> {
    if !root.is_empty() {
        f.write_all(b"mod ")?;
        f.write_all(Cfg::UNSPACED.as_bytes())?;
        f.write_all(b";\n\n")?;

        write_space(f, cfg, module, Cfg::UNSPACED, root)?;
    }

    if !mods.is_empty() {
        for (space, names) in mods {
            f.write_all(b"pub mod ")?;
            write_escaped(f, space)?;
            f.write_all(b";\n")?;

            write_space(f, cfg, module, space, names)?;
        }

        f.write_all(b"\n")?;
    }

    if !root.is_empty() {
        f.write_all(b"pub use ")?;
        f.write_all(Cfg::UNSPACED.as_bytes())?;
        f.write_all(b"::*;\n\n")?;
    }

    Ok(())
}

fn write_space(f: &mut F, cfg: &Cfg, module: &str, space: &str, names: &Vec<&Name>) -> Result<()> {
    let f = &mut cfg.space_file(module, space)?;

    write_mods(f, cfg, names)?;
    write_uses(f, cfg, names)?;

    f.flush()
}

fn write_mods(f: &mut F, cfg: &Cfg, names: &Vec<&Name>) -> Result<()> {
    for name in names {
        f.write_all(b"mod ")?;
        write_escaped(f, &name.file)?;
        f.write_all(b";\n")?;
    }
    f.write_all(b"\n")
}

fn write_uses(f: &mut F, cfg: &Cfg, names: &Vec<&Name>) -> Result<()> {
    for name in names {
        f.write_all(b"pub use ")?;
        write_escaped(f, &name.file)?;
        f.write_all(b"::")?;
        write_escaped(f, &name.actual)?;
        f.write_all(b";\n")?;
    }
    f.write_all(b"\n")
}

fn write_type(cfg: &Cfg, data: &Data, x: &Type) -> Result<()> {
    let f = &mut cfg.item_file("types", &x.combinator.name)?;

    write_imports(f, cfg)?;

    write_struct_body(f, cfg, data, &x.combinator)?;
    if cfg.impl_debug {
        write_debug(f, cfg, data, X::Type(x))?;
    }
    write_identifiable(f, cfg, &x.combinator)?;
    write_serialize(f, cfg, data, X::Type(x))?;
    if x.combinator.args.is_empty() {
        write_serialized_len(f, &x.combinator.name.actual, 0)?;
    }
    write_deserializable(f, cfg, data, X::Type(x))?;

    f.flush()
}

fn write_func(cfg: &Cfg, data: &Data, x: &Func) -> Result<()> {
    let f = &mut cfg.item_file("funcs", &x.combinator.name)?;

    write_imports(f, cfg)?;

    write_struct_body(f, cfg, data, &x.combinator)?;
    if cfg.impl_debug {
        write_debug(f, cfg, data, X::Func(x))?;
    }
    write_identifiable(f, cfg, &x.combinator)?;
    write_function(f, cfg, data, x)?;
    write_serialize(f, cfg, data, X::Func(x))?;

    f.flush()
}

fn write_enum(cfg: &Cfg, data: &Data, x: &Enum) -> Result<()> {
    let f = &mut cfg.item_file("enums", &x.name)?;

    write_imports(f, cfg)?;

    write_enum_body(f, cfg, data, x)?;
    if cfg.impl_debug {
        write_debug(f, cfg, data, X::Enum(x))?;
    }
    write_serialize(f, cfg, data, X::Enum(x))?;
    write_deserializable(f, cfg, data, X::Enum(x))?;

    f.flush()
}

fn write_imports(f: &mut F, cfg: &Cfg) -> Result<()> {
    f.write_all(b"use crate::{")?;
    f.write_all(cfg.schema_name.as_bytes())?;
    f.write_all(b"::{types as _types, enums as _enums}, Identifiable as _};\n")
}

fn write_derive_macros(f: &mut F, cfg: &Cfg) -> Result<()> {
    let mut iter = cfg.derive_macros.iter();

    f.write_all(b"\n")?;

    let Some(x) = iter.next() else { return Ok(()) };

    f.write_all(b"#[derive(")?;
    f.write_all(x.as_bytes())?;

    for x in iter {
        f.write_all(b", ")?;
        f.write_all(x.as_bytes())?;
    }

    f.write_all(b")]\n")
}

pub(crate) fn write_escaped(f: &mut F, s: &str) -> Result<()> {
    match s {
        "self" => f.write_all(b"is_")?,
        "loop" | "type" | "static" | "final" => f.write_all(b"r#")?,
        _ => {}
    }

    f.write_all(s.as_bytes())
}
