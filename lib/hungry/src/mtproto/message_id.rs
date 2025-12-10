use std::fmt;

#[derive(Default)]
pub struct MessageIds {
    previous: u64,
}

impl fmt::Debug for MessageIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessageIds")
            .field("previous", &format_args!("{:#016x}", self.previous))
            .finish()
    }
}

impl MessageIds {
    #[inline]
    pub const fn new() -> Self {
        Self { previous: 0 }
    }

    #[inline]
    pub const fn previous(&self) -> i64 {
        self.previous as i64
    }

    pub const fn get(&mut self, since_epoch: std::time::Duration) -> i64 {
        let subsec_nanos = since_epoch.subsec_nanos() as u64;
        let message_id = (since_epoch.as_secs() << 32 | subsec_nanos << 2);

        if self.previous >= message_id {
            self.previous += 4;

            self.previous as i64
        } else {
            self.previous = message_id;

            message_id as i64
        }
    }
}
