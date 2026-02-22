//! Shared test utilities for integration tests.

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;

use prep_appointments::parser::AppointmentEntry;
use prep_appointments::schedule::types::{DaySchedule, ScheduledAppointment};

/// Creates a minimal valid CSV file for form submissions.
/// Uses headers that match the parser's column detection (contains "alliance", "player ID", etc.)
pub fn create_test_csv(path: &std::path::Path, rows: &[&str]) -> std::io::Result<()> {
    let header = "timestamp,What alliance do you belong to?,If chosen Non of the above please type it here,What is your character name?,What is your player ID?,Is this form a...,Do you want a Construction day appointment?,How many hours of speedups on Construction day?,How much truegold do you plan too spend?,How much tempered truegold do you plan to spend?,What times are you available for your Construction day appointment? (UTC time),Do you want a Research day appointment?,How many hours of speedups on Research day?,How much truegold dust do you plan to spend?,What times are you available for your Research day appointment? (UTC time),Do you want a Troops Training day appointment?,How many hours of speedups on Troops Training day?,What times are you available for your Troops Training day appointment? (UTC time),Please share any additional notes,What suggestions do you have?";
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "{}", header)?;
    for row in rows {
        writeln!(file, "{}", row)?;
    }
    Ok(())
}

/// Creates an AppointmentEntry for testing with sensible defaults.
pub fn make_entry(
    player_id: &str,
    wants_construction: bool,
    wants_research: bool,
    wants_troops: bool,
    construction_slots: Vec<u8>,
    research_slots: Vec<u8>,
    troops_slots: Vec<u8>,
    construction_score: u32,
    research_score: u32,
    troops_speedups: u32,
) -> AppointmentEntry {
    AppointmentEntry {
        alliance: "Test".to_string(),
        name: format!("Player{}", player_id),
        player_id: player_id.to_string(),
        wants_construction,
        wants_research,
        wants_troops,
        construction_speedups: 0,
        research_speedups: 0,
        troops_speedups,
        construction_truegold: 0,
        construction_tempered_truegold: 0,
        construction_score,
        research_truegold_dust: 0,
        research_score,
        construction_available_slots: construction_slots,
        research_available_slots: research_slots,
        troops_available_slots: troops_slots,
    }
}

/// Creates a ScheduledAppointment for testing.
pub fn make_appointment(player_id: &str, slot: u8, priority_score: u32) -> ScheduledAppointment {
    ScheduledAppointment {
        player_id: player_id.to_string(),
        name: format!("Player{}", player_id),
        alliance: "Test".to_string(),
        slot,
        priority_score,
    }
}

/// Creates an empty DaySchedule.
pub fn empty_schedule() -> DaySchedule {
    DaySchedule {
        appointments: HashMap::new(),
        unassigned: vec![],
    }
}

/// Creates a DaySchedule with given appointments.
pub fn schedule_with_appointments(appointments: Vec<(u8, &str, u32)>) -> DaySchedule {
    let appointments: HashMap<u8, ScheduledAppointment> = appointments
        .into_iter()
        .map(|(slot, pid, score)| (slot, make_appointment(pid, slot, score)))
        .collect();
    DaySchedule {
        appointments,
        unassigned: vec![],
    }
}
