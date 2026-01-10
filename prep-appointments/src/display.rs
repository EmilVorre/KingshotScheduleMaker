use std::fs::File;
use std::io::Write;
use crate::parser::AppointmentEntry;
use crate::schedule::DaySchedule;
use crate::schedule::slot_to_time;

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

