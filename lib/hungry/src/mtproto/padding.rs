use std::fmt;
use std::ops::RangeInclusive;

pub const ENCRYPTED_DATA_PADDING: RangeInclusive<usize> = 12..=1024;

/// # Checking message length
///
/// The client must check that <...> the difference (i.e. the length
/// of the random padding) lies in the range from 12 to 1024 bytes.
///
/// ---
///
/// <https://core.telegram.org/mtproto/security_guidelines#checking-message-length>
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaddingError(pub usize);

impl fmt::Display for PaddingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "random padding does not lie in the range {:?} bytes: {}",
            ENCRYPTED_DATA_PADDING, self.0
        )
    }
}

impl std::error::Error for PaddingError {}

/// # Errros
///
/// * [`PaddingError`] occurs if padding does not
///   lie in the [`ENCRYPTED_DATA_PADDING`] range.
#[inline]
pub fn check_random_padding(buf: &[u8]) -> Result<(), PaddingError> {
    if !ENCRYPTED_DATA_PADDING.contains(&buf.len()) {
        return Err(PaddingError(buf.len()));
    }

    Ok(())
}
