// STATUS: stable.

macro_rules! crc32 {
    ( $( $x:expr ),+ $(,)? ) => ({
        let mut hasher = crc32fast::Hasher::new();

        $(
            hasher.update($x);
        )+

        hasher.finalize()
    })
}

pub(crate) use crc32;
