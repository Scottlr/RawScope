//! Event-loop scheduling state for timeline viewport refinement.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimelineScheduledWork {
    Exact { revision: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    SettledExact,
    Reprojecting,
    ExactPending,
    ExactInFlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimelineRenderSchedule {
    phase: Phase,
    latest_revision: u64,
}

impl Default for TimelineRenderSchedule {
    fn default() -> Self {
        Self {
            phase: Phase::SettledExact,
            latest_revision: 0,
        }
    }
}

impl TimelineRenderSchedule {
    pub(crate) fn gesture_started(&mut self) {
        if self.phase == Phase::SettledExact {
            self.phase = Phase::Reprojecting;
        }
    }
    pub(crate) fn viewport_changed(&mut self) -> u64 {
        self.latest_revision = self.latest_revision.saturating_add(1);
        self.phase = Phase::Reprojecting;
        self.latest_revision
    }
    pub(crate) fn gesture_released(&mut self) {
        if self.phase != Phase::SettledExact {
            self.phase = Phase::ExactPending;
        }
    }
    pub(crate) fn request_exact(&mut self) {
        self.latest_revision = self.latest_revision.saturating_add(1);
        self.phase = Phase::ExactPending;
    }
    pub(crate) fn next_work(&mut self) -> Option<TimelineScheduledWork> {
        if self.phase != Phase::ExactPending {
            return None;
        }
        let revision = self.latest_revision;
        self.phase = Phase::ExactInFlight;
        Some(TimelineScheduledWork::Exact { revision })
    }
    pub(crate) fn work_completed(&mut self, work: TimelineScheduledWork) -> bool {
        let TimelineScheduledWork::Exact { revision } = work;
        if self.phase != Phase::ExactInFlight || revision != self.latest_revision {
            return false;
        }
        self.phase = Phase::SettledExact;
        true
    }
    pub(crate) fn work_failed(&mut self) {
        self.phase = Phase::ExactPending;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewport_changes_coalesce_until_one_exact_release() {
        let mut schedule = TimelineRenderSchedule::default();
        schedule.gesture_started();
        assert_eq!(schedule.viewport_changed(), 1);
        assert_eq!(schedule.viewport_changed(), 2);
        schedule.gesture_released();
        assert_eq!(
            schedule.next_work(),
            Some(TimelineScheduledWork::Exact { revision: 2 })
        );
        assert_eq!(schedule.next_work(), None);
    }
    #[test]
    fn stale_completion_does_not_settle_newer_viewport() {
        let mut schedule = TimelineRenderSchedule::default();
        schedule.viewport_changed();
        schedule.gesture_released();
        let work = schedule.next_work().unwrap();
        schedule.viewport_changed();
        assert!(!schedule.work_completed(work));
        assert_eq!(schedule.next_work(), None);
    }
    #[test]
    fn failed_exact_work_is_retryable() {
        let mut schedule = TimelineRenderSchedule::default();
        schedule.viewport_changed();
        schedule.gesture_released();
        let work = schedule.next_work().unwrap();
        schedule.work_failed();
        assert_eq!(schedule.next_work(), Some(work));
    }
}
