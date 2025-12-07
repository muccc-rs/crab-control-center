#![allow(unused)]
use std::time;

#[derive(Default, Debug, Clone)]
pub struct BaseTimer<T> {
    change: Option<time::Instant>,
    last: T,
}

impl<T: PartialEq> BaseTimer<T> {
    pub fn new(start: T) -> Self {
        Self {
            change: None,
            last: start,
        }
    }

    pub fn run(&mut self, now: time::Instant, value: T) {
        if self.last != value {
            self.change = Some(now);
        }
        self.last = value;
    }

    /// Returns true if `preset` time has passed since the last value change
    pub fn timer(&self, now: time::Instant, preset: time::Duration) -> bool {
        self.change.map(|tt| now >= tt + preset).unwrap_or(true)
    }

    /// Returns the time since the last value change
    pub fn timer_value(&self, now: time::Instant) -> time::Duration {
        self.change
            .map(|tt| now - tt)
            .unwrap_or(time::Duration::ZERO)
    }

    /// Reset the "last" value without triggering change detection
    pub fn reset_value(&mut self, value: T) {
        self.last = value;
    }

    /// Trigger timer without a value change
    pub fn trigger(&mut self, now: time::Instant) {
        self.change = Some(now);
    }
}

#[juniper::graphql_object(context = crate::graphql::Context)]
#[graphql(name = "BaseTimer")]
impl<T: PartialEq> BaseTimer<T> {
    async fn time(&self, context: &crate::graphql::Context) -> Option<f64> {
        let now = context.inner.read().await.now;
        self.change.map(|tt| (now - tt).as_secs_f64())
    }

    #[graphql(name = "timer")]
    async fn timer_graphql(&self, preset: f64, context: &crate::graphql::Context) -> bool {
        let now = context.inner.read().await.now;
        self.timer(now, std::time::Duration::from_secs_f64(preset))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TimerResult {
    pub done: bool,
    pub timing: bool,
}

#[derive(Default, Debug, Clone)]
pub struct TimerOn {
    base: BaseTimer<bool>,
}

impl TimerOn {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, now: time::Instant, value: bool, preset: time::Duration) -> TimerResult {
        self.base.run(now, value);
        TimerResult {
            done: value && self.base.timer(now, preset),
            timing: value && !self.base.timer(now, preset),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct TimerOff {
    base: BaseTimer<bool>,
}

impl TimerOff {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, now: time::Instant, value: bool, preset: time::Duration) -> TimerResult {
        self.base.run(now, value);
        TimerResult {
            done: value || !self.base.timer(now, preset),
            timing: !value && !self.base.timer(now, preset),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct PulseTimer {
    base: BaseTimer<bool>,
}

impl PulseTimer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, now: time::Instant, value: bool, preset: time::Duration) -> TimerResult {
        let already_timing = !self.base.timer(now, preset);

        if already_timing || value {
            self.base.run(now, true)
        } else {
            self.base.reset_value(false);
        }

        TimerResult {
            done: !self.base.timer(now, preset),
            timing: !self.base.timer(now, preset),
        }
    }
}

#[juniper::graphql_object(context = crate::graphql::Context)]
#[graphql(name = "PulseTimer")]
impl PulseTimer {
    async fn time(&self, context: &crate::graphql::Context) -> Option<f64> {
        self.base.time(context).await
    }

    #[graphql(name = "timer")]
    async fn timer_graphql(&self, preset: f64, context: &crate::graphql::Context) -> bool {
        let now = context.inner.read().await.now;
        !self
            .base
            .timer(now, std::time::Duration::from_secs_f64(preset))
    }
}

pub trait TimeExt {
    fn millis(&self) -> time::Duration;
    fn secs(&self) -> time::Duration;
}

impl TimeExt for i32 {
    fn millis(&self) -> time::Duration {
        time::Duration::from_millis((*self).try_into().unwrap())
    }

    fn secs(&self) -> time::Duration {
        time::Duration::from_secs((*self).try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMESTEP: time::Duration = time::Duration::from_millis(10);
    const PRESET: time::Duration = time::Duration::from_millis(20);

    #[test]
    fn timer_on() {
        let inp = [0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0];
        let done = [0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0];
        let timing = [0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0];

        let mut timer = TimerOn::new();
        let mut now = time::Instant::now();

        for (i, ((inp, done), timing)) in inp
            .iter()
            .map(|i| *i != 0)
            .zip(done.iter().map(|i| *i != 0))
            .zip(timing.iter().map(|i| *i != 0))
            .enumerate()
        {
            let res = timer.run(now, inp, PRESET);
            assert_eq!(res.done, done, "`done` mismatch at timestep #{i}");
            assert_eq!(res.timing, timing, "`timing` mismatch at timestep #{i}");
            now += TIMESTEP;
        }
    }

    #[test]
    fn timer_on_start() {
        let inp = [1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0];
        let done = [0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0];
        let timing = [1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0, 0];

        let mut timer = TimerOn::new();
        let mut now = time::Instant::now();

        for (i, ((inp, done), timing)) in inp
            .iter()
            .map(|i| *i != 0)
            .zip(done.iter().map(|i| *i != 0))
            .zip(timing.iter().map(|i| *i != 0))
            .enumerate()
        {
            let res = timer.run(now, inp, PRESET);
            assert_eq!(res.done, done, "`done` mismatch at timestep #{i}");
            assert_eq!(res.timing, timing, "`timing` mismatch at timestep #{i}");
            now += TIMESTEP;
        }
    }

    #[test]
    fn timer_off() {
        let inp = [0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0];
        let done = [0, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0];
        let timing = [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0];

        let mut timer = TimerOff::new();
        let mut now = time::Instant::now();

        for (i, ((inp, done), timing)) in inp
            .iter()
            .map(|i| *i != 0)
            .zip(done.iter().map(|i| *i != 0))
            .zip(timing.iter().map(|i| *i != 0))
            .enumerate()
        {
            let res = timer.run(now, inp, PRESET);
            assert_eq!(res.done, done, "`done` mismatch at timestep #{i}");
            assert_eq!(res.timing, timing, "`timing` mismatch at timestep #{i}");
            now += TIMESTEP;
        }
    }

    #[test]
    fn timer_off_start() {
        let inp = [1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0];
        let done = [1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0];
        let timing = [0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 0];

        let mut timer = TimerOff::new();
        let mut now = time::Instant::now();

        for (i, ((inp, done), timing)) in inp
            .iter()
            .map(|i| *i != 0)
            .zip(done.iter().map(|i| *i != 0))
            .zip(timing.iter().map(|i| *i != 0))
            .enumerate()
        {
            let res = timer.run(now, inp, PRESET);
            assert_eq!(res.done, done, "`done` mismatch at timestep #{i}");
            assert_eq!(res.timing, timing, "`timing` mismatch at timestep #{i}");
            now += TIMESTEP;
        }
    }

    #[test]
    fn timer_pulse() {
        let inp = [0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0];
        let done = [0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0];
        let timing = done.clone();

        let mut timer = PulseTimer::new();
        let mut now = time::Instant::now();

        for (i, ((inp, done), timing)) in inp
            .iter()
            .map(|i| *i != 0)
            .zip(done.iter().map(|i| *i != 0))
            .zip(timing.iter().map(|i| *i != 0))
            .enumerate()
        {
            let res = timer.run(now, inp, time::Duration::from_millis(30));
            assert_eq!(res.done, done, "`done` mismatch at timestep #{i}");
            assert_eq!(res.timing, timing, "`timing` mismatch at timestep #{i}");
            now += TIMESTEP;
        }
    }

    #[test]
    fn timer_pulse_start() {
        let inp = [1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 0];
        let done = [1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0];
        let timing = done.clone();

        let mut timer = PulseTimer::new();
        let mut now = time::Instant::now();

        for (i, ((inp, done), timing)) in inp
            .iter()
            .map(|i| *i != 0)
            .zip(done.iter().map(|i| *i != 0))
            .zip(timing.iter().map(|i| *i != 0))
            .enumerate()
        {
            let res = timer.run(now, inp, time::Duration::from_millis(30));
            assert_eq!(res.done, done, "`done` mismatch at timestep #{i}");
            assert_eq!(res.timing, timing, "`timing` mismatch at timestep #{i}");
            now += TIMESTEP;
        }
    }
}
