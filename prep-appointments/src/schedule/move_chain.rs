use std::collections::{HashMap, HashSet};
use crate::parser::AppointmentEntry;
use super::types::{Move, ScheduledAppointment};

/// Tries to find a chain of moves to free up a slot, with depth limit
/// Returns Some(Vec<Move>) if a chain is found, None otherwise
pub fn find_move_chain(
    player_id: &str,
    current_slot: u8,
    available_slots: &[u8],
    schedule: &HashMap<u8, ScheduledAppointment>,
    used_slots: &HashSet<u8>,
    entry_map: &HashMap<String, &AppointmentEntry>,
    get_available_slots: fn(&AppointmentEntry) -> &Vec<u8>,
    depth: u32,
    max_depth: u32,
    visited: &mut HashSet<String>,
    locked_slots: &HashSet<u8>,
) -> Option<Vec<Move>> {
    if depth > max_depth {
        return None;
    }
    
    // Cannot move from a locked slot
    if locked_slots.contains(&current_slot) {
        return None;
    }
    
    // Try to find a free slot first
    for &slot in available_slots {
        if slot != current_slot && !used_slots.contains(&slot) {
            // Found a free slot - return a single move
            return Some(vec![Move {
                player_id: player_id.to_string(),
                from_slot: current_slot,
                to_slot: slot,
            }]);
        }
    }
    
    // No free slot found, try to create a chain by moving other players
    // Sort available slots by priority (try most popular slots first)
    let mut slot_priorities: Vec<(u8, u32)> = available_slots
        .iter()
        .filter(|&&s| s != current_slot)
        .map(|&slot| {
            // Count how many players want this slot (rough priority)
            let priority = schedule.get(&slot).map(|_| 1).unwrap_or(0);
            (slot, priority)
        })
        .collect();
    slot_priorities.sort_by(|a, b| b.1.cmp(&a.1));
    
    for (target_slot, _) in slot_priorities {
        if let Some(blocking_appt) = schedule.get(&target_slot) {
            let blocking_player_id = &blocking_appt.player_id;
            
            // Cannot move from a locked slot
            if locked_slots.contains(&target_slot) {
                continue;
            }
            
            // Avoid cycles - don't revisit players we've already tried in this chain
            if visited.contains(blocking_player_id) {
                continue;
            }
            
            visited.insert(blocking_player_id.to_string());
            
            // Get the blocking player's available slots
            if let Some(blocking_entry) = entry_map.get(blocking_player_id) {
                let blocking_available = get_available_slots(blocking_entry);
                
                // Recursively try to move the blocking player
                if let Some(mut sub_chain) = find_move_chain(
                    blocking_player_id,
                    target_slot,
                    blocking_available,
                    schedule,
                    used_slots,
                    entry_map,
                    get_available_slots,
                    depth + 1,
                    max_depth,
                    visited,
                    locked_slots,
                ) {
                    // Found a chain! Prepend our move
                    sub_chain.insert(0, Move {
                        player_id: player_id.to_string(),
                        from_slot: current_slot,
                        to_slot: target_slot,
                    });
                    return Some(sub_chain);
                }
            }
            
            visited.remove(blocking_player_id);
        }
    }
    
    None
}

/// Applies a chain of moves to the schedule
/// Moves must be applied in REVERSE order to avoid conflicts where
/// a later move's from_slot is an earlier move's to_slot
pub fn apply_move_chain(
    moves: &[Move],
    schedule: &mut HashMap<u8, ScheduledAppointment>,
    used_slots: &mut HashSet<u8>,
) {
    // Apply moves in reverse order to avoid conflicts
    for mv in moves.iter().rev() {
        if let Some(mut appt) = schedule.remove(&mv.from_slot) {
            // Verify we're moving the correct player
            if appt.player_id == mv.player_id {
                appt.slot = mv.to_slot;
                schedule.insert(mv.to_slot, appt);
                used_slots.remove(&mv.from_slot);
                used_slots.insert(mv.to_slot);
            } else {
                // This shouldn't happen, but if it does, put the appointment back
                schedule.insert(mv.from_slot, appt);
                eprintln!("Warning: Attempted to move wrong player from slot {}", mv.from_slot);
            }
        }
    }
}

