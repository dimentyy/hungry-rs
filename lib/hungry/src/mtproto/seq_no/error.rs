use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeqNoError {
    Even,
    Odd,
    Invalid,
}

impl fmt::Display for SeqNoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SeqNoError::*;

        f.write_str("`seq_no` error: ")?;

        f.write_str(match self {
            Even => "even for strictly content-related message",
            Odd => "odd for strictly non content-related message",
            Invalid => "invalid sequence number",
        })
    }
}

impl std::error::Error for SeqNoError {}
