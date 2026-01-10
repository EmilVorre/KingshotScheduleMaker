use std::collections::{HashMap, HashSet};
use crate::parser::AppointmentEntry;
use super::types::{ScheduledAppointment, DaySchedule};
use super::slot_utils::calculate_slot_rankings;
use super::move_chain::{find_move_chain, apply_move_chain};

/// Schedules appointments for Construction day with smart slot ranking and stealing
/// Prioritizes slot 49 for people who want research and have slot 1 available
pub fn schedule_construction_day(entries: &[AppointmentEntry]) -> DaySchedule {
    // Filter candidates who want construction
    let candidates: Vec<&AppointmentEntry> = entries
        .iter()
        .filter(|e| e.wants_construction && !e.construction_available_slots.is_empty())
        .collect();
    
    // Separate candidates into two groups:
    // 1. Those who want research and have slot 1 available (priority for slot 49)
    // 2. Everyone else
    let mut slot49_priority: Vec<&AppointmentEntry> = candidates
        .iter()
        .filter(|e| {
            e.wants_research && 
            e.research_available_slots.contains(&1) && 
            e.construction_available_slots.contains(&49)
        })
        .copied()
        .collect();
    
    let mut other_candidates: Vec<&AppointmentEntry> = candidates
        .iter()
        .filter(|e| {
            !(e.wants_research && 
              e.research_available_slots.contains(&1) && 
              e.construction_available_slots.contains(&49))
        })
        .copied()
        .collect();
    
    // Sort priority candidates by construction score (highest first)
    slot49_priority.sort_by(|a, b| {
        b.construction_score.cmp(&a.construction_score)
    });
    
    // Sort other candidates by construction score (highest first)
    other_candidates.sort_by(|a, b| {
        b.construction_score.cmp(&a.construction_score)
    });
    
    // Calculate slot rankings
    let available_slots_list: Vec<Vec<u8>> = candidates
        .iter()
        .map(|e| e.construction_available_slots.clone())
        .collect();
    let slot_rankings = calculate_slot_rankings(&available_slots_list);
    
    let mut schedule: HashMap<u8, ScheduledAppointment> = HashMap::new();
    let mut used_slots = HashSet::new();
    let mut unassigned = Vec::new();
    
    // Create a map from player_id to entry for quick lookup
    let entry_map: HashMap<String, &AppointmentEntry> = candidates
        .iter()
        .map(|e| (e.player_id.clone(), *e))
        .collect();
    
    // First, try to assign slot 49 to priority candidates
    let mut slot49_assigned = false;
    for entry in &slot49_priority {
        if entry.construction_available_slots.contains(&49) && !used_slots.contains(&49) {
            schedule.insert(49, ScheduledAppointment {
                player_id: entry.player_id.clone(),
                name: entry.name.clone(),
                alliance: entry.alliance.clone(),
                slot: 49,
                priority_score: entry.construction_score,
            });
            used_slots.insert(49);
            slot49_assigned = true;
            break;
        }
    }
    
    // Combine remaining candidates (priority candidates that didn't get slot 49 + other candidates)
    let mut remaining_candidates: Vec<&AppointmentEntry> = if slot49_assigned {
        // Remove the one who got slot 49 from priority list
        slot49_priority.into_iter()
            .filter(|e| !used_slots.contains(&49) || schedule.get(&49).map(|a| a.player_id != e.player_id).unwrap_or(true))
            .collect()
    } else {
        slot49_priority
    };
    remaining_candidates.extend(other_candidates);
    
    // Sort remaining candidates by construction score
    remaining_candidates.sort_by(|a, b| {
        b.construction_score.cmp(&a.construction_score)
    });
    
    // Schedule the rest using the normal logic
    for entry in remaining_candidates {
        let available_slots = &entry.construction_available_slots;
        
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
                    priority_score: entry.construction_score,
                });
                used_slots.insert(*slot);
                assigned = true;
                break;
            }
        }
        
        // If no free slot, try slot stealing
        if !assigned {
            // Find players in the requested slots, collect their data first
            // For slot 49, we need to consider both construction and research scores
            let mut blocking_players: Vec<(u8, String, u32, u32)> = ranked_slots
                .iter()
                .filter_map(|(slot, _)| {
                    if let Some(appt) = schedule.get(slot) {
                        if let Some(blocking_entry) = entry_map.get(&appt.player_id) {
                            // For slot 49, calculate combined score (construction + research if applicable)
                            let combined_score = if *slot == 49 {
                                let base_score = blocking_entry.construction_score;
                                // Add research score if they want research and have slot 1
                                if blocking_entry.wants_research && blocking_entry.research_available_slots.contains(&1) {
                                    base_score + blocking_entry.research_score
                                } else {
                                    base_score
                                }
                            } else {
                                appt.priority_score
                            };
                            Some((*slot, appt.player_id.clone(), appt.priority_score, combined_score))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            
            // For slot 49, sort by combined score (lowest first)
            // For other slots, sort by priority score (lowest first)
            blocking_players.sort_by(|a, b| {
                if a.0 == 49 || b.0 == 49 {
                    // If either is slot 49, use combined score
                    a.3.cmp(&b.3)
                } else {
                    // Otherwise use priority score
                    a.2.cmp(&b.2)
                }
            });
            
            // Try to steal a slot with depth-limited search (up to 5 levels)
            for (requested_slot, _blocking_player_id, _blocking_score, _combined_score) in &blocking_players {
                // Special handling for slot 49: check if requester has better combined score
                if *requested_slot == 49 {
                    if let Some(blocking_appt) = schedule.get(requested_slot) {
                        if let Some(blocking_entry) = entry_map.get(&blocking_appt.player_id) {
                            // Calculate requester's combined score
                            let requester_combined = if entry.wants_research && entry.research_available_slots.contains(&1) {
                                entry.construction_score + entry.research_score
                            } else {
                                entry.construction_score
                            };
                            
                            // Calculate current holder's combined score
                            let holder_combined = if blocking_entry.wants_research && blocking_entry.research_available_slots.contains(&1) {
                                blocking_entry.construction_score + blocking_entry.research_score
                            } else {
                                blocking_entry.construction_score
                            };
                            
                            // Only try to steal if requester has better (higher) combined score
                            if requester_combined <= holder_combined {
                                continue; // Skip - current holder has better or equal combined score
                            }
                        }
                    }
                }
                
                // Try to find a chain of moves to free up this slot
                if let Some(blocking_appt) = schedule.get(requested_slot) {
                    let blocking_entry = entry_map.get(&blocking_appt.player_id);
                    
                    if let Some(blocking_entry) = blocking_entry {
                        let blocking_available = &blocking_entry.construction_available_slots;
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
                            |e| &e.construction_available_slots,
                            1,
                            5, // max depth of 5
                            &mut visited,
                            &HashSet::new(), // No locked slots for construction
                        ) {
                            // Apply the chain of moves
                            apply_move_chain(&move_chain, &mut schedule, &mut used_slots);
                            
                            // Now assign the freed slot to the current player
                            schedule.insert(*requested_slot, ScheduledAppointment {
                                player_id: entry.player_id.clone(),
                                name: entry.name.clone(),
                                alliance: entry.alliance.clone(),
                                slot: *requested_slot,
                                priority_score: entry.construction_score,
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

