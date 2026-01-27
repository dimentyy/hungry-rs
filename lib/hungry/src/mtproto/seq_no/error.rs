use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum SeqNoError {
    Even,
    Odd,
    Invalid,
}

impl fmt::Display for SeqNoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`seq_no` validation error: ")?;

        f.write_str(match self {
            SeqNoError::Even => "even for content-related message",
            SeqNoError::Odd => "odd for non content-related message",
            SeqNoError::Invalid => "invalid sequence number",
        })
    }
}

impl std::error::Error for SeqNoError {}
