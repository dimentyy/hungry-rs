use crate::ser::SerializeUnchecked;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::{fmt, mem};

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

/// # bytes
///
/// Basic bare type. It is an alias of the string type,
/// with the difference that the value may contain arbitrary
/// byte sequences, including invalid UTF-8 sequences.
///
/// When computing crc32 for a constructor or method it is
/// necessary to replace all byte types with string types.
///
/// ---
/// Represents the following built-in TL definition:
/// ```tl
/// double ? = Double;
/// ```
///
/// ---
/// https://core.telegram.org/type/bytes
pub type Bytes = Vec<u8>;

/// Equivalent to the bare constructor `vector`:
///
/// ``` tl
/// vector {t:Type} # [ t ] = Vector t;
/// ```
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BareVec<T>(pub Vec<T>);

impl<T: fmt::Debug> fmt::Debug for BareVec<T> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Deref for BareVec<T> {
    type Target = Vec<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for BareVec<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait IntoEnum {
    type Enum;

    fn into_enum(self) -> Self::Enum;
}

#[inline(always)]
pub fn boxed<T: IntoEnum>(variant: T) -> T::Enum {
    variant.into_enum()
}

pub trait Identifiable {
    const CONSTRUCTOR_ID: u32;
}

pub trait Function: Identifiable + ser::SerializeUnchecked + fmt::Debug {
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

pub fn de<X: de::Deserialize>(buf: &[u8]) -> Result<X, de::Error> {
    de::Buf::new(buf).de()
}

#[derive(Clone)]
pub struct CalculatedLen<'a, X: SerializedLen> {
    len: usize,
    obj: &'a X,
}

impl<'a, X: SerializedLen> CalculatedLen<'a, X> {
    pub fn new(obj: &'a X) -> Self {
        let len = obj.serialized_len();

        Self { len, obj }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
}

impl<'a, X: SerializedLen> Deref for CalculatedLen<'a, X> {
    type Target = X;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.obj
    }
}

#[repr(transparent)]
pub struct ConstructorId<X: SerializeUnchecked + Identifiable>(pub X);

impl<X: SerializeUnchecked + Identifiable> ConstructorId<X> {
    #[inline(always)]
    pub fn from_ref(obj: &'_ X) -> &'_ Self {
        unsafe { mem::transmute(obj) }
    }
}

impl<X: SerializeUnchecked + Identifiable> SerializedLen for ConstructorId<X> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        self.0.serialized_len() + 4
    }
}

impl<X: SerializeUnchecked + Identifiable> SerializeUnchecked for ConstructorId<X> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = X::CONSTRUCTOR_ID.serialize_unchecked(buf);
            self.0.serialize_unchecked(buf)
        }
    }
}
