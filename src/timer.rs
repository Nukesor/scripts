//! Generic phase-based timer system for managing notifications at different intervals.
//!
//! This module provides a flexible timer that can handle multiple notification phases,
//! each with different trigger times and behaviors (one-time or recurring).

use std::{iter::Peekable, vec::IntoIter};

use chrono::{DateTime, Utc};

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
        }
    }

    /// Reset the timer to the beginning
    pub fn reset(&mut self) {
        self.start_time = Utc::now();

        let phases = self.original_phases.clone();

        // Create an iterator over the phases in the correct order.
        let mut phases = phases.into_iter().peekable();
        // Get the first phase.
        let Some(current_phase) = phases.next() else {
            panic!("Initialized Timer with no phases.")
        };

        self.phases = phases;
        self.current_phase = current_phase;
        self.start_time = Utc::now();
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
            } => {
                // Calculate the next expected trigger time based on the last action
                let next_trigger_minute = if *last_action_minute == 0 {
                    // First trigger - use the phase's trigger time
                    phase.trigger_at_minute
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
        (Utc::now() - self.start_time).num_minutes() as usize
    }

    #[cfg(test)]
    /// Test helper to simulate timer behavior at a specific time
    fn action_at_time(&mut self, minutes: usize) -> Option<T> {
        // Temporarily modify start_time to simulate the specified elapsed time
        let original_start = self.start_time;
        self.start_time = Utc::now() - chrono::Duration::minutes(minutes as i64);

        let result = self.check();

        // Restore original start time
        self.start_time = original_start;
        result
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
}
