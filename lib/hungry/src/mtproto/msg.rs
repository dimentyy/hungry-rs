use std::fmt;
use std::ops::Deref;
use std::ptr::NonNull;

use crate::{common, mtproto, tl};

use common::infallible;

use tl::de::DeserializeInfallible;
use tl::ser::SerializeUnchecked;
use tl::{ConstSerializedLen, SerializedLen};

#[derive(Copy, Clone, Debug)]
pub struct MsgNil;

#[must_use]
#[derive(Copy, Clone, Debug)]
pub struct Msg<T = MsgNil> {
    pub msg_id: mtproto::MsgId,
    pub seq_no: mtproto::SeqNo,
    pub object: T,
}

pub type BufMsg<'a> = Msg<tl::de::Buf<'a>>;

impl Msg<MsgNil> {
    pub fn nil(msg_id: mtproto::MsgId, seq_no: mtproto::SeqNo) -> Self {
        Self {
            msg_id,
            seq_no,
            object: MsgNil,
        }
    }
}

impl ConstSerializedLen for Msg<MsgNil> {
    const SERIALIZED_LEN: usize = mtproto::MsgId::SERIALIZED_LEN + mtproto::SeqNo::SERIALIZED_LEN;
}

impl SerializeUnchecked for Msg<MsgNil> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = self.msg_id.serialize_unchecked(buf);
            buf = self.seq_no.serialize_unchecked(buf);
        }

        buf
    }
}

impl DeserializeInfallible for Msg<MsgNil> {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
        unsafe {
            Self {
                msg_id: i64::deserialize_infallible(buf),
                seq_no: i32::deserialize_infallible(buf.add(8)),
                object: MsgNil,
            }
        }
    }
}

impl<X: SerializeUnchecked, T: Deref<Target = X>> SerializedLen for Msg<T> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        16 + self.object.serialized_len()
    }
}

impl<X: SerializeUnchecked, T: Deref<Target = X>> SerializeUnchecked for Msg<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = self.msg_id.serialize_unchecked(buf);
            buf = self.seq_no.serialize_unchecked(buf);

            buf = i32::try_from(self.object.serialized_len())
                .unwrap()
                .serialize_unchecked(buf);

            self.object.serialize_unchecked(buf)
        }
    }
}

#[derive(Debug)]
pub enum MsgDeError {
    HeaderTooSmall(tl::de::EndOfBufferError),
    NegativeLength,
    IncompleteBody(tl::de::EndOfBufferError),
}

impl fmt::Display for MsgDeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`mtproto::Msg<tl::de::Buf>` deserialization error: ")?;

        match self {
            MsgDeError::HeaderTooSmall(err) => write!(f, "header too small: {err}"),
            MsgDeError::NegativeLength => f.write_str("negative length"),
            MsgDeError::IncompleteBody(err) => write!(f, "incomplete body: {err}"),
        }
    }
}

impl std::error::Error for MsgDeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use MsgDeError::*;

        Some(match self {
            HeaderTooSmall(err) => err,
            NegativeLength => return None,
            IncompleteBody(err) => err,
        })
    }
}

impl<'a> Msg<tl::de::Buf<'a>> {
    pub fn deserialize(buf: &mut tl::de::Buf<'a>) -> Result<Self, MsgDeError> {
        let header = buf
            .take_exactly::<16>()
            .map_err(MsgDeError::HeaderTooSmall)?;

        infallible! {
            let msg_id = i64::from_le_bytes(header[0..8].try_into().unwrap());
            let seq_no = i32::from_le_bytes(header[8..12].try_into().unwrap());
            let bytes = i32::from_le_bytes(header[12..16].try_into().unwrap());
        };

        let Ok(bytes) = usize::try_from(bytes) else {
            return Err(MsgDeError::NegativeLength);
        };

        let buf = tl::de::Buf::new(buf.take(bytes).map_err(MsgDeError::IncompleteBody)?);

        Ok(Self {
            msg_id,
            seq_no,
            object: buf,
        })
    }
}
