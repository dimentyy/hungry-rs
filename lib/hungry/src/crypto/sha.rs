macro_rules! sha1 (
    ( $( $x:expr ),* $(,)? ) => ({
        use sha1::digest::Digest;

        let mut hasher = sha1::Sha1::new();

        $(
            hasher.update($x);
        )+

        hasher.finalize()
    })
);

pub(crate) use sha1;

macro_rules! sha256 (
    ( $( $x:expr ),* $(,)? ) => ({
        use sha2::digest::Digest;

        let mut hasher = sha2::Sha256::new();

        $(
            hasher.update($x);
        )+

        hasher.finalize()
    })
);

pub(crate) use sha256;
