use core::time::Duration;

use crate::color::Color;

/// A box shadow specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
}

impl Shadow {
    pub fn new(
        offset_x: f32,
        offset_y: f32,
        blur_radius: f32,
        spread_radius: f32,
        color: Color,
    ) -> Self {
        Self {
            offset_x,
            offset_y,
            blur_radius,
            spread_radius,
            color,
        }
    }
}

/// Easing function for animations and transitions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
    Spring {
        stiffness: f32,
        damping: f32,
        mass: f32,
    },
}

/// A CSS-like transition for a single property.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub property: AnimatableProperty,
    pub duration: Duration,
    pub easing: Easing,
    pub delay: Duration,
}

impl Transition {
    pub fn new(property: AnimatableProperty, duration: Duration) -> Self {
        Self {
            property,
            duration,
            easing: Easing::default(),
            delay: Duration::ZERO,
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

/// A keyframe in an animation sequence, specifying progress (0.0–1.0) and the property value.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    pub progress: f32,
    pub property: AnimatableProperty,
    pub value: AnimatableValue,
    pub easing: Easing,
}

/// How many times an animation repeats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationIterations {
    Count(u32),
    Infinite,
}

impl Default for AnimationIterations {
    fn default() -> Self {
        Self::Count(1)
    }
}

/// Direction of animation playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnimationDirection {
    #[default]
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// A keyframe animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub keyframes: Vec<Keyframe>,
    pub duration: Duration,
    pub iterations: AnimationIterations,
    pub direction: AnimationDirection,
    pub delay: Duration,
    pub easing: Easing,
}

impl Animation {
    pub fn new(keyframes: Vec<Keyframe>, duration: Duration) -> Self {
        Self {
            keyframes,
            duration,
            iterations: AnimationIterations::default(),
            direction: AnimationDirection::default(),
            delay: Duration::ZERO,
            easing: Easing::default(),
        }
    }

    pub fn with_iterations(mut self, iterations: AnimationIterations) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_direction(mut self, direction: AnimationDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

/// Properties that can be animated or transitioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatableProperty {
    Opacity,
    BackgroundColor,
    ForegroundColor,
    BorderColor,
    Width,
    Height,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    BorderRadius,
    BorderWidth,
    FontSize,
    LetterSpacing,
    LineHeight,
    Gap,
    Transform,
    BoxShadow,
}

/// Concrete animatable values used in keyframes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimatableValue {
    Float(f32),
    Color(Color),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_new() {
        let s = Shadow::new(2.0, 4.0, 8.0, 0.0, Color::BLACK);
        assert_eq!(s.offset_x, 2.0);
        assert_eq!(s.offset_y, 4.0);
        assert_eq!(s.blur_radius, 8.0);
        assert_eq!(s.spread_radius, 0.0);
        assert_eq!(s.color, Color::BLACK);
    }

    // AC-11
    #[test]
    fn easing_cubic_bezier() {
        let e = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0);
        assert_eq!(e, Easing::CubicBezier(0.25, 0.1, 0.25, 1.0));
    }

    // AC-12
    #[test]
    fn easing_spring() {
        let e = Easing::Spring {
            stiffness: 100.0,
            damping: 10.0,
            mass: 1.0,
        };
        match e {
            Easing::Spring {
                stiffness,
                damping,
                mass,
            } => {
                assert_eq!(stiffness, 100.0);
                assert_eq!(damping, 10.0);
                assert_eq!(mass, 1.0);
            }
            _ => panic!("Expected Spring"),
        }
    }

    #[test]
    fn transition_builder() {
        let t = Transition::new(AnimatableProperty::Opacity, Duration::from_millis(300))
            .with_easing(Easing::EaseIn)
            .with_delay(Duration::from_millis(100));
        assert_eq!(t.property, AnimatableProperty::Opacity);
        assert_eq!(t.duration, Duration::from_millis(300));
        assert_eq!(t.easing, Easing::EaseIn);
        assert_eq!(t.delay, Duration::from_millis(100));
    }

    #[test]
    fn animation_builder() {
        let anim = Animation::new(vec![], Duration::from_secs(1))
            .with_iterations(AnimationIterations::Infinite)
            .with_direction(AnimationDirection::Alternate)
            .with_delay(Duration::from_millis(500))
            .with_easing(Easing::Linear);
        assert_eq!(anim.iterations, AnimationIterations::Infinite);
        assert_eq!(anim.direction, AnimationDirection::Alternate);
        assert_eq!(anim.delay, Duration::from_millis(500));
        assert_eq!(anim.easing, Easing::Linear);
    }

    #[test]
    fn animation_iterations_default() {
        assert_eq!(
            AnimationIterations::default(),
            AnimationIterations::Count(1)
        );
    }

    #[test]
    fn animation_direction_default() {
        assert_eq!(AnimationDirection::default(), AnimationDirection::Normal);
    }

    #[test]
    fn easing_default() {
        assert_eq!(Easing::default(), Easing::EaseInOut);
    }

    #[test]
    fn animatable_value_variants() {
        let f = AnimatableValue::Float(1.0);
        let c = AnimatableValue::Color(Color::RED);
        assert_eq!(f, AnimatableValue::Float(1.0));
        assert_eq!(c, AnimatableValue::Color(Color::RED));
    }
}
