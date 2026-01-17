use serde::{Deserialize, Serialize};

/// Form submission data structure matching the form fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSubmission {
    pub timestamp: String,
    pub alliance: String,
    pub custom_alliance: Option<String>,
    pub character_name: String,
    pub player_id: String,
    pub submission_type: String, // "New submission" or "Re-Submission"
    pub wants_construction: bool,
    pub construction_speedups: Option<u32>,
    pub construction_truegold: Option<u32>,
    pub construction_time_slots: Vec<u8>,
    pub wants_research: bool,
    pub research_speedups: Option<u32>,
    pub research_truegold_dust: Option<u32>,
    pub research_time_slots: Vec<u8>,
    pub wants_troops: bool,
    pub troops_speedups: Option<u32>,
    pub troops_time_slots: Vec<u8>,
    pub additional_notes: Option<String>,
    pub suggestions: Option<String>,
}

/// Form submission request from frontend
#[derive(Deserialize)]
pub struct FormSubmissionRequest {
    pub alliance: String,
    pub custom_alliance: Option<String>,
    pub character_name: String,
    pub player_id: String,
    pub submission_type: String,
    pub wants_construction: bool,
    pub construction_speedups: Option<u32>,
    pub construction_truegold: Option<u32>,
    pub construction_time_slots: Vec<u8>,
    pub wants_research: bool,
    pub research_speedups: Option<u32>,
    pub research_truegold_dust: Option<u32>,
    pub research_time_slots: Vec<u8>,
    pub wants_troops: bool,
    pub troops_speedups: Option<u32>,
    pub troops_time_slots: Vec<u8>,
    pub additional_notes: Option<String>,
    pub suggestions: Option<String>,
}

/// Validates a form submission
pub fn validate_submission(req: &FormSubmissionRequest) -> Result<(), String> {
    // Validate character name
    if req.character_name.trim().is_empty() {
        return Err("Character name is required".to_string());
    }
    
    // Validate player ID (must be a number)
    if req.player_id.trim().is_empty() {
        return Err("Player ID is required".to_string());
    }
    if !req.player_id.trim().chars().all(|c| c.is_ascii_digit()) {
        return Err("Player ID must contain only digits".to_string());
    }
    
    // Validate submission type
    if req.submission_type != "New submission" && req.submission_type != "Re-Submission" {
        return Err("Invalid submission type".to_string());
    }
    
    // Validate alliance
    if req.alliance.trim().is_empty() {
        return Err("Alliance selection is required".to_string());
    }
    if req.alliance == "Non of the above" && req.custom_alliance.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err("Custom alliance name is required when 'Non of the above' is selected".to_string());
    }
    
    // Validate construction day if selected
    if req.wants_construction {
        if req.construction_time_slots.len() < 5 {
            return Err("Construction day requires at least 5 time slots".to_string());
        }
        // Validate slots are in range 1-49
        for &slot in &req.construction_time_slots {
            if slot < 1 || slot > 49 {
                return Err(format!("Invalid construction time slot: {}", slot));
            }
        }
    }
    
    // Validate research day if selected
    if req.wants_research {
        if req.research_time_slots.len() < 5 {
            return Err("Research day requires at least 5 time slots".to_string());
        }
        for &slot in &req.research_time_slots {
            if slot < 1 || slot > 49 {
                return Err(format!("Invalid research time slot: {}", slot));
            }
        }
    }
    
    // Validate troops day if selected
    if req.wants_troops {
        if req.troops_time_slots.len() < 5 {
            return Err("Troops Training day requires at least 5 time slots".to_string());
        }
        for &slot in &req.troops_time_slots {
            if slot < 1 || slot > 49 {
                return Err(format!("Invalid troops time slot: {}", slot));
            }
        }
    }
    
    // At least one day type must be selected
    if !req.wants_construction && !req.wants_research && !req.wants_troops {
        return Err("At least one day type (Construction, Research, or Troops) must be selected".to_string());
    }
    
    Ok(())
}
