use csv::Reader;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;

mod web;

#[derive(Debug, Clone)]
pub struct AppointmentEntry {
    pub alliance: String,
    pub name: String,
    pub player_id: String,
    pub wants_construction: bool,
    pub wants_research: bool,
    pub wants_troops: bool,
    pub construction_speedups: u32,
    pub research_speedups: u32,
    pub troops_speedups: u32,
    pub construction_truegold: u32,
    pub construction_score: u32,
    pub research_truegold_dust: u32,
    pub research_score: u32,
    pub construction_available_slots: Vec<u8>,
    pub research_available_slots: Vec<u8>,
    pub troops_available_slots: Vec<u8>,
}

/// Converts a time string (e.g., "00:15", "01:45") to a slot number (1-49)
/// Slot 1 = 00:00, Slot 2 = 00:15, Slot 3 = 00:45, then increments by 30 min
fn time_to_slot(time_str: &str) -> Option<u8> {
    // Remove any notes or extra text in parentheses
    let clean_time = time_str.split('(').next().unwrap_or(time_str).trim();
    
    // Handle "00:00" case
    if clean_time == "00:00" {
        return Some(1);
    }
    
    // Parse HH:MM format
    let parts: Vec<&str> = clean_time.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let hours: u32 = parts[0].parse().ok()?;
    let minutes: u32 = parts[1].parse().ok()?;
    
    // Convert to total minutes
    let total_minutes = hours * 60 + minutes;
    
    // Special cases for the first slots
    if total_minutes == 0 {
        return Some(1); // 00:00
    } else if total_minutes == 15 {
        return Some(2); // 00:15
    } else if total_minutes == 45 {
        return Some(3); // 00:45
    }
    
    // For times after 00:45, calculate slot based on 30-minute increments
    // Slot 3 is at 00:45 (45 minutes), so slot 4 should be at 01:15 (75 minutes)
    // The pattern: slot = 3 + ((total_minutes - 45) / 30)
    if total_minutes > 45 {
        let slot = 3 + ((total_minutes - 45) / 30);
        if slot <= 49 {
            return Some(slot as u8);
        }
    }
    
    None
}

/// Parses a comma-separated list of time strings and converts them to slot numbers
fn parse_time_slots(time_string: &str) -> Vec<u8> {
    let mut slots = HashSet::new();
    
    // Split by comma and process each time
    for time_part in time_string.split(',') {
        let trimmed = time_part.trim();
        if let Some(slot) = time_to_slot(trimmed) {
            slots.insert(slot);
        }
    }
    
    let mut result: Vec<u8> = slots.into_iter().collect();
    result.sort();
    result
}

/// Parses a boolean value from various string representations
fn parse_bool(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    lower == "yes" || lower == "true" || lower == "1"
}

/// Parses a number, returning 0 if empty or invalid
fn parse_number(value: &str) -> u32 {
    value.trim().parse().unwrap_or(0)
}

pub fn load_appointments<P: AsRef<Path>>(csv_path: P) -> Result<Vec<AppointmentEntry>, Box<dyn std::error::Error>> {
    let mut reader = Reader::from_path(csv_path)?;
    // Use HashMap to track entries by player_id for handling resubmissions
    let mut entries_map: HashMap<String, AppointmentEntry> = HashMap::new();
    
    // Read the header (which spans multiple lines in this CSV)
    let headers = reader.headers()?;
    
    // Find column indices
    let alliance_col = headers.iter().position(|h| h.contains("alliance")).unwrap_or(1);
    let custom_alliance_col = headers.iter().position(|h| h.contains("Non of the above") && h.contains("type it here")).unwrap_or(2);
    let name_col = headers.iter().position(|h| h.contains("character name")).unwrap_or(3);
    let id_col = headers.iter().position(|h| h.contains("player ID")).unwrap_or(4);
    let submission_type_col = headers.iter().position(|h| h.contains("Is this form")).unwrap_or(5);
    let construction_want_col = headers.iter().position(|h| h.contains("Construction day appointment")).unwrap_or(6);
    let construction_speedups_col = headers.iter().position(|h| h.contains("Construction day") && h.contains("speedups")).unwrap_or(7);
    let construction_truegold_col = headers.iter().position(|h| h.contains("truegold") && !h.contains("dust")).unwrap_or(8);
    let construction_times_col = headers.iter().position(|h| h.contains("Construction day appointment") && h.contains("times")).unwrap_or(9);
    let research_want_col = headers.iter().position(|h| h.contains("Research day appointment") && !h.contains("times")).unwrap_or(10);
    let research_speedups_col = headers.iter().position(|h| h.contains("Research day") && h.contains("speedups")).unwrap_or(11);
    let research_truegold_dust_col = headers.iter().position(|h| h.contains("truegold dust")).unwrap_or(12);
    let research_times_col = headers.iter().position(|h| h.contains("Research day appointment") && h.contains("times")).unwrap_or(13);
    let troops_want_col = headers.iter().position(|h| h.contains("Troops Training day appointment") && !h.contains("times")).unwrap_or(13);
    let troops_speedups_col = headers.iter().position(|h| h.contains("Troops Training day") && h.contains("speedups")).unwrap_or(14);
    let troops_times_col = headers.iter().position(|h| h.contains("Troops Training day appointment") && h.contains("times")).unwrap_or(15);
    
    // Read all records
    for result in reader.records() {
        let record = result?;
        
        if record.len() < 16 {
            continue; // Skip incomplete records
        }
        
        let mut alliance = record.get(alliance_col).unwrap_or("").trim().to_string();
        // If alliance is "Non of the above", use the custom alliance tag instead
        if alliance.to_lowercase().contains("non of the above") || alliance.to_lowercase() == "non" {
            let custom_alliance = record.get(custom_alliance_col).unwrap_or("").trim().to_string();
            if !custom_alliance.is_empty() {
                alliance = custom_alliance;
            }
        }
        let name = record.get(name_col).unwrap_or("").trim().to_string();
        let player_id = record.get(id_col).unwrap_or("").trim().to_string();
        let submission_type = record.get(submission_type_col).unwrap_or("").trim().to_lowercase();
        
        // Skip if essential fields are missing
        if name.is_empty() || player_id.is_empty() {
            continue;
        }
        
        let is_resubmission = submission_type.contains("re-submission") || submission_type.contains("resubmission");
        
        let wants_construction = parse_bool(record.get(construction_want_col).unwrap_or(""));
        let wants_research = parse_bool(record.get(research_want_col).unwrap_or(""));
        let wants_troops = parse_bool(record.get(troops_want_col).unwrap_or(""));
        
        let construction_speedups = parse_number(record.get(construction_speedups_col).unwrap_or(""));
        let research_speedups = parse_number(record.get(research_speedups_col).unwrap_or(""));
        let troops_speedups = parse_number(record.get(troops_speedups_col).unwrap_or(""));
        
        let construction_truegold = parse_number(record.get(construction_truegold_col).unwrap_or(""));
        
        // Calculate construction score: (truegold * 2000) + (speedups * 30)
        let construction_score = (construction_truegold * 2000) + (construction_speedups * 30);
        
        let research_truegold_dust = parse_number(record.get(research_truegold_dust_col).unwrap_or(""));
        
        // Calculate research score: (truegold_dust * 1000) + (speedups * 30)
        let research_score = (research_truegold_dust * 1000) + (research_speedups * 30);
        
        let construction_times = record.get(construction_times_col).unwrap_or("");
        let research_times = record.get(research_times_col).unwrap_or("");
        let troops_times = record.get(troops_times_col).unwrap_or("");
        
        let construction_available_slots = parse_time_slots(construction_times);
        let research_available_slots = parse_time_slots(research_times);
        let troops_available_slots = parse_time_slots(troops_times);
        
        if is_resubmission {
            // Update existing entry if it exists
            if let Some(existing_entry) = entries_map.get_mut(&player_id) {
                // Update all fields with the new values
                existing_entry.alliance = alliance;
                existing_entry.name = name;
                existing_entry.wants_construction = wants_construction;
                existing_entry.wants_research = wants_research;
                existing_entry.wants_troops = wants_troops;
                existing_entry.construction_speedups = construction_speedups;
                existing_entry.research_speedups = research_speedups;
                existing_entry.troops_speedups = troops_speedups;
                existing_entry.construction_truegold = construction_truegold;
                existing_entry.construction_score = construction_score;
                existing_entry.research_truegold_dust = research_truegold_dust;
                existing_entry.research_score = research_score;
                existing_entry.construction_available_slots = construction_available_slots.clone();
                existing_entry.research_available_slots = research_available_slots.clone();
                existing_entry.troops_available_slots = troops_available_slots.clone();
            } else {
                // If no existing entry found, treat it as a new entry (shouldn't happen, but handle gracefully)
                let new_entry = AppointmentEntry {
                    alliance,
                    name,
                    player_id: player_id.clone(),
                    wants_construction,
                    wants_research,
                    wants_troops,
                    construction_speedups,
                    research_speedups,
                    troops_speedups,
                    construction_truegold,
                    construction_score,
                    research_truegold_dust,
                    research_score,
                    construction_available_slots,
                    research_available_slots,
                    troops_available_slots,
                };
                entries_map.insert(player_id, new_entry);
            }
        } else {
            // New submission - add or replace (in case of duplicate new submissions)
            let new_entry = AppointmentEntry {
                alliance,
                name,
                player_id: player_id.clone(),
                wants_construction,
                wants_research,
                wants_troops,
                construction_speedups,
                research_speedups,
                troops_speedups,
                construction_truegold,
                construction_score,
                research_truegold_dust,
                research_score,
                construction_available_slots,
                research_available_slots,
                troops_available_slots,
            };
            entries_map.insert(player_id, new_entry);
        }
    }
    
    // Convert HashMap values to Vec
    let entries: Vec<AppointmentEntry> = entries_map.into_values().collect();
    
    Ok(entries)
}

/// Represents a scheduled appointment for a specific day
#[derive(Debug, Clone)]
pub struct ScheduledAppointment {
    pub player_id: String,
    pub name: String,
    pub alliance: String,
    pub slot: u8,
    pub priority_score: u32,
}

/// Schedule for a single day
#[derive(Debug)]
pub struct DaySchedule {
    pub appointments: HashMap<u8, ScheduledAppointment>, // slot -> appointment
    pub unassigned: Vec<String>, // player IDs that couldn't be assigned
}

/// Converts slot number back to time string for display
pub fn slot_to_time(slot: u8) -> String {
    match slot {
        1 => "00:00".to_string(),
        2 => "00:15".to_string(),
        3 => "00:45".to_string(),
        _ => {
            // Slot 3 is at 00:45 (45 minutes)
            // Slot 4 is at 01:15 (75 minutes)
            // Pattern: total_minutes = 45 + (slot - 3) * 30
            let total_minutes = 45 + (slot as u32 - 3) * 30;
            let hours = total_minutes / 60;
            let minutes = total_minutes % 60;
            format!("{:02}:{:02}", hours, minutes)
        }
    }
}

/// Calculates slot rankings based on how many players requested each slot
/// Returns a HashMap: slot -> request_count (higher count = higher rank/popularity)
fn calculate_slot_rankings(available_slots_list: &[Vec<u8>]) -> HashMap<u8, u32> {
    let mut rankings = HashMap::new();
    for slots in available_slots_list {
        for &slot in slots {
            *rankings.entry(slot).or_insert(0) += 1;
        }
    }
    rankings
}

/// Represents a move in a chain of slot reassignments
#[derive(Debug, Clone)]
struct Move {
    player_id: String,
    from_slot: u8,
    to_slot: u8,
}

/// Tries to find a chain of moves to free up a slot, with depth limit
/// Returns Some(Vec<Move>) if a chain is found, None otherwise
fn find_move_chain(
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
fn apply_move_chain(
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

/// Generic scheduling function with slot ranking and stealing
fn schedule_day_generic<F>(
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
fn schedule_day_generic_with_locked_slots<F>(
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

/// Schedules appointments for Research day with smart slot ranking and stealing
/// The person in slot 49 of construction day must be in slot 1 of research day
pub fn schedule_research_day(entries: &[AppointmentEntry], construction_schedule: &DaySchedule) -> DaySchedule {
    let mut schedule: HashMap<u8, ScheduledAppointment> = HashMap::new();
    let mut used_slots = HashSet::new();
    let mut locked_player_id: Option<String> = None;
    
    // Check if construction day has someone in slot 49
    if let Some(construction_appt) = construction_schedule.appointments.get(&49) {
        let player_id = &construction_appt.player_id;
        
        // Find the entry for this player
        if let Some(entry) = entries.iter().find(|e| e.player_id == *player_id) {
            // Check if they want research and have slot 1 available
            if entry.wants_research && entry.research_available_slots.contains(&1) {
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

/// Schedules appointments for Troops Training day with smart slot ranking and stealing
pub fn schedule_troops_day(entries: &[AppointmentEntry]) -> DaySchedule {
    schedule_day_generic(
        entries,
        |e| e.wants_troops,
        |e| &e.troops_available_slots,
        |e| e.troops_speedups,
    )
}

/// Formats a player name with alliance tag
pub fn format_player_name(alliance: &str, name: &str) -> String {
    if alliance.is_empty() {
        name.to_string()
    } else {
        format!("[{}] {}", alliance, name)
    }
}

/// Writes a day schedule to a file in the format: HH:MM [tag] name
pub fn write_schedule_to_file(
    day_name: &str,
    schedule: &DaySchedule,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(filename)?;
    
    // Write header with day name
    writeln!(file, "** {} **", day_name)?;
    
    // Write all 49 slots, one per line
    for slot in 1..=49 {
        let time = slot_to_time(slot);
        if let Some(appt) = schedule.appointments.get(&slot) {
            let formatted_name = format_player_name(&appt.alliance, &appt.name);
            writeln!(file, "{} {}", time, formatted_name)?;
        } else {
            writeln!(file, "{} [EMPTY]", time)?;
        }
    }
    
    Ok(())
}

/// Prints a day schedule in a readable format
pub fn print_day_schedule<F>(day_name: &str, schedule: &DaySchedule, entries: &[AppointmentEntry], get_priority_score: F)
where
    F: Fn(&AppointmentEntry) -> u32,
{
    println!("\n=== {} Schedule ===", day_name);
    println!("Total appointments scheduled: {}", schedule.appointments.len());
    
    if !schedule.unassigned.is_empty() {
        println!("⚠️  Unassigned players ({}):", schedule.unassigned.len());
        for player_id in &schedule.unassigned {
            if let Some(entry) = entries.iter().find(|e| e.player_id == *player_id) {
                let formatted_name = format_player_name(&entry.alliance, &entry.name);
                let priority_score = get_priority_score(entry);
                println!("  - {} (ID: {}, Priority: {})", formatted_name, player_id, priority_score);
            }
        }
    }
    
    println!("\nSchedule by time slot (all 49 slots):");
    // Show all slots from 1 to 49
    for slot in 1..=49 {
        let time = slot_to_time(slot);
        if let Some(appt) = schedule.appointments.get(&slot) {
            let formatted_name = format_player_name(&appt.alliance, &appt.name);
            println!("  Slot {} ({}) -> {} (ID: {}, Priority: {})", 
                slot, time, formatted_name, appt.player_id, appt.priority_score);
        } else {
            println!("  Slot {} ({}) -> [EMPTY]", slot, time);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if we should run in web mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "web" {
        let port = args.get(2)
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);
        let password = std::env::var("ADMIN_PASSWORD")
            .unwrap_or_else(|_| "admin123".to_string()); // Default password, change this!
        
        println!("Starting web server on port {}...", port);
        println!("Admin password: {}", password);
        println!("Access the site at http://localhost:{}", port);
        
        web::start_server(port, password).await?;
        return Ok(());
    }
    
    // CLI mode (original behavior)
    // Use test data if available, otherwise use the original path
    let csv_path = if std::path::Path::new("data/testData2.csv").exists() {
        "data/testData2.csv"
    } else {
        r"c:\Users\12010\Downloads\SvS Preparation Week for #235 Week 49 (svar) - Formularsvar 1(2).csv"
    };
    
    println!("Loading appointments from CSV...");
    let entries = load_appointments(csv_path)?;
    
    println!("Loaded {} appointment entries (resubmissions merged)", entries.len());
    
    // Verify resubmission handling - check Jobie (37924862) who had a resubmission
    if let Some(jobie) = entries.iter().find(|e| e.player_id == "37924862") {
        println!("\n=== Resubmission Verification ===");
        println!("Jobie (ID: 37924862) - Should have resubmission values:");
        println!("  Construction Speedups: {} hours (should be 2100)", jobie.construction_speedups);
        println!("  Construction Truegold: {} (should be 2500)", jobie.construction_truegold);
        println!("  Construction Score: {}", jobie.construction_score);
    }
    
    // Print first few entries as example
    println!("\n=== Sample Entries ===");
    
    // Debug: Print Bunny's entry specifically
    if let Some(bunny) = entries.iter().find(|e| e.player_id == "39874858") {
        println!("\n=== DEBUG: Bunny Entry ===");
        println!("Name: {}", bunny.name);
        println!("Player ID: {}", bunny.player_id);
        println!("Wants Research: {}", bunny.wants_research);
        println!("Research Available Slots: {:?}", bunny.research_available_slots);
        println!("Research Score: {}", bunny.research_score);
    }
    
    for (i, entry) in entries.iter().take(3).enumerate() {
        println!("\n--- Entry {} ---", i + 1);
        let formatted_name = format_player_name(&entry.alliance, &entry.name);
        println!("Name: {}", formatted_name);
        println!("Player ID: {}", entry.player_id);
        println!("Wants Construction: {}", entry.wants_construction);
        println!("Wants Research: {}", entry.wants_research);
        println!("Wants Troops: {}", entry.wants_troops);
        println!("Construction Speedups: {} hours", entry.construction_speedups);
        println!("Construction Truegold: {}", entry.construction_truegold);
        println!("Construction Score: {}", entry.construction_score);
        println!("Research Speedups: {} hours", entry.research_speedups);
        println!("Research Truegold Dust: {}", entry.research_truegold_dust);
        println!("Research Score: {}", entry.research_score);
        println!("Construction Available Slots: {:?}", entry.construction_available_slots);
        println!("Research Available Slots: {:?}", entry.research_available_slots);
        println!("Troops Available Slots: {:?}", entry.troops_available_slots);
    }
    
    // Run the scheduler
    println!("\n\n=== Running Auto-Scheduler ===");
    
    
    let construction_schedule = schedule_construction_day(&entries);
    let research_schedule = schedule_research_day(&entries, &construction_schedule);
    let troops_schedule = schedule_troops_day(&entries);
    
    print_day_schedule("Construction Day", &construction_schedule, &entries, |e| e.construction_score);
    print_day_schedule("Research Day", &research_schedule, &entries, |e| e.research_score);
    print_day_schedule("Troops Training Day", &troops_schedule, &entries, |e| e.troops_speedups);
    
    // Write schedules to files
    println!("\n=== Writing Schedules to Files ===");
    write_schedule_to_file("Construction Day", &construction_schedule, "schedule_construction.txt")?;
    write_schedule_to_file("Research Day", &research_schedule, "schedule_research.txt")?;
    write_schedule_to_file("Troops Training Day", &troops_schedule, "schedule_troops.txt")?;
    println!("Schedules saved to:");
    println!("  - schedule_construction.txt");
    println!("  - schedule_research.txt");
    println!("  - schedule_troops.txt");
    
    Ok(())
}

