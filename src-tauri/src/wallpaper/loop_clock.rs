use std::time::Duration;

const BUCKET_EPSILON: f64 = 1e-7;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct FilterTiming {
    /// Convert a media timestamp to active wall time with `t / speed + offset`.
    pub offset_seconds: f64,
    /// The first frame after a filter reset must reach at least this bucket.
    pub first_bucket: i64,
}

/// Keeps a looping video's timeline independent of frames removed by a filter.
/// All elapsed time is supplied by the owner, so pauses and tests need no timers.
#[derive(Debug, Clone)]
pub(super) struct LoopClock {
    duration: f64,
    position: f64,
    speed: f64,
    cap: u32,
    active_seconds: f64,
    loop_start_seconds: f64,
    logical_loop: u64,
    presented_loop: u64,
    playing: bool,
}

impl LoopClock {
    pub fn new(duration: f64, position: f64, speed: f64, cap: u32) -> Option<Self> {
        if !duration.is_finite()
            || duration <= 0.0
            || !position.is_finite()
            || position < 0.0
            || !speed.is_finite()
            || speed <= 0.0
            || cap == 0
        {
            return None;
        }

        let position = position.min(duration);
        Some(Self {
            duration,
            position,
            speed,
            cap,
            active_seconds: 0.0,
            loop_start_seconds: -position / speed,
            logical_loop: 0,
            presented_loop: 0,
            playing: true,
        })
    }

    pub fn advance(&mut self, elapsed: Duration, playing: bool) {
        self.playing = playing;
        if !playing {
            return;
        }

        let elapsed = elapsed.as_secs_f64();
        self.active_seconds += elapsed;
        let position = self.position + elapsed * self.speed;
        // Avoid delaying an exact loop boundary because of floating-point sums.
        let completed = ((position / self.duration) + 1e-10).floor() as u64;
        self.position = (position - completed as f64 * self.duration).max(0.0);
        if completed > 0 {
            self.logical_loop = self.logical_loop.saturating_add(completed);
            self.loop_start_seconds = self.active_seconds - self.position / self.speed;
        }
    }

    pub fn set_speed(&mut self, speed: f64) {
        if speed.is_finite() && speed > 0.0 {
            self.speed = speed;
        }
    }

    pub fn update_cap(&mut self, cap: u32) {
        if cap > 0 {
            self.cap = cap;
        }
    }

    pub fn filter_timing(&self) -> FilterTiming {
        FilterTiming {
            offset_seconds: self.active_seconds - self.position / self.speed,
            first_bucket: self.next_bucket(self.active_seconds),
        }
    }

    /// Call only when mpv has reached EOF and is retaining its last image.
    ///
    /// A filtered EOF can arrive early. Hold through the original loop's end,
    /// then wait for a cap bucket that belongs to the next virtual loop. Entire
    /// loops without such a bucket are skipped without seeking or clearing the
    /// retained image. Delaying the seek matters: mpv presents its first retained
    /// frame immediately even when that frame has a nonzero media timestamp.
    pub fn restart_due(&mut self) -> Option<FilterTiming> {
        if !self.playing || self.logical_loop <= self.presented_loop {
            return None;
        }

        let first_bucket = self.next_bucket(self.loop_start_seconds);
        let first_frame_seconds = first_bucket as f64 / f64::from(self.cap);
        let loop_end_seconds = self.active_seconds + (self.duration - self.position) / self.speed;
        if first_frame_seconds >= loop_end_seconds - f64::EPSILON
            || self.active_seconds + BUCKET_EPSILON / f64::from(self.cap) < first_frame_seconds
        {
            return None;
        }

        self.presented_loop = self.logical_loop;
        // If a poll is late, skip elapsed buckets rather than replaying them.
        let current_bucket =
            (self.active_seconds * f64::from(self.cap) + BUCKET_EPSILON).floor() as i64;
        Some(FilterTiming {
            offset_seconds: self.active_seconds - self.position / self.speed,
            first_bucket: first_bucket.max(current_bucket),
        })
    }

    fn next_bucket(&self, seconds: f64) -> i64 {
        (seconds * f64::from(self.cap) - BUCKET_EPSILON).ceil() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance(clock: &mut LoopClock, seconds: f64) {
        clock.advance(Duration::from_secs_f64(seconds), true);
    }

    fn close(left: f64, right: f64) {
        assert!((left - right).abs() < 1e-8, "{left} != {right}");
    }

    #[test]
    fn filtered_eof_waits_for_the_original_loop_duration() {
        let mut clock = LoopClock::new(2.0, 0.0, 1.0, 1).unwrap();
        advance(&mut clock, 1.0);
        assert!(clock.restart_due().is_none());
        advance(&mut clock, 0.999);
        assert!(clock.restart_due().is_none());
        advance(&mut clock, 0.001);
        let timing = clock.restart_due().unwrap();
        close(timing.offset_seconds, 2.0);
        assert_eq!(timing.first_bucket, 2);
        assert!(clock.restart_due().is_none());
    }

    #[test]
    fn pauses_freeze_the_loop_and_do_not_restart_at_eof() {
        let mut clock = LoopClock::new(2.0, 0.0, 1.0, 30).unwrap();
        advance(&mut clock, 1.0);
        clock.advance(Duration::from_secs(90), false);
        close(clock.active_seconds, 1.0);
        close(clock.position, 1.0);
        assert!(clock.restart_due().is_none());
        advance(&mut clock, 1.0);
        clock.advance(Duration::ZERO, false);
        assert!(clock.restart_due().is_none());
        clock.advance(Duration::ZERO, true);
        assert!(clock.restart_due().is_some());
    }

    #[test]
    fn speed_changes_preserve_media_position_and_adjust_remaining_time() {
        let mut clock = LoopClock::new(2.0, 0.0, 1.0, 10).unwrap();
        advance(&mut clock, 0.5);
        clock.set_speed(2.0);
        close(clock.filter_timing().offset_seconds, 0.25);
        advance(&mut clock, 0.74);
        assert!(clock.restart_due().is_none());
        advance(&mut clock, 0.01);
        // The loop ends at 1.25s, but the next cap bucket starts at 1.30s.
        assert!(clock.restart_due().is_none());
        advance(&mut clock, 0.05);
        let timing = clock.restart_due().unwrap();
        close(timing.offset_seconds, 1.25);
        assert_eq!(timing.first_bucket, 13);
    }

    #[test]
    fn empty_virtual_loops_keep_the_previous_image_until_an_eligible_bucket() {
        let mut clock = LoopClock::new(0.2, 0.0, 10.0, 1).unwrap();
        advance(&mut clock, 0.2);
        assert_eq!(clock.logical_loop, 10);
        assert!(clock.restart_due().is_none());
        advance(&mut clock, 0.8);
        let timing = clock.restart_due().unwrap();
        assert_eq!(clock.logical_loop, 50);
        assert_eq!(timing.first_bucket, 1);
        close(timing.offset_seconds, 1.0);
    }

    #[test]
    fn late_polls_skip_elapsed_loops_and_buckets() {
        let mut clock = LoopClock::new(1.0, 0.0, 10.0, 15).unwrap();
        advance(&mut clock, 0.37);
        let timing = clock.restart_due().unwrap();
        assert_eq!(clock.logical_loop, 3);
        close(timing.offset_seconds, 0.3);
        assert_eq!(timing.first_bucket, 5);
        assert!(clock.restart_due().is_none());
    }

    #[test]
    fn cap_changes_recompute_eligibility_while_holding_a_frame() {
        let mut clock = LoopClock::new(1.0, 0.0, 10.0, 1).unwrap();
        advance(&mut clock, 0.3);
        assert!(clock.restart_due().is_none());
        clock.update_cap(20);
        let timing = clock.restart_due().unwrap();
        assert_eq!(timing.first_bucket, 6);
        close(timing.offset_seconds, 0.3);
    }

    #[test]
    fn phase_carry_keeps_fifteen_frames_across_ten_fast_loops() {
        let mut clock = LoopClock::new(1.0, 0.0, 10.0, 15).unwrap();
        let mut timing = clock.filter_timing();
        let mut frames = 0;
        for loop_index in 0..10 {
            if loop_index > 0 {
                let loop_start = f64::from(loop_index) / 10.0;
                let until_start = (loop_start - clock.active_seconds).max(0.0);
                advance(&mut clock, until_start);
                let first_bucket = (loop_start * 15.0 - BUCKET_EPSILON).ceil();
                let until_bucket = (first_bucket / 15.0 - clock.active_seconds).max(0.0);
                advance(&mut clock, until_bucket);
                timing = clock.restart_due().unwrap();
            }

            let mut previous_bucket = timing.first_bucket - 1;
            for frame in 0..60 {
                let timestamp = f64::from(frame) / 60.0;
                let bucket = ((timestamp / 10.0 + timing.offset_seconds) * 15.0 + BUCKET_EPSILON)
                    .floor() as i64;
                if bucket > previous_bucket {
                    previous_bucket = bucket;
                    frames += 1;
                }
            }
        }
        assert_eq!(frames, 15);
    }

    #[test]
    fn initialization_from_an_existing_position_and_invalid_metadata() {
        let mut clock = LoopClock::new(2.0, 1.0, 2.0, 30).unwrap();
        close(clock.filter_timing().offset_seconds, -0.5);
        advance(&mut clock, 0.5);
        close(clock.restart_due().unwrap().offset_seconds, 0.5);
        assert!(LoopClock::new(0.0, 0.0, 1.0, 30).is_none());
        assert!(LoopClock::new(f64::NAN, 0.0, 1.0, 30).is_none());
        assert!(LoopClock::new(2.0, f64::INFINITY, 1.0, 30).is_none());
        assert!(LoopClock::new(2.0, 0.0, 0.0, 30).is_none());
        assert!(LoopClock::new(2.0, 0.0, 1.0, 0).is_none());
    }
}
