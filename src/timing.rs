use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Moment {
    datetime: DateTime<Utc>,
    instant: Instant,
}

impl Moment {
    pub fn now() -> Self {
        Self {
            datetime: Utc::now(),
            instant: Instant::now(),
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

impl PartialOrd<Instant> for Moment {
    fn partial_cmp(&self, other: &Instant) -> Option<std::cmp::Ordering> {
        self.instant.partial_cmp(other)
    }
}

impl PartialEq<Instant> for Moment {
    fn eq(&self, other: &Instant) -> bool {
        self.instant.eq(other)
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
