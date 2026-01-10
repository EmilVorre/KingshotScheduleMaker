use std::collections::{HashMap, HashSet};
use crate::parser::AppointmentEntry;
use super::types::{ScheduledAppointment, DaySchedule};
use super::slot_utils::calculate_slot_rankings;
use super::move_chain::{find_move_chain, apply_move_chain};

/// Generic scheduling function with slot ranking and stealing
pub fn schedule_day_generic<F>(
    entries: &[AppointmentEntry],
    wants_filter: F,
    get_available_slots: fn(&AppointmentEntry) -> &Vec<u8>,
    get_priority_score: fn(&AppointmentEntry) -> u32,
) -> DaySchedule
where
    F: Fn(&AppointmentEntry) -> bool,
{
    schedule_day_generic_with_locked_slots(entries, wants_filter, get_available_slots, get_priority_score, &HashSet::new(), &HashSet::new())
}

/// Generic scheduling function with slot ranking and stealing, with pre-locked slots
pub fn schedule_day_generic_with_locked_slots<F>(
    entries: &[AppointmentEntry],
    wants_filter: F,
    get_available_slots: fn(&AppointmentEntry) -> &Vec<u8>,
    get_priority_score: fn(&AppointmentEntry) -> u32,
    pre_locked_slots: &HashSet<u8>,
    locked_slots: &HashSet<u8>,
) -> DaySchedule
where
    F: Fn(&AppointmentEntry) -> bool,
{
    // Filter candidates
    let mut candidates: Vec<&AppointmentEntry> = entries
        .iter()
        .filter(|e| wants_filter(e) && !get_available_slots(e).is_empty())
        .collect();
    
    // Calculate slot rankings (popularity)
    let available_slots_list: Vec<Vec<u8>> = candidates
        .iter()
        .map(|e| get_available_slots(e).clone())
        .collect();
    let slot_rankings = calculate_slot_rankings(&available_slots_list);
    
    // Sort candidates by priority score descending (highest first)
    candidates.sort_by(|a, b| {
        let score_a = get_priority_score(a);
        let score_b = get_priority_score(b);
        score_b.cmp(&score_a)
    });
    
    let mut schedule: HashMap<u8, ScheduledAppointment> = HashMap::new();
    let mut used_slots = pre_locked_slots.clone();
    let mut unassigned = Vec::new();
    
    // Create a map from player_id to entry for quick lookup
    let entry_map: HashMap<String, &AppointmentEntry> = candidates
        .iter()
        .map(|e| (e.player_id.clone(), *e))
        .collect();
    
    for entry in candidates {
        let available_slots = get_available_slots(entry);
        
        // Sort available slots by ranking (highest rank first)
        let mut ranked_slots: Vec<(u8, u32)> = available_slots
            .iter()
            .map(|&slot| (slot, slot_rankings.get(&slot).copied().unwrap_or(0)))
            .collect();
        ranked_slots.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by rank descending
        
        // Try to assign the highest-ranked available slot
        let mut assigned = false;
        for (slot, _rank) in &ranked_slots {
            if !used_slots.contains(slot) {
                // Free slot available
                schedule.insert(*slot, ScheduledAppointment {
                    player_id: entry.player_id.clone(),
                    name: entry.name.clone(),
                    alliance: entry.alliance.clone(),
                    slot: *slot,
                    priority_score: get_priority_score(entry),
                });
                used_slots.insert(*slot);
                assigned = true;
                break;
            }
        }
        
        // If no free slot, try slot stealing
        if !assigned {
            // Find players in the requested slots, collect their data first
            let mut blocking_players: Vec<(u8, String, u32)> = ranked_slots
                .iter()
                .filter_map(|(slot, _)| {
                    schedule.get(slot).map(|appt| (*slot, appt.player_id.clone(), appt.priority_score))
                })
                .collect();
            
            // Sort by priority score (lowest first) - we'll try to move lowest-scoring players first
            blocking_players.sort_by(|a, b| a.2.cmp(&b.2));
            
            // Try to steal a slot with depth-limited search (up to 5 levels)
            for (requested_slot, _blocking_player_id, _blocking_score) in &blocking_players {
                // Try to find a chain of moves to free up this slot
                // We need to check if we can move the player currently in requested_slot
                if let Some(blocking_appt) = schedule.get(requested_slot) {
                    let blocking_entry = entry_map.get(&blocking_appt.player_id);
                    
                    if let Some(blocking_entry) = blocking_entry {
                        let blocking_available = get_available_slots(blocking_entry);
                        let mut visited = HashSet::new();
                        visited.insert(blocking_appt.player_id.clone());
                        
                        // Try to find a chain of moves (depth limit: 5)
                        if let Some(move_chain) = find_move_chain(
                            &blocking_appt.player_id,
                            *requested_slot,
                            blocking_available,
                            &schedule,
                            &used_slots,
                            &entry_map,
                            get_available_slots,
                            1,
                            5, // max depth of 5
                            &mut visited,
                            locked_slots,
                        ) {
                            // Apply the chain of moves
                            apply_move_chain(&move_chain, &mut schedule, &mut used_slots);
                            
                            // Now assign the freed slot to the current player
                            schedule.insert(*requested_slot, ScheduledAppointment {
                                player_id: entry.player_id.clone(),
                                name: entry.name.clone(),
                                alliance: entry.alliance.clone(),
                                slot: *requested_slot,
                                priority_score: get_priority_score(entry),
                            });
                            used_slots.insert(*requested_slot);
                            assigned = true;
                            break;
                        }
                    }
                }
            }
        }
        
        if !assigned {
            unassigned.push(entry.player_id.clone());
        }
    }
    
    DaySchedule {
        appointments: schedule,
        unassigned,
    }
}

