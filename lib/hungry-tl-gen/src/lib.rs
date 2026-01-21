#![allow(dead_code)]
#![forbid(unsafe_code)]

mod category;
mod code;
mod config;

pub(crate) mod rust;

pub mod meta;
pub mod read;

pub(crate) use config::Cfg;

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
