use std::fmt;
use std::ops::{Deref, DerefMut};

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

/// Identifier of the boolean type constructor `true`:
///
/// ```tl
/// boolTrue#997275b5 = Bool;
/// ```
pub const TRUE: u32 = 0x997275b5;

/// Identifier of the boolean type constructor `false`:
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

/// Equivalent to the following TL constructor:
///
/// ```tl
/// int128 4*[ int ] = Int128;
/// ```
pub type Int128 = [u8; 16];

/// Equivalent to the following TL constructor:
///
/// ```tl
/// int256 8*[ int ] = Int256;
/// ```
pub type Int256 = [u8; 32];

/// Equivalent to the bare constructor `vector`:
///
/// ``` tl
/// vector {t:Type} # [ t ] = Vector t;
/// ```
#[derive(Clone, Eq, PartialEq)]
pub struct BareVec<T>(pub Vec<T>);

impl<T: fmt::Debug> fmt::Debug for BareVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Deref for BareVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for BareVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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

macro_rules! impl_const_serialized_len {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl ConstSerializedLen for $typ {
            const SERIALIZED_LEN: usize = $len;
        }
    )+ };
}

impl_const_serialized_len!(
    u32 => 4,
    i32 => 4,
    i64 => 8,
    f64 => 8,
    bool => 4,
    Int128 => 16,
    Int256 => 32,
);
