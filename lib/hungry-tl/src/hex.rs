use std::fmt;

#[inline(always)]
fn hex_digit(x: u8) -> u8 {
    x + if x <= 9 { b'0' } else { const { b'a' - 10 } }
}

#[inline(always)]
fn hex_byte(x: u8) -> [u8; 2] {
    [hex_digit(x >> 4), hex_digit(x & 15)]
}

/// Buffered `bytes` hex formatter.
pub(crate) struct HexBytesFmt<T: AsRef<[u8]>>(pub(crate) T);

impl<T: AsRef<[u8]>> fmt::Debug for HexBytesFmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("x\"")?;

        let mut buf = [0; 128];

        for chunk in self.0.as_ref().chunks(64) {
            let mut i = 0;

            for &x in chunk.iter() {
                let [h, l] = hex_byte(x);

                buf[i] = h;
                buf[i | 1] = l;

                i += 2;
            }

            // SAFETY: `hex_byte` always returns a valid utf-8 hex representation of a byte.
            let s = unsafe { str::from_utf8_unchecked(&buf[..i]) };

            f.write_str(s)?;
        }

        f.write_str("\"")
    }
}

/// Buffered `int128` and `int256` hex formatter.
pub(crate) struct HexIntFmt<'a, const N: usize>(pub(crate) &'a [u8; N]);

macro_rules! int {
    ( $( $len:expr ),+ $(,)? ) => { $(
        impl fmt::Debug for HexIntFmt<'_, $len> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut buf = [0; $len * 2 + 2];

                buf[0] = b'0';
                buf[1] = b'x';

                let mut int_i = const { cfg!(target_endian = "little") as usize * $len };
                let mut buf_i = 0;

                while {
                    #[cfg(target_endian = "big")]
                    { int_i < $len }

                    #[cfg(target_endian = "little")]
                    { int_i > 0 }
                } {
                    #[cfg(target_endian = "little")]
                    { int_i -= 1; }

                    buf_i += 2;

                    let [h, l] = hex_byte(self.0[int_i]);

                    buf[buf_i] = h;
                    buf[buf_i | 1] = l;

                    #[cfg(target_endian = "big")]
                    { int_i += 1; }
                }

                // SAFETY: `hex_byte` always returns a valid utf-8 hex representation of a byte.
                let s = unsafe { str::from_utf8_unchecked(&buf) };

                f.write_str(s)
            }
        }
    )+ };
}

int!(16, 32);
