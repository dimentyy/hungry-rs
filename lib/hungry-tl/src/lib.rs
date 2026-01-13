extern crate core;

mod flags;

pub mod de;
pub mod ser;

pub use hungry_common as common;
use std::borrow::Borrow;
use std::ptr;
use std::ptr::NonNull;

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

#[repr(transparent)]
pub struct ConstructorId<X: Function>(pub X);

impl<X: Function> ConstructorId<X> {
    #[inline]
    pub const fn from_ref(r: &X) -> &Self {
        // SAFETY: `ConstructorId<X>` is `#[repr(transparent)]` over `X`.
        unsafe { &*ptr::from_ref(r).cast() }
    }

    #[inline]
    pub const fn from_mut(r: &mut X) -> &mut Self {
        // SAFETY: `ConstructorId<X>` is `#[repr(transparent)]` over `X`.
        unsafe { &mut *ptr::from_mut(r).cast() }
    }
}

impl<X: Function> SerializedLen for ConstructorId<X> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        4 + self.0.serialized_len()
    }
}

impl<X: Function> ser::SerializeUnchecked for ConstructorId<X> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = X::CONSTRUCTOR_ID.serialize_unchecked(buf);
            self.0.serialize_unchecked(buf)
        }
    }
}
