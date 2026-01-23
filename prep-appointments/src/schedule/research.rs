use std::collections::HashSet;
use crate::parser::AppointmentEntry;
use super::types::ScheduledAppointment;
use super::DaySchedule;
use super::generic::schedule_day_generic_with_locked_slots;

/// Schedules appointments for Research day with smart slot ranking and stealing
/// The person in the last slot of construction day must be in slot 1 of research day
pub fn schedule_research_day(entries: &[AppointmentEntry], construction_schedule: &DaySchedule) -> DaySchedule {
    schedule_research_day_with_locked(entries, construction_schedule, &HashSet::new())
}

/// Schedules appointments for Research day with pre-locked slots
pub fn schedule_research_day_with_locked(entries: &[AppointmentEntry], construction_schedule: &DaySchedule, pre_locked_slots: &HashSet<u8>) -> DaySchedule {
    use std::collections::HashMap;
    
    let mut schedule: HashMap<u8, ScheduledAppointment> = HashMap::new();
    let mut used_slots = pre_locked_slots.clone();
    let mut locked_player_id: Option<String> = None;
    
    // Find the last slot from construction schedule (the highest slot number)
    let last_construction_slot = construction_schedule.appointments.keys().max().copied();
    
    // Check if construction day has someone in the last slot
    // BUT: Don't override slot 1 if it's already predetermined (in pre_locked_slots/used_slots)
    if let Some(last_slot) = last_construction_slot {
        if let Some(construction_appt) = construction_schedule.appointments.get(&last_slot) {
            let player_id = &construction_appt.player_id;
        
            // Find the entry for this player
            if let Some(entry) = entries.iter().find(|e| e.player_id == *player_id) {
                // Check if they want research and have slot 1 available
                // AND slot 1 is not already predetermined/locked
                if entry.wants_research && entry.research_available_slots.contains(&1) && !used_slots.contains(&1) {
                    // Assign them to slot 1 on research day - this is locked and cannot be changed
                    schedule.insert(1, ScheduledAppointment {
                        player_id: entry.player_id.clone(),
                        name: entry.name.clone(),
                        alliance: entry.alliance.clone(),
                        slot: 1,
                        priority_score: entry.research_score,
                    });
                    used_slots.insert(1);
                    locked_player_id = Some(entry.player_id.clone());
                }
            }
        }
    }
    
    // Create locked slots set (slot 1 is locked if someone was assigned)
    let mut locked_slots = HashSet::new();
    if used_slots.contains(&1) {
        locked_slots.insert(1);
    }
    
    // Filter out the locked player from candidates (they're already scheduled)
    // We need to collect into a Vec<AppointmentEntry> by cloning since we need owned values
    let filtered_entries: Vec<AppointmentEntry> = entries
        .iter()
        .filter(|e| {
            if let Some(ref locked_id) = locked_player_id {
                e.player_id != *locked_id
            } else {
                true
            }
        })
        .cloned()
        .collect();
    
    // Schedule the rest using the generic function, with slot 1 already locked
    let remaining_schedule = schedule_day_generic_with_locked_slots(
        &filtered_entries,
        |e| e.wants_research,
        |e| &e.research_available_slots,
        |e| e.research_score,
        &used_slots,
        &locked_slots,
    );
    
    // Merge the locked slot 1 with the remaining schedule
    schedule.extend(remaining_schedule.appointments);
    
    // Combine unassigned lists
    let unassigned = remaining_schedule.unassigned;
    
    DaySchedule {
        appointments: schedule,
        unassigned,
    }
}

