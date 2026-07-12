//! Nonblocking readback ticket state used by GPU owners.

use std::time::{Duration, Instant};

use crate::DeviceGeneration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadbackGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadbackError {
    MapFailed,
    DeviceLost,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadbackState {
    Submitted,
    Ready,
    Failed(ReadbackError),
    Cancelled,
    Taken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadbackProgress<T> {
    Pending,
    Ready(T),
    Failed(ReadbackError),
    Cancelled,
}

/// Owns one readback result and makes terminal publication one-shot.
#[derive(Debug)]
pub struct GpuReadbackTicket<T> {
    generation: ReadbackGeneration,
    device_generation: DeviceGeneration,
    deadline: Instant,
    state: ReadbackState,
    result: Option<T>,
}

impl<T> GpuReadbackTicket<T> {
    pub fn submitted(
        generation: ReadbackGeneration,
        device_generation: DeviceGeneration,
        now: Instant,
        timeout: Duration,
    ) -> Self {
        Self {
            generation,
            device_generation,
            deadline: now + timeout,
            state: ReadbackState::Submitted,
            result: None,
        }
    }

    pub fn generation(&self) -> ReadbackGeneration {
        self.generation
    }

    pub fn device_generation(&self) -> DeviceGeneration {
        self.device_generation
    }

    pub fn state(&self) -> ReadbackState {
        self.state
    }

    pub fn complete(&mut self, result: Result<T, ReadbackError>) -> bool {
        if !matches!(self.state, ReadbackState::Submitted) {
            return false;
        }
        match result {
            Ok(result) => {
                self.result = Some(result);
                self.state = ReadbackState::Ready;
            }
            Err(error) => self.state = ReadbackState::Failed(error),
        }
        true
    }

    pub fn cancel(&mut self) -> bool {
        if !matches!(self.state, ReadbackState::Submitted) {
            return false;
        }
        self.state = ReadbackState::Cancelled;
        true
    }

    pub fn advance(&mut self, now: Instant) {
        if now >= self.deadline && matches!(self.state, ReadbackState::Submitted) {
            self.state = ReadbackState::Failed(ReadbackError::TimedOut);
        }
    }

    pub fn take_result(&mut self) -> ReadbackProgress<T> {
        match self.state {
            ReadbackState::Submitted => ReadbackProgress::Pending,
            ReadbackState::Ready => {
                self.state = ReadbackState::Taken;
                match self.result.take() {
                    Some(result) => ReadbackProgress::Ready(result),
                    None => ReadbackProgress::Failed(ReadbackError::MapFailed),
                }
            }
            ReadbackState::Failed(error) => {
                self.state = ReadbackState::Taken;
                ReadbackProgress::Failed(error)
            }
            ReadbackState::Cancelled => {
                self.state = ReadbackState::Taken;
                ReadbackProgress::Cancelled
            }
            ReadbackState::Taken => ReadbackProgress::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_is_terminal_and_result_is_taken_once() {
        let now = Instant::now();
        let mut ticket = GpuReadbackTicket::submitted(
            ReadbackGeneration(7),
            DeviceGeneration(3),
            now,
            Duration::from_secs(1),
        );
        assert!(ticket.complete(Ok(42_u32)));
        assert!(!ticket.complete(Ok(99)));
        assert_eq!(ticket.take_result(), ReadbackProgress::Ready(42));
        assert_eq!(ticket.take_result(), ReadbackProgress::Pending);
    }

    #[test]
    fn timeout_and_cancellation_cannot_publish_data() {
        let now = Instant::now();
        let mut timed_out = GpuReadbackTicket::<u32>::submitted(
            ReadbackGeneration(1),
            DeviceGeneration(1),
            now,
            Duration::ZERO,
        );
        timed_out.advance(now);
        assert_eq!(
            timed_out.take_result(),
            ReadbackProgress::Failed(ReadbackError::TimedOut)
        );

        let mut cancelled = GpuReadbackTicket::submitted(
            ReadbackGeneration(2),
            DeviceGeneration(1),
            now,
            Duration::from_secs(1),
        );
        assert!(cancelled.cancel());
        assert_eq!(cancelled.take_result(), ReadbackProgress::Cancelled);
        assert!(!cancelled.complete(Ok(1)));
    }
}
