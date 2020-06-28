use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct Moment {
    datetime: DateTime<Utc>,
}

impl Moment {
    pub fn now() -> Self {
        Self {
            datetime: Utc::now(),
        }
    }

    pub fn checked_duration_since(&self, other: &Moment) -> Option<Duration> {
        let difference = self
            .datetime
            .signed_duration_since(other.datetime)
            .to_std()
            .unwrap();
        if difference.as_nanos() > 0 {
            Some(difference)
        } else {
            None
        }
    }
}

pub struct FrequencyLimiter {
    limit: Duration,
    next_target: Option<Instant>,
}

impl FrequencyLimiter {
    pub fn new<D: Into<Duration>>(limit: D) -> Self {
        let limit = limit.into();
        Self {
            limit,
            next_target: None,
        }
    }

    pub fn remaining(&self) -> Option<Duration> {
        if let Some(next_target) = &self.next_target {
            next_target.checked_duration_since(Instant::now())
        } else {
            None
        }
    }

    pub fn ready(&self) -> bool {
        self.remaining().is_none()
    }

    pub fn advance_frame(&mut self) -> Instant {
        let now = Instant::now();
        self.next_target = match self.next_target {
            Some(next_target) => {
                let new_target = next_target.checked_add(self.limit).unwrap();
                if new_target > now {
                    Some(new_target)
                } else {
                    now.checked_add(self.limit)
                }
            }
            None => now.checked_add(self.limit),
        };
        self.next_target.unwrap()
    }
}
