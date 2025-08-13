//! Generic phase-based timer system for managing notifications at different intervals.
//!
//! This module provides a flexible timer that can handle multiple notification phases,
//! each with different trigger times and behaviors (one-time or recurring).

use chrono::{DateTime, Utc};

/// Defines the behavior of a timer phase
#[derive(Debug, Clone)]
pub enum PhaseType {
    /// Phase triggers once at the specified time
    OneTime,
    /// Phase triggers repeatedly with the given interval after the initial trigger
    Recurring { interval: usize },
}

/// A phase in the timer system
#[derive(Debug, Clone)]
pub struct Phase<T> {
    /// The time (in minutes) when this phase becomes active
    pub trigger_time: usize,
    /// How this phase behaves (one-time or recurring)
    pub phase_type: PhaseType,
    /// The action data associated with this phase
    ///
    /// This is generic so that the timer may be used in different contexts.
    pub action: T,
    /// The last time (in minutes from timer start) when this phase triggered
    last_action_time: usize,
}

impl<T> Phase<T> {
    /// Create a one-time phase that triggers at the specified time
    pub fn one_time(trigger_time: usize, action: T) -> Self {
        Self {
            trigger_time,
            phase_type: PhaseType::OneTime,
            action,
            last_action_time: 0,
        }
    }

    /// Create a recurring phase that triggers at the specified time and then repeats
    pub fn recurring(trigger_time: usize, interval: usize, action: T) -> Self {
        Self {
            trigger_time,
            phase_type: PhaseType::Recurring { interval },
            action,
            last_action_time: 0,
        }
    }
}

/// A generic timer that manages multiple phases with different trigger behaviors.
///
/// The timer evaluates all phases on each check and returns the action from the phase
/// with the latest effective trigger time, ensuring proper priority handling when
/// multiple phases could activate simultaneously.
#[derive(Debug, Clone)]
pub struct PhaseTimer<T> {
    phases: Vec<Phase<T>>,
    start_time: DateTime<Utc>,
}

impl<T: Clone> PhaseTimer<T> {
    /// Create a new phase timer with the given phases
    pub fn new(mut phases: Vec<Phase<T>>) -> Self {
        // Sort phases by trigger time to ensure proper ordering
        phases.sort_by_key(|phase| phase.trigger_time);

        Self {
            phases,
            start_time: Utc::now(),
        }
    }

    /// Reset the timer to the beginning
    pub fn reset(&mut self) {
        self.start_time = Utc::now();
        for phase in &mut self.phases {
            phase.last_action_time = 0;
        }
    }

    /// Calculate what action should be taken (if any) at the current time
    pub fn calculate_action(&mut self) -> Option<T> {
        let current_minutes = (Utc::now() - self.start_time).num_minutes() as usize;
        self.calculate_action_at_time(current_minutes)
    }

    /// Calculate what action should be taken (if any) at the specified time
    pub fn calculate_action_at_time(&mut self, current_minutes: usize) -> Option<T> {
        // Find phases whose trigger time is beyond the current time.
        // Then determine which should trigger by selecting the one with the latest effective
        // trigger time (highest priority)
        let triggerable_phases: Vec<(usize, usize, T)> = self
            .phases
            .iter()
            .enumerate()
            .filter(|(_, phase)| current_minutes >= phase.trigger_time)
            .filter_map(|(index, phase)| {
                self.should_trigger_phase(phase, current_minutes)
                    .map(|(trigger_time, action)| (trigger_time, index, action))
            })
            .collect();

        if let Some((_, phase_index, action)) = triggerable_phases
            .into_iter()
            .max_by_key(|(trigger_time, _, _)| *trigger_time)
        {
            self.phases[phase_index].last_action_time = current_minutes;
            Some(action)
        } else {
            None
        }
    }

    /// Check if a phase should trigger at the given time.
    ///
    /// Returns the effective trigger time and action if the phase should activate.
    /// For recurring phases, calculates the most recent occurrence that hasn't been triggered yet.
    fn should_trigger_phase(&self, phase: &Phase<T>, current_minutes: usize) -> Option<(usize, T)> {
        match phase.phase_type {
            PhaseType::OneTime => {
                // One-time phases only trigger if we haven't already triggered them
                if phase.last_action_time < phase.trigger_time {
                    Some((phase.trigger_time, phase.action.clone()))
                } else {
                    None
                }
            }
            PhaseType::Recurring { interval } => {
                let time_since_trigger = current_minutes - phase.trigger_time;
                let expected_occurrences = (time_since_trigger / interval) + 1;
                let last_occurrence_time =
                    phase.trigger_time + (expected_occurrences - 1) * interval;

                // Recurring phases trigger if we haven't already triggered at this occurrence
                if phase.last_action_time < last_occurrence_time {
                    Some((last_occurrence_time, phase.action.clone()))
                } else {
                    None
                }
            }
        }
    }

    /// Get the current elapsed minutes since the timer started
    pub fn elapsed_minutes(&self) -> usize {
        (Utc::now() - self.start_time).num_minutes() as usize
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestAction {
        Initial,
        Reminder,
        Stop,
    }

    #[test]
    fn creates_timer_with_sorted_phases() {
        let phases = vec![
            Phase::one_time(90, TestAction::Initial),
            Phase::recurring(30, 10, TestAction::Reminder),
        ];

        let timer = PhaseTimer::new(phases);

        // Phases should be sorted by trigger time
        assert_eq!(timer.phases[0].trigger_time, 30);
        assert_eq!(timer.phases[1].trigger_time, 90);
    }

    #[test]
    fn no_action_before_first_phase() {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        let action = timer.calculate_action_at_time(45);
        assert_eq!(action, None);
    }

    #[test]
    fn triggers_one_time_phase() {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        let action = timer.calculate_action_at_time(90);
        assert_eq!(action, Some(TestAction::Initial));

        // Should not trigger again
        let action = timer.calculate_action_at_time(95);
        assert_eq!(action, None);
    }

    #[test]
    fn triggers_recurring_phase() {
        let phases = vec![Phase::recurring(90, 10, TestAction::Reminder)];
        let mut timer = PhaseTimer::new(phases);

        // First occurrence
        let action = timer.calculate_action_at_time(90);
        assert_eq!(action, Some(TestAction::Reminder));

        // Should not trigger again until interval passes
        let action = timer.calculate_action_at_time(95);
        assert_eq!(action, None);

        // Second occurrence
        let action = timer.calculate_action_at_time(100);
        assert_eq!(action, Some(TestAction::Reminder));
    }

    #[rstest]
    #[case::at_trigger_time(90)]
    #[case::after_trigger_time(95)]
    fn one_time_phase_triggers_once_regardless_of_check_time(#[case] check_time: usize) {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        let action = timer.calculate_action_at_time(check_time);
        assert_eq!(action, Some(TestAction::Initial));
    }

    #[test]
    fn handles_multiple_phases() {
        let phases = vec![
            Phase::recurring(60, 30, TestAction::Reminder),
            Phase::one_time(120, TestAction::Stop),
        ];
        let mut timer = PhaseTimer::new(phases);

        // First recurring phase
        let action = timer.calculate_action_at_time(60);
        assert_eq!(action, Some(TestAction::Reminder));

        // Second occurrence of recurring phase
        let action = timer.calculate_action_at_time(90);
        assert_eq!(action, Some(TestAction::Reminder));

        // One-time phase
        let action = timer.calculate_action_at_time(120);
        assert_eq!(action, Some(TestAction::Stop));

        // Third occurrence of recurring phase (after one-time phase)
        // The recurring phase should trigger at 125 since it missed its 120 slot due to the
        // one-time phase taking priority
        let action = timer.calculate_action_at_time(125);
        assert_eq!(action, Some(TestAction::Reminder));

        // Fourth occurrence of recurring phase
        let action = timer.calculate_action_at_time(150);
        assert_eq!(action, Some(TestAction::Reminder));
    }

    #[test]
    fn resets_timer_properly() {
        let phases = vec![Phase::one_time(90, TestAction::Initial)];
        let mut timer = PhaseTimer::new(phases);

        // Trigger the phase
        timer.calculate_action_at_time(90);

        // Reset and verify it can trigger again
        let before_reset = Utc::now();
        timer.reset();
        let after_reset = Utc::now();

        assert!(timer.phases.iter().all(|phase| phase.last_action_time == 0));
        assert!(timer.start_time >= before_reset && timer.start_time <= after_reset);

        // Should trigger again after reset
        let action = timer.calculate_action_at_time(90);
        assert_eq!(action, Some(TestAction::Initial));
    }

    #[test]
    fn prioritizes_later_phases_when_multiple_could_trigger() {
        let phases = vec![
            Phase::recurring(60, 30, TestAction::Reminder),
            Phase::one_time(120, TestAction::Stop),
        ];
        let mut timer = PhaseTimer::new(phases);

        // At 120 minutes, both the recurring phase (3rd occurrence) and one-time phase could
        // trigger The one-time phase should win because it has a later effective trigger
        // time
        let action = timer.calculate_action_at_time(120);
        assert_eq!(action, Some(TestAction::Stop));
    }

    #[test]
    fn phases_track_independent_timing() {
        let phases = vec![
            Phase::recurring(60, 10, TestAction::Reminder),
            Phase::recurring(61, 5, TestAction::Stop),
        ];
        let mut timer = PhaseTimer::new(phases);

        // First phase triggers at 60
        let action = timer.calculate_action_at_time(60);
        assert_eq!(action, Some(TestAction::Reminder));

        // Second phase should trigger at 61, even though the first phase just triggered
        // This tests that each phase tracks its own last_action_time independently
        let action = timer.calculate_action_at_time(61);
        assert_eq!(action, Some(TestAction::Stop));
    }
}
