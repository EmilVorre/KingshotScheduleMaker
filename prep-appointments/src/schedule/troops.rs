use crate::parser::AppointmentEntry;
use super::DaySchedule;
use super::generic::schedule_day_generic;

/// Schedules appointments for Troops Training day with smart slot ranking and stealing
pub fn schedule_troops_day(entries: &[AppointmentEntry]) -> DaySchedule {
    schedule_day_generic(
        entries,
        |e| e.wants_troops,
        |e| &e.troops_available_slots,
        |e| e.troops_speedups,
    )
}

