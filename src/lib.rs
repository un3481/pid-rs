//! A proportional-integral-derivative (PID) controller library.
//!
//! See [Pid] for the adjustable controller itself, as well as [ControlOutput] for the outputs and weights which you can use after setting up your controller. Follow the complete example below to setup your first controller!
//!
//! # Example
//!
//! ```rust
//! use pid::Pid;
//!
//! // Create a new proportional-only PID controller with a setpoint of 15
//! let mut pid = Pid::new()
//!     .setpoint(15.0)
//!     .limit(-100.0, 100.0)
//!     .p(10.0);
//!
//! // Input a measurement with an error of 5.0 from our setpoint
//! let output = pid.update(10.0).unwrap();
//!
//! // Show that the error is correct by multiplying by our kp
//! assert_eq!(output.output, 50.0); // <--
//! assert_eq!(output.p, 50.0);
//!
//! // It won't change on repeat; the controller is proportional-only
//! let output = pid.update(10.0).unwrap();
//! assert_eq!(output.output, 50.0); // <--
//! assert_eq!(output.p, 50.0);
//!
//! // Add a new integral term to the controller and input again
//! pid.i(1.0);
//! let output = pid.update(10.0).unwrap();
//!
//! // Now that the integral makes the controller stateful, it will change
//! assert_eq!(output.output, 55.0); // <--
//! assert_eq!(output.p, 50.0);
//! assert_eq!(output.i, 5.0);
//!
//! // Add our final derivative term and match our setpoint target
//! pid.d(2.0);
//! let output = pid.update(15.0).unwrap();
//!
//! // The output will now say to go down due to the derivative
//! assert_eq!(output.output, -5.0); // <--
//! assert_eq!(output.p, 0.0);
//! assert_eq!(output.i, 5.0);
//! assert_eq!(output.d, -10.0);
//! ```
#![no_std]

/// A trait for any numeric type usable in the PID controller
///
/// This trait is automatically implemented for all types that satisfy `PartialOrd + num_traits::Signed + Copy`. This includes all of the signed float types and builtin integer except for [isize]:
/// - [i8]
/// - [i16]
/// - [i32]
/// - [i64]
/// - [i128]
/// - [f32]
/// - [f64]
///
/// As well as any user type that matches the requirements
pub trait Number: PartialOrd + num_traits::Signed + Copy {}

// Implement `Number` for all types that
// satisfy `PartialOrd + num_traits::Signed + Copy`.
impl<T: PartialOrd + num_traits::Signed + Copy> Number for T {}

/// Adjustable proportional-integral-derivative (PID) controller.
///
/// # Examples
///
/// This controller provides a builder pattern interface which allows you to pick-and-choose which PID inputs you'd like to use during operation. Here's what a basic proportional-only controller could look like:
///
/// ```rust
/// use pid::Pid;
///
/// // Create limited controller
/// let mut p_controller = Pid::new()
///     .setpoint(15.0)
///     .limit(-100.0, 100.0)
///     .p(10.0);
///
/// // Get first output
/// let p_output = p_controller.update(400.0).unwrap();
/// ```
///
/// This controller would give you set a proportional controller to `10.0` with a target of `15.0` and an output limit of `100.0` per [output](Self::next_control_output) iteration. The same controller with a full PID system built in looks like:
///
/// ```rust
/// use pid::Pid;
///
/// // Create full PID controller
/// let mut full_controller = Pid::new()
///     .setpoint(15.0)
///     .limit(-100.0, 100.0);
///     .p(10.0)
///     .i(4.5)
///     .d(0.25);
///
/// // Get first output
/// let full_output = full_controller.update(400.0).unwrap();
/// ```
///
/// This [`update`](Self::update) method is what's used to input new values into the controller to tell it what the current state of the system is. In the examples above it's only being used once, but realistically this will be a hot method. Please see [ControlOutput] for examples of how to handle these outputs; it's quite straight forward and mirrors the values of this structure in some ways.
///
/// The last item of note is that these [`p`](Self::p()), [`i`](Self::i()), and [`d`](Self::d()) methods can be used *during* operation which lets you add and/or modify these controller values if need be.
///
/// # Type Warning
///
/// [Number] is abstract and can be used with anything from a [i32] to an [i128] (as well as user-defined types). Because of this, very small types might overflow during calculation in [`next_control_output`](Self::next_control_output). You probably don't want to use [i8] or user-defined types around that size so keep that in mind when designing your controller.
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Pid<T> {
    /// Ideal setpoint to strive for.
    pub setpoint: Option<T>,
    /// Proportional gain.
    pub kp: Option<T>,
    /// Integral gain.
    pub ki: Option<T>,
    /// Derivative gain.
    pub kd: Option<T>,
    /// Previous control output calculated unsing the [Pid::update()] method.
    pub prev: Option<ControlOutput<T>>,
    /// Limiter for the proportional term: `-p_limit <= P <= p_limit`.
    pub p_limit: PidLimit<T>,
    /// Limiter for the integral term: `i_limit_low <= I <= i_limit_high`.
    pub i_limit: PidLimit<T>,
    /// Limiter for the derivative term: `d_limit_low <= D <= d_limit_high`.
    pub d_limit: PidLimit<T>,
    /// Limiter for the derivative term: `o_limit_low <= O <= o_limit_high`.
    pub out_limit: PidLimit<T>, 
}

/// Limits of controller terms
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PidLimit<T> {
    min: Option<T>,
    max: Option<T>,
}

/// Output of [controller iterations](Pid::next_control_output) with weights
///
/// # Example
///
/// This structure is simple to use and features three weights: [p](Self::p), [i](Self::i), and [d](Self::d). These can be used to figure out how much each term from [Pid] contributed to the final [output](Self::output) value which should be taken as the final controller output for this iteration:
///
/// ```rust
/// use pid::{Pid, ControlOutput};
///
/// // Setup controller
/// let mut pid = Pid::new()
///     .setpoint(15.0)
///     .limit(0.0, 100.0)
///     .p(10.0)
///     .i(1.0)
///     .d(2.0);
///
/// // Input an example value and get a report for an output iteration
/// let output = pid.update(26.2456).unwrap();
/// println!("P: {}\nI: {}\nD: {}\nFinal Output: {}", output.p, output.i, output.d, output.output);
/// ```
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ControlOutput<T> {
    /// The input value to the controller.
    pub input: T,
    /// The error value based on input.
    pub error: T,
    /// Contribution of the P term to the output.
    pub p: T,
    /// Contribution of the I term to the output.
    ///
    /// This integral term is equal to `sum[error(t) * ki(t)] (for all t)`
    pub i: T,
    /// Contribution of the D term to the output.
    pub d: T,
    /// Output of the PID controller.
    pub output: T,
}

// PidLimit methods
impl<T> PidLimit<T>
where
    T: Number
{
    /// Creates a new limit struct
    pub const fn new() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    /// Sets the min and max limits
    pub fn set(&mut self, min: impl Into<T>, max: impl Into<T>) -> &mut Self {
        self.min = Some(min.into());
        self.max = Some(max.into());
        self
    }

    /// Clamp a given value using pre-defined limits
    pub fn clamp(&self, value: impl Into<T>) -> T {
        let mut value: T = value.into();
        value = self.min.map(|min| min.gt(&value).then_some(min))
            .flatten()
            .map_or(value, |x| x);
        value = self.max.map(|max| max.lt(&value).then_some(max))
            .flatten()
            .map_or(value, |x| x);
        value
    }
}

// Pid methods
impl<T> Pid<T>
where
    T: Number
{
    /// Creates a new controller
    ///
    /// To set your P, I, and D gains into this controller, please use the following builder methods:
    /// - [Self::p()]: Proportional gain setting
    /// - [Self::i()]: Integral gain setting
    /// - [Self::d()]: Derivative gain setting
    ///
    /// To set output limits please use the [Self::limit()] method.
    pub const fn new() -> Self {
        Self {
            setpoint: None,
            kp: None,
            ki: None,
            kd: None,
            prev: None,
            p_limit: PidLimit::<T>::new(),
            i_limit: PidLimit::<T>::new(),
            d_limit: PidLimit::<T>::new(),
            out_limit: PidLimit::<T>::new(),
        }
    }

    /// Sets the [Self::kp] gain for this controller.
    pub fn p(&mut self, gain: impl Into<T>) -> &mut Self {
        self.kp = Some(gain.into());
        self
    }

    /// Sets the [Self::ki] gain for this controller.
    pub fn i(&mut self, gain: impl Into<T>) -> &mut Self {
        self.ki = Some(gain.into());
        self
    }

    /// Sets the [Self::kd] gain for this controller.
    pub fn d(&mut self, gain: impl Into<T>) -> &mut Self {
        self.kd = Some(gain.into());
        self
    }

    /// Sets the [Pid::setpoint] to target for this controller.
    pub fn setpoint(&mut self, setpoint: impl Into<T>) -> &mut Self {
        self.setpoint = Some(setpoint.into());
        self
    }

    /// Sets the `min` and `max` limits for the controller output.
    ///
    /// Calling this method sets asymmetric limits in the output,
    /// that is, in the final sum of the `p`, `i` and `d` terms.
    ///
    /// This method also sets symmetric limits to each individual term.
    /// The symmetric limits are `-sym` and `+sym`, where `sym` is the
    /// maximum between the absolute values of `min` and `max`.
    ///
    /// When these limits are set they will prevent integral term windup.
    pub fn limit(&mut self, min: impl Into<T>, max: impl Into<T>) -> &mut Self {
        let min: T = min.into();
        let max: T = max.into();
        // Set asymmetric limits
        self.out_limit.set(min, max);
        // Get maximum absolute value
        let sym = if min.abs() > max.abs() {min.abs()}
        else {max.abs()};
        // Set symmetric limits
        self.p_limit.set(-sym, sym);
        self.i_limit.set(-sym, sym);
        self.d_limit.set(-sym, sym);
        self
    }

    /// Set integral term to a custom value. This might be useful to restore the
    /// pid controller to a previous state after an interruption or crash.
    pub fn set_integral_term(&mut self, term: impl Into<T>) -> &mut Self {
        let i_unbound: T = term.into();
        let i = self.i_limit.clamp(i_unbound);
        let out = self.prev.map_or(
            ControlOutput {
                input: T::zero(),
                error: T::zero(),
                p: T::zero(),
                i,
                d: T::zero(),
                output: T::zero(),
            },
            |mut out| {
                out.i = i;
                out
            }
        );
        self.prev = Some(out);
        self
    }

    /// Resets the integral term back to zero, this may drastically change the
    /// control output.
    pub fn reset(&mut self) -> &mut Self {
        self.prev = self.prev.map(|mut out| {
            out.i = T::zero();
            out
        });
        self
    }

    /// Given a new measurement, calculates the next [control output](ControlOutput).
    ///
    /// # Returns None
    /// 
    /// - If a setpoint has not been set via `setpoint()`.
    /// - If no gain is set via `p()`, `i()` or `d()`.
    pub fn update(&mut self, input: impl Into<T>) -> Option<ControlOutput<T>> {
        // Convert parameters to number type
        let input: T = input.into();
        
        // Return early if no setpoint is defined
        let setpoint = match self.setpoint {
            Some(value) => value,
            None => return None,
        };

        // Return early if no gains are defined
        if (self.kp == None) &&
            (self.ki == None) &&
            (self.kd == None)
        { return None };

        // Calculate the error between the ideal setpoint and the current
        // measurement to compare against
        let error = setpoint - input;

        // Calculate proportional if it exists
        let p = self.kp.map_or(
            T::zero(),
            |kp| {
                // Calculate the proportional term and limit to it's individual limit
                let p_unbounded = kp * error;
                self.p_limit.clamp(p_unbounded)
            }
        );

        // Calculate integral if it exists
        let i = self.ki.map_or(
            T::zero(),
            |ki| {
                let i_prev = self.prev.map_or(T::zero(), |out| out.i);
                // Mitigate output jumps when ki(t) != ki(t-1).
                // While it's standard to use an error_integral that's a running sum of
                // just the error (no ki), because we support ki changing dynamically,
                // we store the entire term so that we don't need to remember previous
                // ki values.
                let i_unbounded = (ki * error) + i_prev;
                // Mitigate integral windup: Don't want to keep building up error
                // beyond what i_limit will allow.
                self.i_limit.clamp(i_unbounded)
            }
        );
        
        // Calculate derivative if it exists
        let d = self.kd.map_or(
            T::zero(),
            |kd| self.prev.map_or(
                T::zero(),
                |prev| {
                    // Mitigate derivative kick: Use the derivative of the measurement
                    // rather than the derivative of the error.
                    let d_unbounded = -kd * (input - prev.input);
                    self.d_limit.clamp(d_unbounded)
                }
        ));

        let output = {
            // Calculate the final output by adding together the PID terms, then
            // apply the final defined output limit
            let o_unbounded = p + i + d;
            self.out_limit.clamp(o_unbounded)
        };

        // Register the individual term's contributions and the final output
        self.prev = Some(ControlOutput {
            input,
            error,
            p,
            i,
            d,
            output,
        });

        // return output
        self.prev.clone()
    }

    /// Given a new measurement and a delta time, calculates the next [control output](ControlOutput).
    ///
    /// Param `dt`: Delta Time is a unitary duration value used to adjust integral and derivative terms for any sampling frequency.
    /// If `dt` is assumed to be in seconds, `ki` and `kd` will be represented in 1:1 ratio after 1 second of delay between samples.
    /// Integral term is directly proportional to dt and derivative term is inversely proportional. This means that if dt=2 then integral
    /// term will be ki*2 and derivative term will be kd/2.
    ///
    /// # Returns None
    /// 
    /// - If a setpoint has not been set via `setpoint()`.
    /// - If no gain is set via `p()`, `i()` or `d()`.
    /// - If `dt` <= zero.
    pub fn update_with_dt(&mut self, input: impl Into<T>, dt: impl Into<T>) -> Option<ControlOutput<T>> {
        // Convert parameters to number type
        let dt: T = dt.into();
        // Verify delta time value
        if dt <= T::zero() { return None };
        // Store previous output
        let prev = self.prev.clone();
        // Call normal update
        self.update(input)
            .map(|mut out| {
                // Calculate new integral term with delta time
                let ki = self.ki.map_or(T::zero(), |ki| ki);
                let i_prev = prev.map_or(T::zero(), |out| out.i);
                let i_unbounded = (ki * out.error * dt) + i_prev;
                out.i = self.i_limit.clamp(i_unbounded);
                // Calculate new derivative term with delta time
                let d_unbounded = out.d / dt;
                out.d = self.d_limit.clamp(d_unbounded);
                // Sum new terms into output
                let o_unbounded = out.p + out.i + out.d;
                out.output = self.out_limit.clamp(o_unbounded);
                self.prev = Some(out);
                out
            })
    }
}

#[cfg(test)]
mod tests {
    use super::Pid;
    use crate::ControlOutput;

    /// Proportional-only controller operation and limits
    #[test]
    fn proportional() {
        let mut pid = Pid::new()
            .setpoint(10.0)
            .limit(-100.0, 100.0)
            .p(2.0);
        
        assert_eq!(pid.setpoint, 10.0);

        // Test simple proportional
        assert_eq!(pid.update(0.0).unwrap().output, 20.0);

        // Test proportional limit
        pid.p_limit.set(-10.0, 10.0);
        assert_eq!(pid.update(0.0).unwrap().output,10.0);
    }

    /// Derivative-only controller operation and limits
    #[test]
    fn derivative() {
        let mut pid = Pid::new()
            .setpoint(10.0)
            .limit(-100.0, 100.0)
            .d(2.0);

        // Test that there's no derivative since it's the first measurement
        assert_eq!(pid.update(0.0).unwrap().output, 0.0);

        // Test that there's now a derivative
        assert_eq!(pid.update(5.0).unwrap().output, -10.0);

        // Test derivative limit
        pid.d_limit.set(-5.0, 5.0);
        assert_eq!(pid.update(10.0).unwrap().output, -5.0);
    }

    /// Integral-only controller operation and limits
    #[test]
    fn integral() {
        let mut pid = Pid::new()
            .setpoint(10.0)
            .limit(-100.0, 100.0)
            .i(2.0);

        // Test basic integration
        assert_eq!(pid.update(0.0).unwrap().output, 20.0);
        assert_eq!(pid.update(0.0).unwrap().output, 40.0);
        assert_eq!(pid.update(5.0).unwrap().output, 50.0);

        // Test limit
        pid.i_limit.set(-50.0, 50.0);
        assert_eq!(pid.update(5.0).unwrap().output, 50.0);
        // Test that limit doesn't impede reversal of error integral
        assert_eq!(pid.update(15.0).unwrap().output, 40.0);

        // Test that error integral accumulates negative values
        let mut pid2 = Pid::new()
            .setpoint(-10.0)
            .limit(-100.0, 100.0)
            .i(2.0);

        assert_eq!(pid2.update(0.0).unwrap().output, -20.0);
        assert_eq!(pid2.update(0.0).unwrap().output, -40.0);

        pid2.i_limit.set(-50.0, 50.0);
        assert_eq!(pid2.update(-5.0).unwrap().output, -50.0);
        // Test that limit doesn't impede reversal of error integral
        assert_eq!(pid2.update(-15.0).unwrap().output, -40.0);
    }

    /// Checks that a full PID controller's limits work properly through multiple output iterations
    #[test]
    fn output_limit() {
        let mut pid = Pid::new()
            .setpoint(10.0)
            .limit(-100.0, 100.0)
            .p(1.0);

        pid.out_limit.set(-1.0, 1.0);

        let out = pid.update(0.0).unwrap();
        assert_eq!(out.p, 10.0); // 1.0 * 10.0
        assert_eq!(out.output, 1.0);

        let out = pid.update(20.0).unwrap();
        assert_eq!(out.p, -10.0); // 1.0 * (10.0 - 20.0)
        assert_eq!(out.output, -1.0);
    }

    /// Combined PID operation
    #[test]
    fn pid() {
        let mut pid = Pid::new()
            .setpoint(10.0)
            .limit(-100.0, 100.0)
            .p(1.0)
            .i(0.1)
            .d(1.0);

        let out = pid.update(0.0).unwrap();
        assert_eq!(out.p, 10.0); // 1.0 * 10.0
        assert_eq!(out.i, 1.0); // 0.1 * 10.0
        assert_eq!(out.d, 0.0); // -(1.0 * 0.0)
        assert_eq!(out.output, 11.0);

        let out = pid.update(5.0).unwrap();
        assert_eq!(out.p, 5.0); // 1.0 * 5.0
        assert_eq!(out.i, 1.5); // 0.1 * (10.0 + 5.0)
        assert_eq!(out.d, -5.0); // -(1.0 * 5.0)
        assert_eq!(out.output, 1.5);

        let out = pid.update(11.0).unwrap();
        assert_eq!(out.p, -1.0); // 1.0 * -1.0
        assert_eq!(out.i, 1.4); // 0.1 * (10.0 + 5.0 - 1)
        assert_eq!(out.d, -6.0); // -(1.0 * 6.0)
        assert_eq!(out.output, -5.6);

        let out = pid.update(10.0).unwrap();
        assert_eq!(out.p, 0.0); // 1.0 * 0.0
        assert_eq!(out.i, 1.4); // 0.1 * (10.0 + 5.0 - 1.0 + 0.0)
        assert_eq!(out.d, 1.0); // -(1.0 * -1.0)
        assert_eq!(out.output, 2.4);
    }

    // NOTE: use for new test in future: /// Full PID operation with mixed float checking to make sure they're equal
    /// PID operation with zero'd values, checking to see if different floats equal each other
    #[test]
    fn floats_zeros() {
        let mut pid_f32 = Pid::new()
            .setpoint(10.0f32)
            .limit(-100.0, 100.0)
            .p(0.0);

        let mut pid_f64 = Pid::new()
            .setpoint(10.0)
            .limit(-100.0f64, 100.0f64)
            .p(0.0);

        for _ in 0..5 {
            assert_eq!(
                pid_f32.update(0.0).unwrap().output,
                pid_f64.update(0.0).unwrap().output as f32
            );
        }
    }

    // NOTE: use for new test in future: /// Full PID operation with mixed signed integer checking to make sure they're equal
    /// PID operation with zero'd values, checking to see if different floats equal each other
    #[test]
    fn signed_integers_zeros() {
        let mut pid_i8 = Pid::new()
            .setpoint(10i8)
            .limit(-100, 100)
            .p(0.0);

        let mut pid_i32 = Pid::new()
            .setpoint(10i32)
            .limit(-100, 100)
            .p(0.0);

        for _ in 0..5 {
            assert_eq!(
                pid_i32.update(0).unwrap().output,
                pid_i8.update(0i8).unwrap().output as i32
            );
        }
    }

    /// See if the controller can properly target to the setpoint after 2 output iterations
    #[test]
    fn setpoint() {
        let mut pid = Pid::new()
            .setpoint(10.0)
            .limit(-100.0, 100.0)
            .p(1.0)
            .i(0.1)
            .d(1.0);

        let out = pid.update(0.0).unwrap();
        assert_eq!(out.p, 10.0); // 1.0 * 10.0
        assert_eq!(out.i, 1.0); // 0.1 * 10.0
        assert_eq!(out.d, 0.0); // -(1.0 * 0.0)
        assert_eq!(out.output, 11.0);

        pid.setpoint(0.0);

        assert_eq!(
            pid.update(0.0).unwrap(),
            ControlOutput {
                input: 0.0,
                error: 10.0,
                p: 0.0,
                i: 1.0,
                d: -0.0,
                output: 1.0
            }
        );
    }

    /// Make sure negative limits don't break the controller
    #[test]
    fn negative_limits() {
        let mut pid = Pid::new()
            .setpoint(10.0f32)
            .limit(50.0, -50.0)
            .p(1.0)
            .i(1.0)
            .d(1.0);

        pid.out_limit.set(10.0, -10.0);

        let out = pid.update(0.0).unwrap();
        assert_eq!(out.p, 10.0);
        assert_eq!(out.i, 10.0);
        assert_eq!(out.d, 0.0);
        assert_eq!(out.output, 10.0);
    }
}
