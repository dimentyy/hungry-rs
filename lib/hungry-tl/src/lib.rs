extern crate core;

mod flags;

pub mod de;
pub mod ser;

pub use hungry_common as common;

pub use common::tl::*;

#[allow(unused_imports, unused_mut, clippy::all)]
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/hungry_tl/api/mod.rs"));
}

#[allow(unused_imports, unused_mut, clippy::all)]
pub mod mtproto {
    include!(concat!(env!("OUT_DIR"), "/hungry_tl/mtproto/mod.rs"));
}

/// Identifier of the `Bool` type constructor `boolTrue`:
///
/// ```tl
/// boolTrue#997275b5 = Bool;
/// ```
pub const TRUE: u32 = 0x997275b5;

/// Identifier of the `Bool` type constructor `boolFalse`:
///
/// ```tl
/// boolFalse#bc799737 = Bool;
/// ```
pub const FALSE: u32 = 0xbc799737;

/// Identifier of the `vector` constructor:
///
/// ```tl
/// vector#1cb5c415 {t:Type} # [ t ] = Vector t;
/// ```
pub const VECTOR: u32 = 0x1cb5c415;

pub trait Identifiable {
    const CONSTRUCTOR_ID: u32;
}

pub trait Function: Identifiable + ser::SerializeUnchecked {
    type Response: de::Deserialize;
}

pub trait ConstSerializedLen {
    /// The constant number of bytes required to serialize any instance.
    const SERIALIZED_LEN: usize;
}

pub trait SerializedLen {
    /// Returns the exact number of bytes required to serialize the instance.
    fn serialized_len(&self) -> usize;
}

impl<T: ConstSerializedLen> SerializedLen for T {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        Self::SERIALIZED_LEN
    }
}

macro_rules! const_serialized_len {
    ( $( $typ:ty ),+ $( , )? ) => { $(
        impl ConstSerializedLen for $typ {
            const SERIALIZED_LEN: usize = size_of::<Self>();
        }
    )+ };
}

const_serialized_len!(u32, i32, i64, f64, Int128, Int256);

impl ConstSerializedLen for bool {
    const SERIALIZED_LEN: usize = u32::SERIALIZED_LEN;
}
