use std::ops::Deref;
use std::path::PathBuf;
use std::{env, fs, io};

use crate::meta::Ident;

pub(crate) type F = io::BufWriter<fs::File>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub impl_debug: bool,
    pub derive_clone: bool,
    pub impl_into_enum: bool,
}

pub(crate) struct Cfg {
    pub(crate) config: Config,
    pub(crate) schemas: Vec<String>,
    pub(crate) current: usize,
    pub(crate) out_dir: PathBuf,
    pub(crate) derive: String,
}

impl Cfg {
    pub(crate) fn new(config: Config, schemas: Vec<String>) -> Self {
        let mut derives = Vec::new();

        if config.derive_clone {
            derives.push("Clone");
        }

        let mut iter = derives.iter();

        let mut derive = "".to_owned();

        if let Some(x) = iter.next() {
            derive.push_str("\n#[derive(");
            derive.push_str(x);

            for x in iter {
                derive.push_str(", ");
                derive.push_str(x);
            }

            derive.push_str(")]");
        };

        Self {
            config,
            schemas,
            current: usize::MAX,
            out_dir: PathBuf::new(),
            derive,
        }
    }

    pub(crate) fn switch(&mut self, schema: usize) {
        self.current = schema;
        self.out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        self.out_dir.push("hungry_tl");
        self.out_dir.push(&self.schemas[schema]);
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
    pub(crate) const ROOT: &'static str = "_root";

    pub(crate) fn mod_file(&self, module: &str) -> io::Result<F> {
        Ok(file!(self => module))
    }

    pub(crate) fn space_file(&self, module: &str, space: &str) -> io::Result<F> {
        Ok(file!(self, module => space))
    }

    pub(crate) fn item_file(&self, module: &str, ident: &Ident) -> io::Result<F> {
        let space = match ident.space {
            None => Self::ROOT,
            Some(ref space) => space,
        };

        Ok(file!(self, module, space => &ident.file))
    }
}
