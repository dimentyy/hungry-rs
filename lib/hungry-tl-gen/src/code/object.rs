use std::io::{Result, Write};

use crate::code::{push_enum_variant, push_escaped};
use crate::config::Cfg;
use crate::meta::Data;

pub(super) fn write_object(cfg: &Cfg, data: &Data, s: &mut String) -> Result<()> {
    let mut f = cfg.root_file("object")?;

    s.push_str("#[repr(align(16))]\n#[allow(non_camel_case_types)]\n");
    s.push_str(&cfg.enum_derive);
    s.push_str("\npub enum Object {\n    Bool(bool),\n");

    let mut ns = None;

    for (i, split) in data.enums_split.windows(2).enumerate() {
        let real_schema = &cfg.schemas[i];

        let mut schema = "    ".to_owned();
        schema.push_str(&cfg.schemas[i]);
        schema.push('_');

        let mut newline = true;

        for x in &data.enums[split[0]..split[1]] {
            let new_ns = x.ident.space.as_deref();

            if new_ns != ns {
                ns = new_ns;
                newline = true;
            }

            if newline {
                s.push('\n');
            }

            s.push_str(&schema);

            if let Some(space) = ns {
                s.push_str(space);
                s.push('_');
            }

            push_escaped(s, &x.ident.actual);
            s.push('(');
            if x.object_box {
                s.push_str("Box<")
            }
            s.push_str(real_schema);
            s.push_str("::enums::");
            if let Some(space) = ns {
                s.push_str(space);
                s.push_str("::");
            }
            push_escaped(s, &x.ident.actual);
            if x.object_box {
                s.push('>');
            }
            s.push_str("),\n");

            newline = false;
        }
    }

    s.push_str("}\n\nimpl SerializedLen for Object {\n    #[inline(never)]\n    fn serialized_len(&self) -> usize {\n        match self {\n            Self::Bool(_) => 4,\n");

    for (i, split) in data.enums_split.windows(2).enumerate() {
        let mut schema = "            Self::".to_owned();
        schema.push_str(&cfg.schemas[i]);
        schema.push('_');

        let mut newline = true;

        for x in &data.enums[split[0]..split[1]] {
            let new_ns = x.ident.space.as_deref();

            if new_ns != ns {
                ns = new_ns;
                newline = true;
            }

            if newline {
                s.push('\n');
            }

            s.push_str(&schema);

            if let Some(space) = ns {
                s.push_str(space);
                s.push('_');
            }

            push_escaped(s, &x.ident.actual);
            s.push_str("(x) => x.serialized_len(),\n");

            newline = false;
        }
    }

    s.push_str("        }\n    }\n}\n\nimpl Object {\n    #[inline(never)]\n    pub fn deserialize(typ: u32, buf: &mut crate::de::Buf) -> Result<Self, crate::de::Error> {\n        use de::Deserialize;\n\n        Ok(match typ {\n            TRUE => Self::Bool(true),\n            FALSE => Self::Bool(false),\n\n");

    for (i, split) in data.enums_split.windows(2).enumerate() {
        let real_schema = &cfg.schemas[i];

        let schema = &cfg.schemas[i];

        for x_enum in &data.enums[split[0]..split[1]] {
            let ns = x_enum.ident.space.as_deref();

            for x in x_enum.variants.iter().map(|&i| &data.types[i]) {
                let push_type = |s: &mut String| {
                    s.push_str(schema);
                    s.push_str("::types::");

                    if let Some(space) = ns {
                        s.push_str(space);
                        s.push_str("::");
                    }

                    push_escaped(s, &x.combinator.ident.actual);
                };

                s.push_str("            ");

                push_type(s);

                s.push_str("::CONSTRUCTOR_ID => Self::");

                s.push_str(schema);
                s.push('_');

                if let Some(space) = ns {
                    s.push_str(space);
                    s.push('_');
                }

                push_escaped(s, &x_enum.ident.actual);

                s.push('(');

                if x_enum.object_box {
                    s.push_str("Box::new(")
                }

                s.push_str(real_schema);
                s.push_str("::enums::");
                if let Some(space) = ns {
                    s.push_str(space);
                    s.push_str("::");
                }
                push_escaped(s, &x_enum.ident.actual);
                s.push_str("::");

                push_enum_variant(cfg, s, x);
                s.push('(');

                if x.enum_box {
                    s.push_str("Box::new(")
                }

                push_type(s);

                s.push_str("::deserialize(buf)?");

                if x_enum.object_box || x.enum_box {
                    s.push(')');
                }

                s.push_str(")),\n");
            }
        }
    }

    s.push_str(
        "            _ => return Err(de::Error::unexpected_constructor()),\n        })\n    }\n}\n",
    );

    if cfg.derive_debug {
        s.push_str("\nimpl std::fmt::Debug for Object {\n    #[inline(never)]\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        match self {            Self::Bool(x) => x.fmt(f),\n");

        for (i, split) in data.enums_split.windows(2).enumerate() {
            let mut schema = "            Self::".to_owned();
            schema.push_str(&cfg.schemas[i]);
            schema.push('_');

            let mut newline = true;

            for x in &data.enums[split[0]..split[1]] {
                let new_ns = x.ident.space.as_deref();

                if new_ns != ns {
                    ns = new_ns;
                    newline = true;
                }

                if newline {
                    s.push('\n');
                }

                s.push_str(&schema);

                if let Some(space) = ns {
                    s.push_str(space);
                    s.push('_');
                }

                push_escaped(s, &x.ident.actual);
                s.push_str("(x) => x.fmt(f),\n");

                newline = false;
            }
        }

        s.push_str("        }\n    }\n}\n");
    }

    f.write_all(s.as_bytes())?;
    s.clear();
    f.flush()
}
