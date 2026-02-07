use std::time::Instant;

pub(super) struct Now {
    unix_time: i32,
    instant: Instant,
}

impl Now {
    #[inline]
    pub(super) fn new(unix_time: i32) -> Self {
        let instant = Instant::now();

        Self { unix_time, instant }
    }

    #[inline]
    pub(super) fn unix_time(&self) -> i32 {
        self.unix_time + i32::try_from(self.instant.elapsed().as_secs()).unwrap()
    }
}
