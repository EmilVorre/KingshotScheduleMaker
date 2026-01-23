use std::collections::HashSet;
use crate::parser::AppointmentEntry;
use super::DaySchedule;

/// Schedules appointments for Troops Training day with smart slot ranking and stealing
pub fn schedule_troops_day(entries: &[AppointmentEntry]) -> DaySchedule {
    schedule_troops_day_with_locked(entries, &HashSet::new())
}

/// Schedules appointments for Troops Training day with pre-locked slots
pub fn schedule_troops_day_with_locked(entries: &[AppointmentEntry], pre_locked_slots: &HashSet<u8>) -> DaySchedule {
    use super::generic::schedule_day_generic_with_locked_slots;
    schedule_day_generic_with_locked_slots(
        entries,
        |e| e.wants_troops,
        |e| &e.troops_available_slots,
        |e| e.troops_speedups,
        pre_locked_slots,
        &HashSet::new(), // No locked slots for troops
    )
}

