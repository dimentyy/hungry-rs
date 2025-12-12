#![allow(unused)]

mod casing;
mod category;
mod code;
mod config;
mod macros;
mod meta;
mod read;
mod rust;

use std::fmt::Formatter;
use std::{fmt, fs, io};

pub(crate) use config::Cfg;

pub use chumsky;

pub use category::Category;
pub use config::Config;

type F = io::BufWriter<fs::File>;

pub fn generate(config: Config, schema: &str) {
    let config = Cfg::new(config);

    let parsed = read::parse(&config, schema).unwrap();

    let (data, temp) = meta::validate(&parsed).unwrap();

    code::generate(&config, &data, &temp).unwrap();
}
