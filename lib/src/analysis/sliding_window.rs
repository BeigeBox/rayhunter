//! A fixed-size sliding window counter for computing ratios over recent messages.
//!
//! Used by the IMSI exposure ratio analyzer to track the fraction of
//! IMSI-exposing messages within a rolling window of the most recent N messages.

use std::collections::VecDeque;

/// Tracks a boolean signal (exposed/not-exposed) over a sliding window
/// of the most recent `window_size` observations, and computes the ratio
/// of positive signals to total observations.
pub struct SlidingWindowRatio {
    /// Ring buffer of recent observations. `true` = IMSI-exposing.
    window: VecDeque<bool>,
    /// Maximum number of observations to retain.
    window_size: usize,
    /// Running count of `true` values in the window for O(1) ratio computation.
    positive_count: usize,
}

impl SlidingWindowRatio {
    /// Create a new sliding window counter with the given capacity.
    ///
    /// `window_size` must be > 0.
    pub fn new(window_size: usize) -> Self {
        assert!(window_size > 0, "window_size must be positive");
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
            positive_count: 0,
        }
    }

    /// Record an observation. If `is_positive` is true, this message was
    /// IMSI-exposing. The oldest observation is evicted if the window is full.
    pub fn push(&mut self, is_positive: bool) {
        if self.window.len() == self.window_size {
            if let Some(evicted) = self.window.pop_front() {
                if evicted {
                    self.positive_count -= 1;
                }
            }
        }
        if is_positive {
            self.positive_count += 1;
        }
        self.window.push_back(is_positive);
    }

    /// Returns the current ratio of positive observations to total observations,
    /// or `None` if no observations have been recorded yet.
    pub fn ratio(&self) -> Option<f64> {
        if self.window.is_empty() {
            None
        } else {
            Some(self.positive_count as f64 / self.window.len() as f64)
        }
    }

    /// Returns the number of observations currently in the window.
    pub fn count(&self) -> usize {
        self.window.len()
    }

    /// Returns the number of positive (IMSI-exposing) observations in the window.
    pub fn positive_count(&self) -> usize {
        self.positive_count
    }

    /// Returns the configured window size.
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_window() {
        let window = SlidingWindowRatio::new(10);
        assert_eq!(window.ratio(), None);
        assert_eq!(window.count(), 0);
        assert_eq!(window.positive_count(), 0);
    }

    #[test]
    fn test_single_positive() {
        let mut window = SlidingWindowRatio::new(10);
        window.push(true);
        assert_eq!(window.ratio(), Some(1.0));
        assert_eq!(window.count(), 1);
        assert_eq!(window.positive_count(), 1);
    }

    #[test]
    fn test_single_negative() {
        let mut window = SlidingWindowRatio::new(10);
        window.push(false);
        assert_eq!(window.ratio(), Some(0.0));
        assert_eq!(window.count(), 1);
        assert_eq!(window.positive_count(), 0);
    }

    #[test]
    fn test_mixed_observations() {
        let mut window = SlidingWindowRatio::new(10);
        window.push(true);
        window.push(false);
        window.push(true);
        window.push(false);
        assert_eq!(window.ratio(), Some(0.5));
        assert_eq!(window.count(), 4);
        assert_eq!(window.positive_count(), 2);
    }

    #[test]
    fn test_window_eviction() {
        let mut window = SlidingWindowRatio::new(4);
        // Fill window: [true, false, true, false]
        window.push(true);
        window.push(false);
        window.push(true);
        window.push(false);
        assert_eq!(window.ratio(), Some(0.5));

        // Push a positive, evicts the first `true`: [false, true, false, true]
        window.push(true);
        assert_eq!(window.count(), 4);
        assert_eq!(window.positive_count(), 2);
        assert_eq!(window.ratio(), Some(0.5));

        // Push a positive, evicts `false`: [true, false, true, true]
        window.push(true);
        assert_eq!(window.positive_count(), 3);
        assert_eq!(window.ratio(), Some(0.75));
    }

    #[test]
    fn test_window_all_evicted_to_zero() {
        let mut window = SlidingWindowRatio::new(3);
        window.push(true);
        window.push(true);
        window.push(true);
        assert_eq!(window.ratio(), Some(1.0));

        // Evict all positives
        window.push(false);
        window.push(false);
        window.push(false);
        assert_eq!(window.ratio(), Some(0.0));
        assert_eq!(window.positive_count(), 0);
    }

    #[test]
    fn test_normal_network_ratio() {
        // Simulate normal network: <3% IMSI-exposing messages per Tucker et al.
        let mut window = SlidingWindowRatio::new(200);
        for i in 0..200 {
            // 2 out of 200 = 1% exposure rate
            window.push(i == 50 || i == 150);
        }
        let ratio = window.ratio().unwrap();
        assert!(ratio < 0.03, "Normal network should be <3% exposure");
        assert!((ratio - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_imsi_catcher_ratio() {
        // Simulate IMSI catcher: every connection triggers exposure
        let mut window = SlidingWindowRatio::new(200);
        for _ in 0..200 {
            window.push(true);
        }
        assert_eq!(window.ratio(), Some(1.0));
    }

    #[test]
    #[should_panic(expected = "window_size must be positive")]
    fn test_zero_window_panics() {
        SlidingWindowRatio::new(0);
    }
}
