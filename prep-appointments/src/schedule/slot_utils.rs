use std::collections::HashMap;

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
pub fn calculate_slot_rankings(available_slots_list: &[Vec<u8>]) -> HashMap<u8, u32> {
    let mut rankings = HashMap::new();
    for slots in available_slots_list {
        for &slot in slots {
            *rankings.entry(slot).or_insert(0) += 1;
        }
    }
    rankings
}

