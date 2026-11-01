use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Watchdog {
    deadline: Option<Instant>,
}

impl Watchdog {
    pub fn renew(&mut self, now: Instant, ttl_ms: u64) {
        self.deadline = Some(now + Duration::from_millis(ttl_ms));
    }

    pub fn idle(&mut self) {
        self.deadline = None;
    }

    pub fn must_restore(&self, now: Instant, connected: bool, parent_alive: bool) -> bool {
        !connected || !parent_alive || self.deadline.is_some_and(|deadline| now >= deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_disconnect_and_parent_exit_restore() {
        let now = Instant::now();
        let mut watchdog = Watchdog::default();
        assert!(!watchdog.must_restore(now, true, true));
        watchdog.renew(now, 1000);
        assert!(!watchdog.must_restore(now + Duration::from_millis(999), true, true));
        assert!(watchdog.must_restore(now + Duration::from_secs(1), true, true));
        assert!(watchdog.must_restore(now, false, true));
        assert!(watchdog.must_restore(now, true, false));
        watchdog.idle();
        assert!(!watchdog.must_restore(now + Duration::from_secs(100), true, true));
    }
}
