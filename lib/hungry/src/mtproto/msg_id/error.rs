use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum MsgIdError {
    Even,
    Negative,
    LowerThanAll,
    EqualToAny,
    InTheFuture,
    InThePast,
}

impl fmt::Display for MsgIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MsgIdError::*;

        f.write_str("`msg_id` error: ")?;

        f.write_str(match self {
            Even => "even parity",
            Negative => "negative",
            LowerThanAll => "lower than all",
            EqualToAny => "equal to any",
            InTheFuture => "calculated `unix_time` is in the future",
            InThePast => "calculated `unix_time` is in the past",
        })
    }
}

impl std::error::Error for MsgIdError {}
