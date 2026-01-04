use std::{fmt, ops, ptr};

use crate::hex;

macro_rules! big_int {
    ( $( $typ:ident => $len:literal , $bit:literal , $rep:literal );+ $( ; )? ) => { $(
        #[doc = concat!(
            "# int", $bit,
            "\n\nRepresents a ", $bit, "-bit integer.\
            \n\n```tl\n",
            "int", $bit, " ", $rep, "*[ int ] = Int", $bit, ";\
            \n```",
        )]
        #[must_use]
        #[repr(transparent)]
        #[derive(Clone, Default, Eq, PartialEq)]
        pub struct $typ(pub [u8; $len]);

        impl $typ {
            #[inline]
            pub const unsafe fn from_ref(r: &[u8; $len]) -> &Self {
                // SAFETY: `$typ` is `#[repr(transparent)]` over `[u8; $len]`.
                unsafe { &*ptr::from_ref(r).cast() }
            }

            #[inline]
            pub const unsafe fn from_mut(r: &mut [u8; $len]) -> &mut Self {
                // SAFETY: `$typ` is `#[repr(transparent)]` over `[u8; $len]`.
                unsafe { &mut *ptr::from_mut(r).cast() }
            }
        }

        impl AsRef<[u8; $len]> for $typ {
            #[inline]
            fn as_ref(&self) -> &[u8; $len] {
                &self.0
            }
        }

        impl AsMut<[u8; $len]> for $typ {
            #[inline]
            fn as_mut(&mut self) -> &mut [u8; $len] {
                &mut self.0
            }
        }

        impl AsRef<[u8]> for $typ {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl AsMut<[u8]> for $typ {
            #[inline]
            fn as_mut(&mut self) -> &mut [u8] {
                &mut self.0
            }
        }

        impl fmt::Debug for $typ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut buf = [0; $len * 2 + 2];

                buf[0] = b'0';
                buf[1] = b'x';

                let mut buf_i = 2;
                let mut int_i = 0;

                while int_i < $len {
                    let [hi, lo] = hex::byte(self.0[int_i]);

                    int_i += 1;

                    buf[buf_i] = hi;
                    buf[buf_i + 1] = lo;

                    buf_i += 2;
                }

                // SAFETY: `hex::byte` always returns a valid UTF-8 hex representation of a byte.
                let s = unsafe { str::from_utf8_unchecked(&buf) };

                f.write_str(s)
            }
        }

        impl ops::Deref for $typ {
            type Target = [u8; $len];

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ops::DerefMut for $typ {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<[u8; $len]> for $typ {
            #[inline]
            fn from(value: [u8; $len]) -> Self {
                Self(value)
            }
        }

        impl PartialEq<[u8; $len]> for $typ {
            #[inline]
            fn eq(&self, other: &[u8; $len]) -> bool {
                &self.0 == other
            }
        }

        impl PartialEq<$typ> for [u8; $len] {
            #[inline]
            fn eq(&self, other: &$typ) -> bool {
                self == &other.0
            }
        }
    )+ };
}

big_int!(
    Int128 => 16, "128", "4";
    Int256 => 32, "256", "8";
);
