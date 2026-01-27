#![deny(
    unused_imports,
    clippy::undocumented_unsafe_blocks,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::pedantic,
    clippy::nursery
)]
#![allow(clippy::wildcard_imports, clippy::inline_always)]

mod bare_vec;
mod big_int;
mod bytes;

pub mod hex;

pub mod tl {
    use super::*;

    pub use bare_vec::BareVec;
    pub use big_int::{Int128, Int256};
    pub use bytes::Bytes;
}

#[macro_export]
macro_rules! infallible {
    ( $e:expr ) => (
        #[expect(clippy::missing_panics_doc)]
        $e
    );
    { $( $s:stmt );+ $( ; )? } => {
        $(
            #[expect(clippy::missing_panics_doc)]
            $s;
        )+
    };
}
