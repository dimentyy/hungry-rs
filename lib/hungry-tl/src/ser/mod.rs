mod big_int;
mod bytes;
mod primitives;
mod vec;

pub use bytes::{bytes_len, prepare_bytes};

pub trait Serialize {
    fn serialized_len(&self) -> usize;

    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8;
}
