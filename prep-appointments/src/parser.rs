use csv::Reader;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Maps a time string to a slot number using custom time slot mapping
/// Returns None if the time string doesn't match any slot in the mapping
fn time_string_to_slot_number(
    time_str: &str,
    time_slots: &[(u8, String)]
) -> Option<u8> {
    // Remove any notes or extra text in parentheses
    let clean_time = time_str.split('(').next().unwrap_or(time_str).trim();
    
    time_slots.iter()
        .find(|(_, time)| time.trim() == clean_time)
        .map(|(slot, _)| *slot)
}

/// Parses a comma-separated list of time strings and converts them to slot numbers
/// If custom_time_slots is provided, uses that mapping; otherwise falls back to fixed mapping
fn parse_time_slots(
    time_string: &str,
    custom_time_slots: Option<&[(u8, String)]>
) -> Vec<u8> {
    let mut slots = HashSet::new();
    
    // Split by comma and process each time
    for time_part in time_string.split(',') {
        let trimmed = time_part.trim();
        let slot = if let Some(custom_slots) = custom_time_slots {
            // Use custom mapping
            time_string_to_slot_number(trimmed, custom_slots)
        } else {
            // Fallback to fixed mapping (backward compatibility)
            time_to_slot(trimmed)
        };
        
        if let Some(slot) = slot {
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

/// Loads appointments from a CSV file
/// 
/// # Arguments
/// * `csv_path` - Path to the CSV file
/// * `construction_time_slots` - Optional mapping of (slot_number, time_string) for construction day
/// * `research_time_slots` - Optional mapping of (slot_number, time_string) for research day
/// * `troops_time_slots` - Optional mapping of (slot_number, time_string) for troops day
/// 
/// If time slot mappings are not provided, falls back to the fixed time mapping (backward compatibility)
pub fn load_appointments<P: AsRef<Path>>(
    csv_path: P,
    construction_time_slots: Option<&[(u8, String)]>,
    research_time_slots: Option<&[(u8, String)]>,
    troops_time_slots: Option<&[(u8, String)]>,
) -> Result<Vec<AppointmentEntry>, Box<dyn std::error::Error>> {
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
        
        let construction_available_slots = parse_time_slots(construction_times, construction_time_slots);
        let research_available_slots = parse_time_slots(research_times, research_time_slots);
        let troops_available_slots = parse_time_slots(troops_times, troops_time_slots);
        
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

