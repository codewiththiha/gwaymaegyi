//! Monotonically widening root aspiration windows with bounded retry state.

use crate::{INFINITY, MATE_THRESHOLD};

#[derive(Clone, Copy, Debug)]
pub(super) struct Window {
    pub bounds: [i32; 2],
    margin: i32,
}
impl Window {
    pub(super) fn new(guess: Option<i32>, margin: i32) -> Self {
        let bounds = guess
            .filter(|score| score.abs() < MATE_THRESHOLD)
            .map_or([-INFINITY, INFINITY], |score| {
                [score - margin, score + margin]
            });
        Self { bounds, margin }
    }
    pub(super) fn retry(&mut self, score: i32) -> bool {
        if score <= self.bounds[0] && self.bounds[0] > -INFINITY {
            self.bounds[0] = (score - self.margin).max(-INFINITY);
        } else if score >= self.bounds[1] && self.bounds[1] < INFINITY {
            self.bounds[1] = (score + self.margin).min(INFINITY);
        } else {
            return false;
        }
        self.margin = (self.margin * 2).min(INFINITY);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Window;
    use crate::INFINITY;
    #[test]
    fn failed_bounds_only_expand_and_eventually_become_full() {
        let mut window = Window::new(Some(0), 20);
        assert!(window.retry(-80));
        assert_eq!(window.bounds[1], 20);
        assert!(window.retry(90));
        assert!(window.bounds[0] <= -100);
        for _ in 0..20 {
            window.retry(INFINITY);
        }
        assert_eq!(window.bounds[1], INFINITY);
        assert!(!window.retry(0));
    }
}
