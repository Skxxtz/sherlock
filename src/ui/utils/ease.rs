use std::f32::consts::PI;

#[allow(dead_code)]
pub struct Ease;

#[allow(dead_code)]
impl Ease {
    /// Creates a tactile "pop" effect by overshooting the target and then pulling back.
    ///
    /// This curve accelerates quickly at the start, overshoots the target value (1.0),
    /// and then settles back into place. It is perfect for interactive elements like
    /// buttons, list items, or modals that need to feel "physical."
    pub fn ease_out_back(t: f32) -> f32 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;

        let t_minus_one = t - 1.0;
        1.0 + c3 * t_minus_one.powi(3) + c1 * t_minus_one.powi(2)
    }

    /// Provides an aggressive, snappy deceleration curve.
    ///
    /// Starts with maximum velocity and decelerates extremely quickly. This feel is
    /// often described as "premium" or "high-momentum." It is excellent for
    /// command palettes, dropdowns, or notifications that need to feel instant.
    pub fn ease_out_expo(t: f32) -> f32 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - 2.0f32.powf(-10.0 * t)
        }
    }

    /// Creates an organic, fluid motion with a gentle acceleration and deceleration.
    ///
    /// This curve avoids sudden jumps in velocity, making it the most "natural"
    /// feeling curve. It is ideal for transitions, hover states, or animations
    /// that should feel soft and elegant rather than energetic.
    pub fn ease_in_out_sine(t: f32) -> f32 {
        -((PI * t).cos() - 1.0) / 2.0
    }
}
