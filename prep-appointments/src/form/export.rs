use crate::form::submission::FormSubmission;
use crate::schedule::calculate_time_slots;
use std::path::Path;
use csv::WriterBuilder;
use std::fs::OpenOptions;

/// Exports a single form submission to CSV format compatible with the existing parser
/// 
/// # Arguments
/// * `submission` - The form submission data
/// * `csv_path` - Path to the CSV file
/// * `construction_times` - Tuple of (start_time, end_time) for construction day
/// * `research_times` - Tuple of (start_time, end_time) for research day
/// * `troops_times` - Tuple of (start_time, end_time) for troops day
pub fn export_submission_to_csv(
    submission: &FormSubmission,
    csv_path: &Path,
    construction_times: (&str, Option<&str>),
    research_times: (&str, Option<&str>),
    troops_times: (&str, Option<&str>),
) -> Result<(), Box<dyn std::error::Error>> {
    let file_exists = csv_path.exists();
    
    // Open file in append mode
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_path)?;
    
    // If file is new, we need to write headers first
    if !file_exists {
        drop(file); // Close the file
        
        // Write headers manually (the CSV format has complex multiline headers)
        use std::fs::File;
        use std::io::Write;
        let mut header_file = File::create(csv_path)?;
        writeln!(header_file, "timestamp,What alliance do you belong to? ,\"If chosen \"\"Non of the above\"\" please type it here\",\"What is your character name? \n(Note: Copy your name from your profile and paste it in the answer below.) \n(Pro Tip: Please do not change your character name after filling this form and before Friday of SvS preparation week.)\n\",\"What is your player ID?\n(Note: Your ID must be a number)\n\",Is this form a...,Do you want a Construction day appointment?,\"How many hours of speedups do you plan to use on Construction day? \n(Note: Your response must be in hours. Add together amount of general speedup hours and construction speedup hours you are planning to use on this day.)\n\",How much truegold do you plan too spend?,\"What times are you available for your Construction day appointment? (UTC time)\n(Note: Choose a minimum of 5 times.)\n\",Do you want a Research day appointment?,\"How many hours of speedups do you plan to use on Research day? \n(Note: Your response must be in hours. Add together amount of general speedup hours and research speedup hours you are planning to use during this day.)\n\",How much truegold dust do you plan to spend?,\"What times are you available for your Research day appointment? (UTC time)\n(Note: Choose a minimum of 5 times.)\n\",Do you want a Troops Training day appointment?,\"How many hours of speedups do you plan to use on Troops Training day? \n(Note: Your response must be in hours. Add together amount of general speedup hours and troops training speedup hours you are planning to use during this day.)\n\",\"What times are you available for your Troops Training day appointment? (UTC time)\n(Note: Choose a minimum of 5 times.)\n\",\"Please share any additional notes, clarifications, or comments about your responses on this form.\",What suggestions do you have for improving our state? We value your feedback!")?;
        drop(header_file);
    }
    
    // Now append the record
    let file = OpenOptions::new()
        .append(true)
        .open(csv_path)?;
    
    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    
    // Generate time slots for each day type based on form configuration
    let construction_slots = calculate_time_slots(construction_times.0, construction_times.1);
    let research_slots = calculate_time_slots(research_times.0, research_times.1);
    let troops_slots = calculate_time_slots(troops_times.0, troops_times.1);
    
    // Convert slot numbers to actual time strings from form configuration
    let construction_time_strings: Vec<String> = submission.construction_time_slots.iter()
        .filter_map(|&slot| {
            construction_slots.iter()
                .find(|(s, _)| *s == slot)
                .map(|(_, time)| time.clone())
        })
        .collect();
    let construction_times_str = construction_time_strings.join(", ");
    
    let research_time_strings: Vec<String> = submission.research_time_slots.iter()
        .filter_map(|&slot| {
            research_slots.iter()
                .find(|(s, _)| *s == slot)
                .map(|(_, time)| time.clone())
        })
        .collect();
    let research_times_str = research_time_strings.join(", ");
    
    let troops_time_strings: Vec<String> = submission.troops_time_slots.iter()
        .filter_map(|&slot| {
            troops_slots.iter()
                .find(|(s, _)| *s == slot)
                .map(|(_, time)| time.clone())
        })
        .collect();
    let troops_times_str = troops_time_strings.join(", ");
    
    // Determine alliance value
    let alliance_value = if submission.alliance == "Non of the above" {
        "Non of the above".to_string()
    } else {
        submission.alliance.clone()
    };
    
    let custom_alliance = submission.custom_alliance.clone().unwrap_or_default();
    
    // Prepare all values as strings
    let construction_yes_no = if submission.wants_construction { "Yes" } else { "No" };
    let research_yes_no = if submission.wants_research { "Yes" } else { "No" };
    let troops_yes_no = if submission.wants_troops { "Yes" } else { "No" };
    
    let construction_speedups_str = submission.construction_speedups.map(|v| v.to_string()).unwrap_or_default();
    let construction_truegold_str = submission.construction_truegold.map(|v| v.to_string()).unwrap_or_default();
    let research_speedups_str = submission.research_speedups.map(|v| v.to_string()).unwrap_or_default();
    let research_truegold_dust_str = submission.research_truegold_dust.map(|v| v.to_string()).unwrap_or_default();
    let troops_speedups_str = submission.troops_speedups.map(|v| v.to_string()).unwrap_or_default();
    let additional_notes_str = submission.additional_notes.clone().unwrap_or_default();
    let suggestions_str = submission.suggestions.clone().unwrap_or_default();
    
    wtr.write_record(&[
        &submission.timestamp,
        &alliance_value,
        &custom_alliance,
        &submission.character_name,
        &submission.player_id,
        &submission.submission_type,
        construction_yes_no,
        &construction_speedups_str,
        &construction_truegold_str,
        &construction_times_str,
        research_yes_no,
        &research_speedups_str,
        &research_truegold_dust_str,
        &research_times_str,
        troops_yes_no,
        &troops_speedups_str,
        &troops_times_str,
        &additional_notes_str,
        &suggestions_str,
    ])?;
    
    wtr.flush()?;
    Ok(())
}
