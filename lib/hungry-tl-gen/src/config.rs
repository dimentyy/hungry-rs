use crate::F;
use crate::meta::Name;
use std::io::{Seek, SeekFrom, Write};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs, io};

#[derive(Clone, Debug)]
pub struct Config {
    pub schema_name: String,
    pub impl_debug: bool,
    pub derive_clone: bool,
    pub impl_into_enum: bool,
}

pub(crate) struct Cfg {
    pub(crate) config: Config,
    pub(crate) out_dir: PathBuf,
    pub(crate) derive_macros: Vec<&'static str>,
}

impl Cfg {
    pub(crate) fn new(config: Config) -> Self {
        let mut out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        out_dir.push("hungry_tl");
        out_dir.push(&config.schema_name);

        let mut derive_macros = Vec::new();

        if config.derive_clone {
            derive_macros.push("Clone");
        }

        Self {
            config,
            out_dir,
            derive_macros,
        }
    }
}

impl Deref for Cfg {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

macro_rules! file {
    ($self:expr $( ,$path:expr )* $(,)? => $file:expr) => {{
        let mut path = $self.out_dir.clone();

        $(
            path.push($path);
        )*

        if !path.try_exists()? {
            fs::create_dir_all(&path)?;
        }

        path.push($file);
        path.set_extension("rs");

        io::BufWriter::new(fs::File::create(path)?)
    }};
}

impl Cfg {
    pub(crate) const UNSPACED: &'static str = "_unspaced";

    pub(crate) fn mod_file(&self, module: &str) -> io::Result<F> {
        Ok(file!(self => module))
    }

    pub(crate) fn space_file(&self, module: &str, space: &str) -> io::Result<F> {
        Ok(file!(self, module => space))
    }

    pub(crate) fn item_file(&self, module: &str, name: &Name) -> io::Result<F> {
        let space = match name.space {
            None => Self::UNSPACED,
            Some(ref space) => space,
        };

        Ok(file!(self, module, space => &name.file))
    }
}
