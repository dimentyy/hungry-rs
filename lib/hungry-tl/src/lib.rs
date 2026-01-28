#![deny(
    // clippy::missing_panics_doc,
    // clippy::missing_errors_doc,
    unused_imports,
)]

mod flags;

pub mod de;
pub mod ser;

pub use hungry_common as common;

pub use common::tl::{BareVec, Bytes, Int128, Int256};

include!(concat!(env!("OUT_DIR"), "/hungry_tl/object.rs"));

#[allow(unused_imports, clippy::module_inception, clippy::large_enum_variant)]
pub mod api {
    include!(concat!(env!("OUT_DIR"), "/hungry_tl/api/mod.rs"));
}

#[allow(unused_imports, clippy::module_inception, clippy::large_enum_variant)]
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

/// Identifier of the `rpc_result` constructor:
///
/// ```tl
/// rpc_result#f35c6d01 req_msg_id:long result:Object = RpcResult;
/// ```
pub const RPC_RESULT: u32 = 0xf35c6d01;

/// Identifier of the `gzip_packed` constructor.
///
/// ```tl
/// msg_container#73f1f8dc messages:vector<message> = MessageContainer;
/// ```
pub const MSG_CONTAINER: u32 = 0x73f1f8dc;

/// Identifier of the `gzip_packed` constructor.
///
/// ```tl
/// gzip_packed#3072cfa1 packed_data:bytes = Object;
/// ```
pub const GZIP_PACKED: u32 = 0x3072cfa1;

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
    #[inline(always)]
    pub const fn from_ref(r: &X) -> &Self {
        // SAFETY: `ConstructorId<X>` is `#[repr(transparent)]` over `X`.
        unsafe { &*std::ptr::from_ref(r).cast() }
    }

    #[inline(always)]
    pub const fn from_mut(r: &mut X) -> &mut Self {
        // SAFETY: `ConstructorId<X>` is `#[repr(transparent)]` over `X`.
        unsafe { &mut *std::ptr::from_mut(r).cast() }
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
    unsafe fn serialize_unchecked(&self, mut buf: std::ptr::NonNull<u8>) -> std::ptr::NonNull<u8> {
        unsafe {
            buf = X::CONSTRUCTOR_ID.serialize_unchecked(buf);
            self.0.serialize_unchecked(buf)
        }
    }
}

#[inline]
pub fn ser<X: ser::SerializeUnchecked>(buf: &mut [u8], x: &X) {
    let _ = ser::Buf::new(buf).chain_ser(x);
}

#[inline]
#[must_use]
pub fn ser_uninit<'a, X: ser::SerializeUnchecked>(
    buf: &'a mut [std::mem::MaybeUninit<u8>],
    x: &X,
) -> &'a mut [u8] {
    ser::Buf::uninit(buf).chain_ser(x).as_mut_slice()
}

#[inline]
pub fn de<X: de::Deserialize>(buf: &[u8]) -> Result<X, de::Error> {
    let mut buf = de::Buf::new(buf);
    buf.de()
}
