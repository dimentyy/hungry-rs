use std::fmt;

#[inline(always)]
pub const fn byte(x: u8) -> [u8; 2] {
    #[cfg(target_endian = "little")]
    let x = ((x as u16 & 0x0f) << 8) | ((x as u16 & 0xf0) >> 4);

    #[cfg(target_endian = "big")]
    let x = ((x as u16 & 0xf0) << 4) | (x as u16 & 0x0f);

    let mask = ((x + 0x0606) & 0x1010) >> 4;

    let offset = const { (b'a' - b'0' - 10) as u16 } * mask;

    (x + const { b'0' as u16 * 0x0101 } + offset).to_ne_bytes()
}

/// # Errors
///
/// * Returns the first `fmt::Error` if any underlying operation fails.
pub fn bytes_fmt(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("x\"")?;

    let mut buf = [0; 128];

    for chunk in bytes.chunks(64) {
        let mut i = 0;

        for &x in chunk {
            let [hi, lo] = byte(x);

            buf[i] = hi;
            buf[i + 1] = lo;

            i += 2;
        }

        // SAFETY: `byte` always returns a valid UTF-8 representation of a byte.
        let s = unsafe { str::from_utf8_unchecked(&buf[..i]) };

        f.write_str(s)?;
    }

    f.write_str("\"")
}

/// # Panics
///
/// * If the provided string `s` contains invalid hexadecimal characters.
/// * If the constant `N` is not an exact number of decoded bytes.
pub const fn decode<const N: usize>(s: &str) -> [u8; N] {
    #[inline(always)]
    const fn nibble(x: u8) -> u8 {
        assert!((b'0' <= x && x <= b'9') || (b'A' <= x && x < b'Z') || (b'a' <= x && x <= b'z'));

        (x & 0x0f) + (x >> 6) * 9
    }

    assert!(s.len() == N * 2);

    let bytes = s.as_bytes();

    let mut buf = [0; N];

    let mut i = 0;

    while i < N {
        buf[i] = (nibble(bytes[i * 2]) << 4) + nibble(bytes[i * 2 + 1]);

        i += 1;
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_hex_byte() {
        for i in 0..=255 {
            assert_eq!(str::from_utf8(&byte(i)).unwrap(), format!("{i:02x}"));
        }
    }

    #[test]
    fn test_common_hex_decode() {
        let buf = std::array::from_fn::<_, 256, _>(|x| x as u8);

        let mut s = String::with_capacity(512);

        for b in buf {
            s.push_str(str::from_utf8(&byte(b)).unwrap());
        }

        assert_eq!(decode(&s), buf);
    }
}
