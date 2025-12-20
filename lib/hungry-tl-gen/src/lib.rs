#![allow(dead_code)]
#![deny(unsafe_code)]

mod category;
mod config;
mod ident;
mod meta;

pub(crate) mod read;
pub(crate) mod rust;
mod code;

pub use category::Category;
pub use config::Config;
pub use ident::Ident;


