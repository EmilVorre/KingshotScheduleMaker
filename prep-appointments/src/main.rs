mod parser;
mod schedule;
mod display;
mod web;

use parser::load_appointments;
use schedule::{schedule_construction_day, schedule_research_day, schedule_troops_day};
use display::{format_player_name, print_day_schedule, write_schedule_to_file};

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