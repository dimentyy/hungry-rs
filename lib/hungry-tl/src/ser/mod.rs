mod big_int;
mod buf;
mod primitives;
mod string;
mod vec;

use std::ptr::NonNull;

use crate::SerializedLen;

pub use buf::Buf;
pub use string::string_len;

pub trait SerializeUnchecked: SerializedLen {
    /// Serializes the instance into `buf` without checking its capacity.
    ///
    /// # Safety
    ///
    /// * `buf` must have at least [`serialized_len`] bytes of capacity.
    ///
    /// [`serialized_len`]: SerializedLen::serialized_len
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8>;
}
