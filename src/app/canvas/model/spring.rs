const STEP: f64 = 1. / 240.;

/// 속도를 보존하는 스프링. 목표가 바뀌어도 움직임이 끊기지 않아 중간에 끼어들 수 있다.
#[derive(Clone, Copy, Debug)]
pub(in crate::app::canvas) struct Spring {
    pub value: f64,
    pub target: f64,
    velocity: f64,
    stiffness: f64,
    damping: f64,
    tolerance: f64,
}

impl Spring {
    /// `response`는 진동 한 주기(초), `ratio`는 감쇠비다. 1이면 넘치지 않는다.
    pub fn new(value: f64, target: f64, response: f64, ratio: f64, tolerance: f64) -> Self {
        let omega = std::f64::consts::TAU / response;
        Self {
            value,
            target,
            velocity: 0.,
            stiffness: omega * omega,
            damping: 2. * ratio * omega,
            tolerance,
        }
    }

    pub fn advance(&mut self, elapsed: f64) {
        let elapsed = elapsed.clamp(0., 1.);
        let steps = (elapsed / STEP).ceil() as usize;
        let step = elapsed / steps.max(1) as f64;
        for _ in 0..steps {
            let acceleration =
                self.stiffness * (self.target - self.value) - self.damping * self.velocity;
            self.velocity += acceleration * step;
            self.value += self.velocity * step;
        }
        if self.settled() {
            self.value = self.target;
            self.velocity = 0.;
        }
    }

    pub fn settled(&self) -> bool {
        (self.target - self.value).abs() < self.tolerance && self.velocity.abs() < self.tolerance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_002_critical_spring_is_monotonic_and_settles() {
        let mut spring = Spring::new(0., 1., 0.3, 1., 0.0005);
        let mut last = 0.;
        for _ in 0..120 {
            spring.advance(1. / 60.);
            assert!(spring.value >= last && spring.value <= 1.);
            last = spring.value;
        }
        assert!(spring.settled());
        assert_eq!(spring.value, 1.);
    }

    #[test]
    fn canvas_002_retarget_keeps_velocity() {
        let mut spring = Spring::new(0., 100., 0.34, 0.86, 0.05);
        spring.advance(0.08);
        let (position, velocity) = (spring.value, spring.velocity);
        assert!(velocity > 0.);
        spring.target = 0.;
        spring.advance(1. / 240.);
        assert!(
            spring.value > position,
            "the card keeps travelling for a moment instead of snapping back"
        );
    }

    #[test]
    fn canvas_002_high_refresh_jitter_preserves_elapsed_time() {
        let simulate = |hz: usize, jitter: bool| {
            let mut spring = Spring::new(0., 100., 0.34, 1., 0.);
            for frame in 0..hz / 2 {
                let factor = if jitter {
                    if frame % 2 == 0 {
                        0.8
                    } else {
                        1.2
                    }
                } else {
                    1.
                };
                spring.advance(factor / hz as f64);
            }
            spring.value
        };
        let reference = simulate(120, false);
        for hz in [60, 120, 144] {
            let actual = simulate(hz, true);
            assert!(
                (actual - reference).abs() < 0.05,
                "{hz}Hz jitter advances by real elapsed time: {actual} vs {reference}"
            );
        }
    }

    #[test]
    fn canvas_002_frame_rate_does_not_change_the_result() {
        let mut slow = Spring::new(0., 1., 0.34, 0.86, 0.);
        let mut fast = Spring::new(0., 1., 0.34, 0.86, 0.);
        for _ in 0..6 {
            slow.advance(1. / 30.);
        }
        for _ in 0..24 {
            fast.advance(1. / 120.);
        }
        assert!((slow.value - fast.value).abs() < 1e-9);
    }
}
