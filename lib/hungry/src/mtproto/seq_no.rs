#[derive(Debug, Default)]
pub struct SeqNos {
    current: i32,
}

impl SeqNos {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self { current: 0 }
    }

    #[inline]
    #[must_use]
    pub const fn non_content_related(&self) -> i32 {
        self.current
    }

    #[inline]
    #[must_use]
    pub const fn get_content_related(&mut self) -> i32 {
        self.current += 2;
        self.current - 1
    }
}
