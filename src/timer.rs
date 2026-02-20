//! Generic phase-based timer system for managing notifications at different intervals.
//!
//! This module provides a flexible timer that can handle multiple notification phases,
//! each with different trigger times and behaviors (one-time or recurring).

use std::{iter::Peekable, vec::IntoIter};

use chrono::{DateTime, Duration, Utc};
use log::info;

/// Defines the behavior of a timer phase
#[derive(Debug, Clone)]
pub enum PhaseType {
    /// Phase triggers once at the specified time
    OneTime { triggered: bool },
    /// Phase triggers repeatedly with the given interval after the initial trigger
    Recurring {
        interval: usize,
        /// The last time when this phase triggered.
        /// Measured in minutes from `PhaseTimer.start_time`
        last_action_minute: usize,
        /// If true, the phase won't trigger at the start time but waits for the first interval
        delayed: bool,
    },
}

/// A phase in the timer system
#[derive(Debug, Clone)]
pub struct Phase<T> {
    /// How this phase behaves (one-time or recurring)
    pub phase_type: PhaseType,
    /// The action data associated with this phase
    ///
    /// This is generic so that the timer may be used in different contexts.
    pub action: T,
    /// The time (in minutes) when this phase becomes active
    pub trigger_at_minute: usize,
}

impl<T> Phase<T> {
    /// Create a one-time phase that triggers at the specified time
    pub fn one_time(trigger_time: usize, action: T) -> Self {
        Self {
            phase_type: PhaseType::OneTime { triggered: false },
            action,
            trigger_at_minute: trigger_time,
        }
    }

    /// Create a recurring phase that triggers at the specified time and then repeats
    pub fn recurring(trigger_time: usize, interval: usize, action: T) -> Self {
        Self {
            phase_type: PhaseType::Recurring {
                interval,
                last_action_minute: 0,
                delayed: false,
            },
            action,
            trigger_at_minute: trigger_time,
        }
    }

    /// Create a delayed recurring phase that waits for the first interval before triggering
    pub fn recurring_delayed(trigger_time: usize, interval: usize, action: T) -> Self {
        Self {
            phase_type: PhaseType::Recurring {
                interval,
                last_action_minute: 0,
                delayed: true,
            },
            action,
            trigger_at_minute: trigger_time,
        }
    }
}

/// A generic timer that can manage multiple successive phases with different behaviors.
///
/// The idea is to allow parterns like this:
/// - Do nothing for 90 minutes
/// - Then notify 2 times in 30 min intervals
/// - The notify every 10 minutes until reset
///
/// There's always only a single phase active, which is the phase with the highest `start_time`.
#[derive(Debug, Clone)]
pub struct PhaseTimer<T> {
    original_phases: Vec<Phase<T>>,
    phases: Peekable<IntoIter<Phase<T>>>,
    current_phase: Phase<T>,
    start_time: DateTime<Utc>,
    last_check_time: Option<DateTime<Utc>>,
}

impl<T: Clone> PhaseTimer<T> {
    /// Create a new phase timer with the given phases
    pub fn new(mut phases: Vec<Phase<T>>) -> Self {
        // Sort phases by trigger time to ensure the correct order.
        phases.sort_by_key(|phase| phase.trigger_at_minute);

        // Make a copy of the phases in case of a reset.
        let original_phases = phases.clone();

        // Create an iterator over the phases in the correct order.
        let mut phases = phases.into_iter().peekable();
        // Get the first phase.
        let Some(current_phase) = phases.next() else {
            panic!("Initialized Timer with no phases.")
        };

        Self {
            original_phases,
            phases,
            current_phase,
            start_time: Utc::now(),
            last_check_time: None,
        }
    }

    /// Reset the timer to the beginning
    pub fn reset(&mut self) {
        self.start_time = Utc::now();

        let phases = self.original_phases.clone();

        // Create an iterator over the phases in the correct order.
        let mut phases = phases.into_iter().peekable();
        let Some(current_phase) = phases.next() else {
            panic!("Initialized Timer with no phases.")
        };

        self.phases = phases;
        self.current_phase = current_phase;
        self.start_time = Utc::now();
        self.last_check_time = None;
    }

    /// Check if a phase should trigger right now.
    ///
    /// If so, the respective action  will be returned.
    pub fn check(&mut self) -> Option<T> {
        let minutes_since_start = self.elapsed_minutes();

        // Trigger the current phase. Do this even if we might switch to the next phase just
        // afterwards.
        if self.should_trigger_current_phase(minutes_since_start) {
            return Some(self.current_phase.action.clone());
        }

        // Check if we should switch to the next phase.
        if let Some(next_phase) = self.phases.peek()
            && minutes_since_start >= next_phase.trigger_at_minute
        {
            self.current_phase = self.phases.next().unwrap();
        }

        None
    }

    /// Check if a phase should trigger right now, with automatic sleep detection and reset.
    ///
    /// If so, the respective action will be returned.
    ///
    /// If more than 30 minutes have passed since the last check, the timer assumes the
    /// machine went to sleep and automatically resets the timer.
    pub fn check_with_sleep_detection(&mut self) -> Option<T> {
        let now = Utc::now();

        // Check for sleep if we have a previous check time
        if let Some(last_check) = self.last_check_time {
            let time_since_check = now - last_check;
            if time_since_check > Duration::minutes(30) {
                info!(
                    "Sleep detected ({}min gap), resetting timer",
                    time_since_check.num_minutes()
                );
                self.reset();
            }
        }

        // Only set the last_check_time in here, as the `check()` call doesn't use this logic.
        self.last_check_time = Some(now);
        self.check()
    }

    /// Check if a phase should trigger at the given time.
    ///
    /// Returns the effective trigger time and action if the phase should activate.
    /// For recurring phases, calculates the most recent occurrence that hasn't been triggered yet.
    fn should_trigger_current_phase(&mut self, minutes_since_start: usize) -> bool {
        let phase = &mut self.current_phase;
        match &mut phase.phase_type {
            // One-time phases trigger once when their trigger time is reached
            PhaseType::OneTime { triggered } => {
                if !*triggered && minutes_since_start >= phase.trigger_at_minute {
                    *triggered = true;
                    true
                } else {
                    false
                }
            }
            // Recurring phases trigger at their initial time and then at regular intervals
            PhaseType::Recurring {
                interval,
                last_action_minute,
                delayed,
            } => {
                // Calculate the next expected trigger time based on the last action
                let next_trigger_minute = if *last_action_minute == 0 {
                    if *delayed {
                        // First trigger for delayed phase - wait for interval after trigger time
                        phase.trigger_at_minute + *interval
                    } else {
                        // First trigger - use the phase's trigger time
                        phase.trigger_at_minute
                    }
                } else {
                    // Subsequent triggers - add interval to last action time
                    *last_action_minute + *interval
                };

                // Check if enough time has passed for the next trigger
                if minutes_since_start >= next_trigger_minute {
                    *last_action_minute = next_trigger_minute;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Get the current elapsed minutes since the timer started
    pub fn elapsed_minutes(&self) -> usize {
        // We clamp to `0` in case UTC::now() is slightly before start_time
        // (Probably happens due to time shift adjustments at boot).
        (Utc::now() - self.start_time).num_minutes().max(0) as usize
    }

    /// Test helper to simulate timer behavior at a specific time
    #[cfg(test)]
    fn action_at_time(&mut self, minutes: usize) -> Option<T> {
        // Temporarily modify start_time to simulate the specified elapsed time
        let original_start = self.start_time;
        self.start_time = Utc::now() - chrono::Duration::minutes(minutes as i64);

        let result = self.check();

        // Restore original start time
        self.start_time = original_start;
        result
    }

    /// Return the current phase
    #[cfg(test)]
    pub fn current_phase(&self) -> &Phase<T> {
        &self.current_phase
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestAction {
        Initial,
        Reminder,
    }

    #[test]
    fn creates_timer_with_sorted_phases() {
        let phases = vec![
            Phase::one_time(90, TestAction::Initial),
            Phase::recurring(30, 10, TestAction::Reminder),
        ];

        let timer = PhaseTimer::new(phases);

        // First phase should be the one with earliest trigger time
        assert_eq!(timer.current_phase.trigger_at_minute, 30);

        // Original phases should be sorted by trigger time
        assert_eq!(timer.original_phases[0].trigger_at_minute, 30);
        assert_eq!(timer.original_phases[1].trigger_at_minute, 90);
    }

    #[test]
    fn no_action_before_first_phase() {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        // Should not trigger before the phase's designated trigger time
        let action = timer.action_at_time(45);
        assert_eq!(action, None);
    }

    #[test]
    fn one_time_phase_triggers_once() {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        let action = timer.action_at_time(90);
        assert_eq!(action, Some(TestAction::Initial));

        // Should not trigger again
        let action = timer.action_at_time(95);
        assert_eq!(action, None);
    }

    #[test]
    fn triggers_recurring_phase() {
        let phases = vec![Phase::recurring(90, 10, TestAction::Reminder)];
        let mut timer = PhaseTimer::new(phases);

        // First occurrence
        let action = timer.action_at_time(90);
        assert_eq!(action, Some(TestAction::Reminder));

        // Should not trigger again until interval passes
        let action = timer.action_at_time(95);
        assert_eq!(action, None);

        // Second occurrence
        let action = timer.action_at_time(100);
        assert_eq!(action, Some(TestAction::Reminder));
    }

    #[test]
    fn resets_timer() {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        // Trigger the phase
        timer.action_at_time(90);

        // Reset and verify it can trigger again
        let before_reset = Utc::now();
        timer.reset();
        let after_reset = Utc::now();

        // After reset, the current phase should be the first one again
        assert_eq!(timer.current_phase.trigger_at_minute, 90);
        assert!(timer.start_time >= before_reset && timer.start_time <= after_reset);

        // Should trigger again after reset
        let action = timer.action_at_time(90);
        assert_eq!(action, Some(TestAction::Initial));
    }

    #[test]
    fn detects_sleep_and_resets_timer() {
        let phases = vec![Phase::recurring(10, 10, TestAction::Reminder)];
        let mut timer = PhaseTimer::new(phases);

        // First trigger at 10 minutes
        let action = timer.action_at_time(10);
        assert_eq!(action, Some(TestAction::Reminder));

        // Simulate normal check at 15 minutes (no action expected)
        timer.last_check_time = Some(Utc::now() - chrono::Duration::minutes(15));
        let action = timer.check_with_sleep_detection();
        assert_eq!(action, None);

        // Simulate sleep: set last_check_time to 35 minutes ago
        timer.last_check_time = Some(Utc::now() - chrono::Duration::minutes(35));

        // This should detect sleep and reset the timer
        let action = timer.check_with_sleep_detection();

        // After reset, we should be at the beginning of the timer
        // No immediate action since we're starting fresh
        assert_eq!(action, None);

        // Verify timer was actually reset by checking the start time is recent
        let minutes_since_start = timer.elapsed_minutes();
        assert!(
            minutes_since_start < 2,
            "Timer should have been reset, but elapsed time is {}",
            minutes_since_start
        );

        // Verify the timer works normally after reset
        let action = timer.action_at_time(10);
        assert_eq!(action, Some(TestAction::Reminder));
    }

    #[test]
    fn delayed_recurring_phase() {
        // Test the dehn-polizei scenario: one-time at 90min, delayed recurring starts at 90min but
        // first triggers at 100min
        let phases = vec![
            Phase::one_time(90, TestAction::Initial),
            Phase::recurring_delayed(90, 10, TestAction::Reminder),
        ];
        let mut timer = PhaseTimer::new(phases);

        // No action before first phase
        assert_eq!(timer.action_at_time(89), None);

        // One-time phase triggers at 90 minutes
        assert_eq!(timer.action_at_time(90), Some(TestAction::Initial));

        // No action between phases - delayed recurring waits for interval
        assert_eq!(timer.action_at_time(95), None);
        assert!(
            matches!(
                timer.current_phase().phase_type,
                PhaseType::Recurring { .. }
            ),
            "We should've entered the recurring phase"
        );

        // Delayed recurring phase first triggers at 100 minutes (90 + 10 interval)
        assert_eq!(timer.action_at_time(100), Some(TestAction::Reminder));

        // No action before next interval
        assert_eq!(timer.action_at_time(105), None);

        // Next recurring trigger at 110 minutes
        assert_eq!(timer.action_at_time(110), Some(TestAction::Reminder));
    }
}
