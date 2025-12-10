use std::fmt;

mod hex;

pub mod de;
pub mod ser;

#[allow(unused_imports, unused_mut)]
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/hungry_tl/api/mod.rs"));
}

#[allow(unused_imports, unused_mut)]
pub mod mtproto {
    include!(concat!(env!("OUT_DIR"), "/hungry_tl/mtproto/mod.rs"));
}

const BOOL_TRUE: u32 = 0x997275b5;
const BOOL_FALSE: u32 = 0xbc799737;
const VECTOR: u32 = 0x1cb5c415;

pub type Int128 = [u8; 16];
pub type Int256 = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BareVec<T>(pub Vec<T>);

pub trait IntoEnum {
    type Enum;

    fn into_enum(self) -> Self::Enum;
}

#[inline]
pub fn boxed<T: IntoEnum>(variant: T) -> T::Enum {
    variant.into_enum()
}

pub trait Identifiable {
    const CONSTRUCTOR_ID: u32;
}

pub trait Function: Identifiable + ser::Serialize + fmt::Debug {
    type Response: de::Deserialize;
}

pub trait SerializedLen {
    const SERIALIZED_LEN: usize;
}

macro_rules! impl_serialized_len {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl SerializedLen for $typ {
            const SERIALIZED_LEN: usize = $len;
        }
    )+ };
}

impl_serialized_len!(
    u32 => 4,
    i32 => 4,
    i64 => 8,
    f64 => 8,
    bool => 4,
    Int128 => 16,
    Int256 => 32,
);
