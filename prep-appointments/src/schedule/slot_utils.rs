use std::collections::HashMap;

/// Converts slot number back to time string for display (legacy function for backward compatibility)
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

/// Parses a time string (HH:MM) to minutes since midnight
pub fn parse_time_to_minutes(time_str: &str) -> Option<u32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hours: u32 = parts[0].parse().ok()?;
    let minutes: u32 = parts[1].parse().ok()?;
    if hours >= 24 || minutes >= 60 {
        return None;
    }
    Some(hours * 60 + minutes)
}

/// Formats minutes since midnight to time string (HH:MM)
pub fn minutes_to_time_string(minutes: u32) -> String {
    let hours = minutes / 60;
    let mins = minutes % 60;
    format!("{:02}:{:02}", hours % 24, mins)
}

/// Calculates time slots based on start time, end time, and interval rules
/// Slot 1 = start_time
/// Slot 2 = start_time + 15 minutes
/// Slot 3+ = previous slot + 30 minutes
/// Continues until end_time (or start_time + 24 hours if end_time is None)
pub fn calculate_time_slots(start_time: &str, end_time: Option<&str>) -> Vec<(u8, String)> {
    let start_minutes = parse_time_to_minutes(start_time).unwrap_or(0);
    let end_minutes = if let Some(end) = end_time {
        parse_time_to_minutes(end).unwrap_or(start_minutes + 24 * 60)
    } else {
        start_minutes + 24 * 60
    };
    
    let mut slots = Vec::new();
    let mut current_minutes = start_minutes;
    let mut slot_num = 1u8;
    
    // Slot 1 = start time
    slots.push((slot_num, minutes_to_time_string(current_minutes)));
    slot_num += 1;
    
    // Slot 2 = start + 15 minutes
    current_minutes += 15;
    if current_minutes < end_minutes || (end_minutes < start_minutes && current_minutes < end_minutes + 24 * 60) {
        slots.push((slot_num, minutes_to_time_string(current_minutes % (24 * 60))));
        slot_num += 1;
        
        // Slot 3+ = previous + 30 minutes
        while slot_num <= 200 { // Safety limit
            current_minutes += 30;
            let check_minutes = if end_minutes < start_minutes {
                // Handle wrap-around (e.g., 23:00 to 01:00)
                if current_minutes >= 24 * 60 {
                    current_minutes % (24 * 60)
                } else {
                    current_minutes
                }
            } else {
                current_minutes
            };
            
            // Check if we've reached the end time
            if end_minutes < start_minutes {
                // Wrap-around case
                if check_minutes >= end_minutes && check_minutes < start_minutes {
                    break;
                }
            } else {
                // Normal case
                if check_minutes >= end_minutes {
                    break;
                }
            }
            
            slots.push((slot_num, minutes_to_time_string(check_minutes % (24 * 60))));
            slot_num += 1;
        }
    }
    
    slots
}

/// Calculates slot rankings based on how many players requested each slot
/// Returns a HashMap: slot -> request_count (higher count = higher rank/popularity)
pub fn calculate_slot_rankings(available_slots_list: &[Vec<u8>]) -> HashMap<u8, u32> {
    let mut rankings = HashMap::new();
    for slots in available_slots_list {
        for &slot in slots {
            *rankings.entry(slot).or_insert(0) += 1;
        }
    }
    rankings
}

