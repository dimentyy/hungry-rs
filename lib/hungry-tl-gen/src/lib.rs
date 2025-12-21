#![allow(dead_code)]
#![deny(unsafe_code)]

mod category;
mod code;
mod config;
mod meta;

pub(crate) mod read;
pub(crate) mod rust;

pub(crate) use config::{Cfg, F};

pub use category::Category;
pub use config::Config;

pub fn generate(config: Config, names: Vec<String>, schemas: &[&str]) {
    let mut parsed = Vec::new();

    for schema in schemas {
        parsed.push(read::parse(schema).unwrap());
    }

    let data = meta::validate(&parsed);

    let mut cfg = Cfg::new(config, names);

    for i in 0..schemas.len() {
        cfg.switch(i);

        code::generate(&cfg, &data).unwrap();
    }
}
