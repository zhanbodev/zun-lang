use chrono::Utc;

#[derive(Debug, Clone)]
pub(crate) struct LazyGcWindow {
    next_gc_at: i64,
    interval_seconds: i64,
}

impl LazyGcWindow {
    pub(crate) fn new(interval_seconds: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            next_gc_at: now.saturating_add(interval_seconds),
            interval_seconds,
        }
    }

    pub(crate) fn is_due(&self, now: i64) -> bool {
        now >= self.next_gc_at
    }

    pub(crate) fn mark_ran(&mut self, now: i64) {
        self.next_gc_at = now.saturating_add(self.interval_seconds);
    }

    #[cfg(test)]
    pub(crate) fn set_next_gc_at_for_test(&mut self, next_gc_at: i64) {
        self.next_gc_at = next_gc_at;
    }
}
