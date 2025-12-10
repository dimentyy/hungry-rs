use crate::tl::Int128;

#[derive(Debug)]
pub struct NonceMismatch {
    pub expected: Int128,
    pub received: Int128,
}
