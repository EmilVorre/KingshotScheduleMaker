use actix_web::{web, App, HttpServer, HttpResponse, Result, HttpRequest, middleware, cookie::Key};
use actix_files::Files;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::Path;
use rand::Rng;
use crate::parser::{load_appointments, AppointmentEntry};
use crate::schedule::{schedule_construction_day, schedule_research_day, schedule_troops_day, DaySchedule, slot_to_time, calculate_time_slots};
use crate::schedule::types::ScheduledAppointment;
use crate::display::format_player_name;
use crate::form::{FormSubmissionRequest, FormSubmission, validate_submission, export_submission_to_csv};
use std::collections::HashSet;

// Account structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_name: String,
    pub server_number: u32,
    pub password: String,
    pub in_game_name: String,
}

// Schedule data for an account/server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleData {
    pub construction_schedule: Option<DaySchedule>,
    pub research_schedule: Option<DaySchedule>,
    pub troops_schedule: Option<DaySchedule>,
    pub entries: Option<Vec<AppointmentEntry>>,
}

// Admin configuration for form settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayTimeConfig {
    pub start_time: String, // Format: "HH:MM" (e.g., "00:20")
    pub end_time: Option<String>, // Format: "HH:MM", defaults to start_time + 24 hours if None
}

// Predetermined slot assignment - locks a specific time slot to a player
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredeterminedSlot {
    pub day: String, // "construction", "research", or "troops"
    pub time: String, // Time string like "00:20"
    pub alliance: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub alliances: Vec<String>, // List of alliance names (admin must input, no defaults)
    pub construction_times: DayTimeConfig,
    pub research_times: DayTimeConfig,
    pub troops_times: DayTimeConfig,
    #[serde(default)]
    pub predetermined_slots: Vec<PredeterminedSlot>, // Predetermined slot assignments
    #[serde(default)]
    pub intro_text: Option<String>, // Optional introduction text displayed at the top of the form
}

impl Default for FormConfig {
    fn default() -> Self {
        FormConfig {
            alliances: vec![], // No default alliances - admin must input them
            construction_times: DayTimeConfig {
                start_time: "00:00".to_string(),
                end_time: None,
            },
            research_times: DayTimeConfig {
                start_time: "00:00".to_string(),
                end_time: None,
            },
            troops_times: DayTimeConfig {
                start_time: "00:00".to_string(),
                end_time: None,
            },
            predetermined_slots: vec![], // No predetermined slots by default
            intro_text: None, // No intro text by default
        }
    }
}

// Form data structure - stores form configuration with code and account info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormData {
    pub code: String, // 12-character alphanumeric code
    pub account_name: String,
    pub server_number: u32,
    pub name: String, // Form name (e.g., "Week 1 Form", "January 2025 Form")
    pub created_at: String, // ISO 8601 timestamp when form was created
    pub config: FormConfig,
}

// App state with account-based storage
pub struct AppState {
    pub accounts: Mutex<HashMap<String, Account>>, // key: account_name
    pub schedules: Mutex<HashMap<String, ScheduleData>>, // key: account_name:server_number
    pub forms: Mutex<HashMap<String, FormData>>, // key: form_code (12-char alphanumeric)
    pub current_forms: Mutex<HashMap<String, String>>, // key: account_name:server_number -> form_code
    pub data_dir: String,
}

// Account creation request
#[derive(Deserialize)]
pub struct CreateAccountRequest {
    account_name: String,
    server_number: u32,
    password: String,
    in_game_name: String,
}

#[derive(Serialize)]
pub struct CreateAccountResponse {
    success: bool,
    message: String,
    schedule_url: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    account_name: Option<String>,
    password: String,
}

#[derive(Serialize)]
pub struct ServerInfo {
    account_name: String,
    server_number: u32,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    alliance_counts: HashMap<String, AllianceStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_slot_popularity: Option<HashMap<String, TimeSlotStats>>, // Deprecated, kept for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    construction_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    troops_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    construction_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    troops_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AllianceStats {
    construction_requests: u32,
    research_requests: u32,
    troops_requests: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimeSlotStats {
    construction_requests: u32,
    research_requests: u32,
    troops_requests: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FormTimeSlotStats {
    requests: u32,
}

#[derive(Serialize)]
pub struct ScheduleResponse {
    day_name: String,
    appointments: Vec<ScheduleSlot>,
}

#[derive(Serialize)]
pub struct ScheduleSlot {
    time: String,
    player: Option<String>,
    is_empty: bool,
}

// Helper function to load accounts from file
fn load_accounts(data_dir: &str) -> HashMap<String, Account> {
    let accounts_path = format!("{}/accounts.json", data_dir);
    if Path::new(&accounts_path).exists() {
        if let Ok(content) = std::fs::read_to_string(&accounts_path) {
            if let Ok(accounts) = serde_json::from_str::<HashMap<String, Account>>(&content) {
                return accounts;
            }
        }
    }
    HashMap::new()
}

// Helper function to save accounts to file
fn save_accounts(data_dir: &str, accounts: &HashMap<String, Account>) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let accounts_path = format!("{}/accounts.json", data_dir);
    let content = serde_json::to_string_pretty(accounts)?;
    std::fs::write(&accounts_path, content)?;
    Ok(())
}

// Helper function to get schedule key
fn schedule_key(account_name: &str, server_number: u32) -> String {
    format!("{}:{}", account_name, server_number)
}

// Helper function to get the current form for an account/server
fn get_current_form(forms: &HashMap<String, FormData>, current_forms: &HashMap<String, String>, account_name: &str, server_number: u32) -> Option<FormData> {
    let account_name_lower = account_name.to_lowercase();
    let key = schedule_key(&account_name_lower, server_number);
    if let Some(form_code) = current_forms.get(&key) {
        forms.get(form_code).cloned()
    } else {
        // Fallback: get most recent form by created_at (case-insensitive account_name comparison)
        forms.values()
            .filter(|f| f.account_name.to_lowercase() == account_name_lower && f.server_number == server_number)
            .max_by_key(|f| &f.created_at)
            .cloned()
    }
}

// Helper function to load current forms mapping
fn load_current_forms(data_dir: &str) -> HashMap<String, String> {
    let path = format!("{}/current_forms_map.json", data_dir);
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mapping) = serde_json::from_str::<HashMap<String, String>>(&content) {
                return mapping;
            }
        }
    }
    HashMap::new()
}

// Helper function to save current forms mapping
fn save_current_forms(data_dir: &str, current_forms: &HashMap<String, String>) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let path = format!("{}/current_forms_map.json", data_dir);
    let content = serde_json::to_string_pretty(current_forms)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// Helper function to save schedule to disk
fn save_schedule(data_dir: &str, account_name: &str, server_number: u32, schedule_data: &ScheduleData) -> std::io::Result<()> {
    let schedules_dir = format!("{}/schedules/{}", data_dir, account_name);
    std::fs::create_dir_all(&schedules_dir)?;
    let path = format!("{}/{}.json", schedules_dir, server_number);
    let content = serde_json::to_string_pretty(schedule_data)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// Helper function to load schedule from disk
fn load_schedule(data_dir: &str, account_name: &str, server_number: u32) -> Option<ScheduleData> {
    let path = format!("{}/schedules/{}/{}.json", data_dir, account_name, server_number);
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<ScheduleData>(&content) {
                Ok(schedule_data) => return Some(schedule_data),
                Err(e) => {
                    eprintln!("Failed to deserialize schedule from {}: {}", path, e);
                    return None;
                }
            }
        } else {
            eprintln!("Failed to read schedule file: {}", path);
        }
    }
    None
}

// Helper function to save statistics to disk
fn save_statistics(data_dir: &str, account_name: &str, server_number: u32, stats: &StatsResponse) -> std::io::Result<()> {
    let stats_dir = format!("{}/statistics/{}", data_dir, account_name);
    std::fs::create_dir_all(&stats_dir)?;
    let path = format!("{}/{}.json", stats_dir, server_number);
    let content = serde_json::to_string_pretty(stats)?;
    std::fs::write(&path, content)?;
    Ok(())
}

// Helper function to load statistics from disk
fn load_statistics(data_dir: &str, account_name: &str, server_number: u32) -> Option<StatsResponse> {
    let path = format!("{}/statistics/{}/{}.json", data_dir, account_name, server_number);
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<StatsResponse>(&content) {
                Ok(stats) => return Some(stats),
                Err(e) => {
                    eprintln!("Failed to deserialize statistics from {}: {}", path, e);
                    return None;
                }
            }
        } else {
            eprintln!("Failed to read statistics file: {}", path);
        }
    }
    None
}

// Generate a unique 12-character alphanumeric code
fn generate_form_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// Helper function to load all forms from current_forms folder
fn load_forms(data_dir: &str) -> HashMap<String, FormData> {
    let current_forms_dir = format!("{}/current_forms", data_dir);
    let mut forms = HashMap::new();
    
    if !Path::new(&current_forms_dir).exists() {
        // Try to migrate old forms.json if it exists
        let old_forms_path = format!("{}/forms.json", data_dir);
        if Path::new(&old_forms_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&old_forms_path) {
                if let Ok(old_forms) = serde_json::from_str::<HashMap<String, FormData>>(&content) {
                    // Create directories
                    std::fs::create_dir_all(&current_forms_dir).ok();
                    std::fs::create_dir_all(format!("{}/old_forms", data_dir)).ok();
                    
                    // Move each form to current_forms
                    for (code, mut form_data) in old_forms {
                        // Add default name and created_at if missing (for old forms)
                        if form_data.name.is_empty() {
                            form_data.name = format!("Form {} {}", form_data.account_name, form_data.server_number);
                        }
                        if form_data.created_at.is_empty() {
                            form_data.created_at = chrono::Local::now().to_rfc3339();
                        }
                        
                        // Save account/server info before cloning
                        let account_name = form_data.account_name.clone();
                        let server_number = form_data.server_number;
                        
                        let form_path = format!("{}/{}.json", current_forms_dir, code);
                        if let Ok(content) = serde_json::to_string_pretty(&form_data) {
                            std::fs::write(&form_path, content).ok();
                            forms.insert(code.clone(), form_data.clone());
                            
                            // Try to move old CSV file if it exists
                            let old_csv_path = format!("{}/{}_{}_form_submissions.csv", data_dir, account_name, server_number);
                            if Path::new(&old_csv_path).exists() {
                                let new_csv_path = format!("{}/{}_submissions.csv", current_forms_dir, code);
                                std::fs::copy(&old_csv_path, &new_csv_path).ok();
                                // Keep old CSV for now (don't delete during migration)
                            }
                        }
                    }
                    // Delete old forms.json after migration
                    std::fs::remove_file(&old_forms_path).ok();
                }
            }
        }
        return forms;
    }
    
    // Load all JSON files from current_forms directory
    if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(form_data) = serde_json::from_str::<FormData>(&content) {
                            forms.insert(form_data.code.clone(), form_data);
                        }
                    }
                }
            }
        }
    }
    
    forms
}

// Helper function to save a single form to current_forms folder
fn save_form(data_dir: &str, form_data: &FormData) -> std::io::Result<()> {
    let current_forms_dir = format!("{}/current_forms", data_dir);
    std::fs::create_dir_all(&current_forms_dir)?;
    let form_path = format!("{}/{}.json", current_forms_dir, form_data.code);
    let content = serde_json::to_string_pretty(form_data)?;
    std::fs::write(&form_path, content)?;
    Ok(())
}

// Helper function to move old forms to old_forms folder (including CSV files)
fn archive_old_forms(data_dir: &str, account_name: &str, server_number: u32) -> std::io::Result<()> {
    let current_forms_dir = format!("{}/current_forms", data_dir);
    let old_forms_dir = format!("{}/old_forms", data_dir);
    std::fs::create_dir_all(&old_forms_dir)?;
    
    // Find all forms for this account/server
    if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(form_data) = serde_json::from_str::<FormData>(&content) {
                            // Check if this form belongs to the account/server being updated
                            if form_data.account_name == account_name && form_data.server_number == server_number {
                                let code = &form_data.code;
                                
                                // Move JSON file to old_forms
                                let old_form_json_path = format!("{}/{}_{}_{}.json", old_forms_dir, account_name, server_number, timestamp);
                                std::fs::copy(entry.path(), &old_form_json_path)?;
                                std::fs::remove_file(entry.path())?;
                                
                                // Move CSV file if it exists
                                let csv_file_name = format!("{}_submissions.csv", code);
                                let csv_path = format!("{}/{}", current_forms_dir, csv_file_name);
                                if Path::new(&csv_path).exists() {
                                    let old_csv_path = format!("{}/{}_{}_{}_submissions.csv", old_forms_dir, account_name, server_number, timestamp);
                                    std::fs::copy(&csv_path, &old_csv_path)?;
                                    std::fs::remove_file(&csv_path)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

// Create account endpoint
async fn create_account(
    req: web::Json<CreateAccountRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let account_name = req.account_name.trim().to_lowercase();
    
    if account_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CreateAccountResponse {
            success: false,
            message: "Account name cannot be empty".to_string(),
            schedule_url: None,
        }));
    }
    
    // Check if account already exists
    let mut accounts = state.accounts.lock().unwrap();
    if accounts.contains_key(&account_name) {
        return Ok(HttpResponse::BadRequest().json(CreateAccountResponse {
            success: false,
            message: "Account name already exists".to_string(),
            schedule_url: None,
        }));
    }
    
    // Create new account
    let account = Account {
        account_name: account_name.clone(),
        server_number: req.server_number,
        password: req.password.clone(),
        in_game_name: req.in_game_name.clone(),
    };
    
    accounts.insert(account_name.clone(), account);
    save_accounts(&state.data_dir, &accounts).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save account: {}", e))
    })?;
    
    // Initialize schedule data
    let mut schedules = state.schedules.lock().unwrap();
    let key = schedule_key(&account_name, req.server_number);
    schedules.insert(key, ScheduleData {
        construction_schedule: None,
        research_schedule: None,
        troops_schedule: None,
        entries: None,
    });
    drop(schedules);
    
    let schedule_url = format!("/{}/{}", account_name, req.server_number);
    
    Ok(HttpResponse::Ok().json(CreateAccountResponse {
        success: true,
        message: "Account created successfully".to_string(),
        schedule_url: Some(schedule_url),
    }))
}

// Account login endpoint (for upload authentication)
async fn account_login(
    path: web::Path<(String, u32)>,
    req: web::Json<LoginRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();
    
    let accounts = state.accounts.lock().unwrap();
    if let Some(account) = accounts.get(&account_name) {
        if account.password == req.password {
        Ok(HttpResponse::Ok().json(serde_json::json!({"success": true})))
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({"success": false, "error": "Invalid password"})))
        }
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"success": false, "error": "Account not found"})))
    }
}

// CSV upload endpoint
async fn account_upload(
    path: web::Path<(String, u32)>,
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();
    
    // Check password from header
    let password = req
        .headers()
        .get("X-Password")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    // Verify account and password
    let accounts = state.accounts.lock().unwrap();
    let account = accounts.get(&account_name)
        .ok_or_else(|| actix_web::error::ErrorNotFound("Account not found"))?;
    
    if account.password != password || account.server_number != server_number {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({"success": false, "error": "Unauthorized"})));
    }
    drop(accounts);

    // Save uploaded CSV
    std::fs::create_dir_all(&state.data_dir)?;
    let csv_path = format!("{}/{}_{}.csv", state.data_dir, account_name, server_number);
    std::fs::write(&csv_path, &body)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save file: {}", e)))?;

    // Process the CSV
    // For uploaded CSV files, use None for time slot mappings to maintain backward compatibility
    // (uploaded CSVs might use the old fixed time format)
    match load_appointments(&csv_path, None, None, None) {
        Ok(entries) => {
            let construction_schedule = schedule_construction_day(&entries);
            let research_schedule = schedule_research_day(&entries, &construction_schedule);
            let troops_schedule = schedule_troops_day(&entries);

            // Update state
            let mut schedules = state.schedules.lock().unwrap();
            let key = schedule_key(&account_name, server_number);
            schedules.insert(key, ScheduleData {
                construction_schedule: Some(construction_schedule),
                research_schedule: Some(research_schedule),
                troops_schedule: Some(troops_schedule),
                entries: Some(entries),
            });

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Schedule generated successfully"
            })))
        }
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to process CSV: {}", e)
        })))
    }
}

// Stats endpoint
async fn get_stats(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();
    let key = schedule_key(&account_name, server_number);
    
    // Try to load cached statistics from disk first
    if let Some(cached_stats) = load_statistics(&state.data_dir, &account_name, server_number) {
        return Ok(HttpResponse::Ok().json(cached_stats));
    }
    
        let mut alliance_counts: HashMap<String, AllianceStats> = HashMap::new();
        let mut time_slot_popularity: HashMap<String, TimeSlotStats> = HashMap::new();

    // Separate time slot popularity maps for each day
    let mut construction_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>> = None;
    let mut research_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>> = None;
    let mut troops_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>> = None;
    let mut construction_start_time: Option<String> = None;
    let mut research_start_time: Option<String> = None;
    let mut troops_start_time: Option<String> = None;
    
    // First, try to load from form submissions CSV (this is the source of truth)
    // First try to find current form and use its CSV, otherwise try old location for migration
    let form_csv_path = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        if let Some(current_form) = get_current_form(&forms, &current_forms, &account_name, server_number) {
            // Use new location: current_forms/{code}_submissions.csv
            drop(current_forms);
            format!("{}/current_forms/{}_submissions.csv", state.data_dir, current_form.code)
        } else {
            drop(current_forms);
            // Fallback to old location for migration
            format!("{}/{}_{}_form_submissions.csv", state.data_dir, account_name, server_number)
        }
    };
    
    if Path::new(&form_csv_path).exists() {
        // Try to get form config to use custom time slots
        let form_config = {
            let forms = state.forms.lock().unwrap();
            let current_forms = state.current_forms.lock().unwrap();
            get_current_form(&forms, &current_forms, &account_name, server_number)
                .map(|f| f.config.clone())
        };
        
        let (construction_slots, research_slots, troops_slots) = if let Some(config) = &form_config {
            construction_start_time = Some(config.construction_times.start_time.clone());
            research_start_time = Some(config.research_times.start_time.clone());
            troops_start_time = Some(config.troops_times.start_time.clone());
            (
                Some(calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref())),
                Some(calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref())),
                Some(calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref())),
            )
        } else {
            (None, None, None)
        };
        
        // Initialize separate time slot popularity maps if we have form config
        if let (Some(ref cs), Some(ref rs), Some(ref ts)) = (&construction_slots, &research_slots, &troops_slots) {
            let mut cons_map = HashMap::new();
            for (_, time) in cs {
                cons_map.insert(time.clone(), FormTimeSlotStats { requests: 0 });
            }
            construction_time_slot_popularity = Some(cons_map);
            
            let mut res_map = HashMap::new();
            for (_, time) in rs {
                res_map.insert(time.clone(), FormTimeSlotStats { requests: 0 });
            }
            research_time_slot_popularity = Some(res_map);
            
            let mut troops_map = HashMap::new();
            for (_, time) in ts {
                troops_map.insert(time.clone(), FormTimeSlotStats { requests: 0 });
            }
            troops_time_slot_popularity = Some(troops_map);
        }
        
        // Create slot-to-time maps for efficient lookup
        let construction_slot_to_time: HashMap<u8, String> = construction_slots.as_ref()
            .map(|slots| slots.iter().map(|(s, t)| (*s, t.clone())).collect())
            .unwrap_or_default();
        let research_slot_to_time: HashMap<u8, String> = research_slots.as_ref()
            .map(|slots| slots.iter().map(|(s, t)| (*s, t.clone())).collect())
            .unwrap_or_default();
        let troops_slot_to_time: HashMap<u8, String> = troops_slots.as_ref()
            .map(|slots| slots.iter().map(|(s, t)| (*s, t.clone())).collect())
            .unwrap_or_default();
        
        if let Ok(form_entries) = load_appointments(
            &form_csv_path,
            construction_slots.as_ref().map(|v| v.as_slice()),
            research_slots.as_ref().map(|v| v.as_slice()),
            troops_slots.as_ref().map(|v| v.as_slice()),
        ) {
            for entry in form_entries {
                // Count by alliance
                let stats = alliance_counts.entry(entry.alliance.clone()).or_insert_with(|| AllianceStats {
                    construction_requests: 0,
                    research_requests: 0,
                    troops_requests: 0,
                });
                
                if entry.wants_construction {
                    stats.construction_requests += 1;
                }
                if entry.wants_research {
                    stats.research_requests += 1;
                }
                if entry.wants_troops {
                    stats.troops_requests += 1;
                }
                
                // Count time slot popularity for construction (separate map)
                if let Some(ref mut cons_map) = construction_time_slot_popularity {
                    for slot in &entry.construction_available_slots {
                        if let Some(time) = construction_slot_to_time.get(slot) {
                            if let Some(slot_stats) = cons_map.get_mut(time) {
                                slot_stats.requests += 1;
                            }
                        }
                    }
                }
                
                // Count time slot popularity for research (separate map)
                if let Some(ref mut res_map) = research_time_slot_popularity {
                    for slot in &entry.research_available_slots {
                        if let Some(time) = research_slot_to_time.get(slot) {
                            if let Some(slot_stats) = res_map.get_mut(time) {
                                slot_stats.requests += 1;
                            }
                        }
                    }
                }
                
                // Count time slot popularity for troops (separate map)
                if let Some(ref mut troops_map) = troops_time_slot_popularity {
                    for slot in &entry.troops_available_slots {
                        if let Some(time) = troops_slot_to_time.get(slot) {
                            if let Some(slot_stats) = troops_map.get_mut(time) {
                                slot_stats.requests += 1;
                            }
                        }
                    }
                }
                
                // Also maintain backward-compatible combined map
                for slot in &entry.construction_available_slots {
                    let time = if let Some(ref slots) = construction_slots {
                        slots.iter().find(|(s, _)| *s == *slot).map(|(_, t)| t.clone()).unwrap_or_else(|| slot_to_time(*slot))
                    } else {
                        slot_to_time(*slot)
                    };
                    let slot_stats = time_slot_popularity.entry(time.clone()).or_insert_with(|| TimeSlotStats {
                        construction_requests: 0,
                        research_requests: 0,
                        troops_requests: 0,
                    });
                    slot_stats.construction_requests += 1;
                }
                
                for slot in &entry.research_available_slots {
                    let time = if let Some(ref slots) = research_slots {
                        slots.iter().find(|(s, _)| *s == *slot).map(|(_, t)| t.clone()).unwrap_or_else(|| slot_to_time(*slot))
                    } else {
                        slot_to_time(*slot)
                    };
                    let slot_stats = time_slot_popularity.entry(time).or_insert_with(|| TimeSlotStats {
                        construction_requests: 0,
                        research_requests: 0,
                        troops_requests: 0,
                    });
                    slot_stats.research_requests += 1;
                }
                
                for slot in &entry.troops_available_slots {
                    let time = if let Some(ref slots) = troops_slots {
                        slots.iter().find(|(s, _)| *s == *slot).map(|(_, t)| t.clone()).unwrap_or_else(|| slot_to_time(*slot))
                    } else {
                        slot_to_time(*slot)
                    };
                    let slot_stats = time_slot_popularity.entry(time).or_insert_with(|| TimeSlotStats {
                        construction_requests: 0,
                        research_requests: 0,
                        troops_requests: 0,
                    });
                    slot_stats.troops_requests += 1;
                }
            }
        }
    } else {
        // Fallback: If no form CSV exists, try to load from uploaded CSV (if exists in memory)
        // This is for backward compatibility with old CSV uploads
        let schedules = state.schedules.lock().unwrap();
        if let Some(schedule_data) = schedules.get(&key) {
            if let Some(ref entries) = schedule_data.entries {
        for entry in entries {
            // Count by alliance
            let stats = alliance_counts.entry(entry.alliance.clone()).or_insert_with(|| AllianceStats {
                construction_requests: 0,
                research_requests: 0,
                troops_requests: 0,
            });
            
            if entry.wants_construction {
                stats.construction_requests += 1;
            }
            if entry.wants_research {
                stats.research_requests += 1;
            }
            if entry.wants_troops {
                stats.troops_requests += 1;
            }

            // Count time slot popularity
            for slot in &entry.construction_available_slots {
                let time = slot_to_time(*slot);
                let slot_stats = time_slot_popularity.entry(time.clone()).or_insert_with(|| TimeSlotStats {
                    construction_requests: 0,
                    research_requests: 0,
                    troops_requests: 0,
                });
                slot_stats.construction_requests += 1;
            }

            for slot in &entry.research_available_slots {
                let time = slot_to_time(*slot);
                let slot_stats = time_slot_popularity.entry(time).or_insert_with(|| TimeSlotStats {
                    construction_requests: 0,
                    research_requests: 0,
                    troops_requests: 0,
                });
                slot_stats.research_requests += 1;
            }

            for slot in &entry.troops_available_slots {
                let time = slot_to_time(*slot);
                let slot_stats = time_slot_popularity.entry(time).or_insert_with(|| TimeSlotStats {
                    construction_requests: 0,
                    research_requests: 0,
                    troops_requests: 0,
                });
                slot_stats.troops_requests += 1;
            }
                }
            }
        }
        drop(schedules);
    }
    
    // Build final response
    let stats_response = StatsResponse {
        alliance_counts: alliance_counts.clone(),
        time_slot_popularity: if time_slot_popularity.is_empty() { None } else { Some(time_slot_popularity.clone()) },
        construction_start_time,
        research_start_time,
        troops_start_time,
        construction_time_slot_popularity,
        research_time_slot_popularity,
        troops_time_slot_popularity,
    };
    
    // Save statistics to disk
    if let Err(e) = save_statistics(&state.data_dir, &account_name, server_number, &stats_response) {
        eprintln!("Warning: Failed to save statistics to disk: {}", e);
    }
    
    Ok(HttpResponse::Ok().json(stats_response))
    }


// Schedule endpoint
async fn get_schedule(
    path: web::Path<(String, u32, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number, day_str) = path.into_inner();
    let account_name = account_name.to_lowercase();
    let key = schedule_key(&account_name, server_number);
    
    // Try to load from disk first
    if let Some(schedule_data) = load_schedule(&state.data_dir, &account_name, server_number) {
        // Also update in-memory cache
        let mut schedules = state.schedules.lock().unwrap();
        schedules.insert(key.clone(), schedule_data.clone());
        drop(schedules);
        
        // Get form config for custom time slots
        let form_config = {
            let forms = state.forms.lock().unwrap();
            let current_forms = state.current_forms.lock().unwrap();
            get_current_form(&forms, &current_forms, &account_name, server_number)
                .map(|f| f.config.clone())
        };
        
        // Get the requested day's schedule
        let schedule = match day_str.as_str() {
            "construction" => schedule_data.construction_schedule.clone(),
            "research" => schedule_data.research_schedule.clone(),
            "troops" => schedule_data.troops_schedule.clone(),
        _ => return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid day"}))),
    };

        if let Some(schedule) = schedule {
            // Generate time slots based on form config or use fixed mapping
            let time_slots: Vec<(u8, String)> = match (day_str.as_str(), form_config.as_ref()) {
                ("construction", Some(config)) => {
                    calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref())
                },
                ("research", Some(config)) => {
                    calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref())
                },
                ("troops", Some(config)) => {
                    calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref())
                },
                _ => {
                    // Fallback to fixed mapping
                    (1..=49).map(|slot| (slot, slot_to_time(slot))).collect()
                }
            };
            
            // Build response with appointments
            let mut appointments = Vec::new();
            for (slot, time) in time_slots {
                if let Some(appt) = schedule.appointments.get(&slot) {
                    appointments.push(ScheduleSlot {
                        time,
                        player: Some(format_player_name(&appt.alliance, &appt.name)),
                        is_empty: false,
                    });
                } else {
                    appointments.push(ScheduleSlot {
                        time,
                        player: None,
                        is_empty: true,
                    });
                }
            }
            
            let day_name = match day_str.as_str() {
            "construction" => "Construction Day",
            "research" => "Research Day",
            "troops" => "Troops Training Day",
                _ => "Unknown Day",
        };

            return Ok(HttpResponse::Ok().json(ScheduleResponse {
                day_name: day_name.to_string(),
                appointments,
            }));
        }
    }
    
    // If not found on disk, get form config for this account/server to get custom time slots
    let form_config = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        get_current_form(&forms, &current_forms, &account_name, server_number)
            .map(|f| f.config.clone())
    };
    
    // Generate time slots based on form config or use fixed mapping
    let time_slots: Vec<(u8, String)> = match (day_str.as_str(), form_config.as_ref()) {
        ("construction", Some(config)) => {
            calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref())
        },
        ("research", Some(config)) => {
            calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref())
        },
        ("troops", Some(config)) => {
            calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref())
        },
        _ => {
            // Fallback to fixed mapping (backward compatibility for uploaded CSVs)
            (1..=49).map(|slot| (slot, slot_to_time(slot))).collect()
        }
    };
    
    let day_name = match day_str.as_str() {
        "construction" => "Construction Day",
        "research" => "Research Day",
        "troops" => "Troops Training Day",
        _ => return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid day"}))),
    };
    
    // Check if schedule exists in memory
    let schedule_opt = {
        let schedules = state.schedules.lock().unwrap();
        if let Some(schedule_data) = schedules.get(&key) {
            match day_str.as_str() {
                "construction" => schedule_data.construction_schedule.as_ref().cloned(),
                "research" => schedule_data.research_schedule.as_ref().cloned(),
                "troops" => schedule_data.troops_schedule.as_ref().cloned(),
                _ => None,
            }
        } else {
            None
        }
    };
    
    // If schedule doesn't exist, try to regenerate from form submissions CSV
    let schedule = if let Some(s) = schedule_opt {
        s
    } else {
        // Try to load from form submissions CSV and regenerate schedules
        // First try to find current form and use its CSV, otherwise try old location for migration
        let form_csv_path = {
            let forms = state.forms.lock().unwrap();
            let current_forms = state.current_forms.lock().unwrap();
            if let Some(current_form) = get_current_form(&forms, &current_forms, &account_name, server_number) {
                // Use new location: current_forms/{code}_submissions.csv
                drop(current_forms);
                format!("{}/current_forms/{}_submissions.csv", state.data_dir, current_form.code)
            } else {
                drop(current_forms);
                // Fallback to old location for migration
                format!("{}/{}_{}_form_submissions.csv", state.data_dir, account_name, server_number)
            }
        };
        
        if Path::new(&form_csv_path).exists() {
            let config_for_loading = form_config.clone();
            let (construction_slots, research_slots, troops_slots) = if let Some(config) = &config_for_loading {
                (
                    Some(calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref())),
                    Some(calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref())),
                    Some(calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref())),
                )
            } else {
                (None, None, None)
            };
            
            if let Ok(entries) = load_appointments(
                &form_csv_path,
                construction_slots.as_ref().map(|v| v.as_slice()),
                research_slots.as_ref().map(|v| v.as_slice()),
                troops_slots.as_ref().map(|v| v.as_slice()),
            ) {
                // Generate schedules
                let construction_schedule = schedule_construction_day(&entries);
                let research_schedule = schedule_research_day(&entries, &construction_schedule);
                let troops_schedule = schedule_troops_day(&entries);
                
                // Create schedule data
                let schedule_data = ScheduleData {
                    construction_schedule: Some(construction_schedule.clone()),
                    research_schedule: Some(research_schedule.clone()),
                    troops_schedule: Some(troops_schedule.clone()),
                    entries: Some(entries.clone()),
                };
                
                // Save to state
                let mut schedules = state.schedules.lock().unwrap();
                schedules.insert(key.clone(), schedule_data.clone());
                drop(schedules);
                
                // Save to disk
                if let Err(e) = save_schedule(&state.data_dir, &account_name, server_number, &schedule_data) {
                    eprintln!("Warning: Failed to save schedule to disk: {}", e);
                }
                
                // Return the appropriate schedule
                match day_str.as_str() {
                    "construction" => construction_schedule,
                    "research" => research_schedule,
                    "troops" => troops_schedule,
                    _ => return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid day"}))),
                }
            } else {
                // No form submissions or error loading, return empty schedule
                DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                }
            }
        } else {
            // No form submissions CSV, return empty schedule
            DaySchedule {
                appointments: HashMap::new(),
                unassigned: Vec::new(),
            }
        }
    };
    
    // Build response with appointments
        let mut appointments = Vec::new();
    for (slot, time) in time_slots {
            if let Some(appt) = schedule.appointments.get(&slot) {
                let formatted_name = format_player_name(&appt.alliance, &appt.name);
                appointments.push(ScheduleSlot {
                    time,
                    player: Some(formatted_name),
                    is_empty: false,
                });
            } else {
                appointments.push(ScheduleSlot {
                    time,
                    player: None,
                    is_empty: true,
                });
            }
        }

        Ok(HttpResponse::Ok().json(ScheduleResponse {
            day_name: day_name.to_string(),
            appointments,
        }))
}

// HTML page handlers - account creation page
async fn create_account_page() -> Result<HttpResponse> {
    let html = include_str!("../templates/create_account.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// HTML page handlers - schedules page
async fn schedules_page(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    
    // Verify account exists
    let accounts = state.accounts.lock().unwrap();
    let account_name_lower = account_name.to_lowercase();
    if !accounts.contains_key(&account_name_lower) {
        return Ok(HttpResponse::NotFound().body("Account not found"));
    }
    drop(accounts);
    
    let html = include_str!("../templates/schedules.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// HTML page handlers - stats page
async fn stats_page(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    
    // Verify account exists
    let accounts = state.accounts.lock().unwrap();
    let account_name_lower = account_name.to_lowercase();
    if !accounts.contains_key(&account_name_lower) {
        return Ok(HttpResponse::NotFound().body("Account not found"));
    }
    drop(accounts);
    
    let html = include_str!("../templates/stats.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// HTML page handlers - admin page
async fn admin_page(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    
    // Verify account exists
    let accounts = state.accounts.lock().unwrap();
    let account_name_lower = account_name.to_lowercase();
    if !accounts.contains_key(&account_name_lower) {
        return Ok(HttpResponse::NotFound().body("Account not found"));
    }
    drop(accounts);
    
    let html = include_str!("../templates/admin.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Public form page - accessible via /form/{code}
async fn public_form_page(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    
    // Verify form exists
    let forms = state.forms.lock().unwrap();
    if !forms.contains_key(&code) {
        drop(forms);
        return Ok(HttpResponse::NotFound().body("Form not found"));
    }
    drop(forms);
    
    let html = include_str!("../templates/form.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Form submission endpoint (by form code)
async fn submit_form_by_code(
    path: web::Path<String>,
    req: web::Json<FormSubmissionRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    
    // Verify form exists and get config
    let forms = state.forms.lock().unwrap();
    let form_data = forms.get(&code).cloned();
    drop(forms);
    
    let config = if let Some(fd) = form_data {
        fd.config
    } else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form not found"
        })));
    };
    
    // Validate submission
    if let Err(err) = validate_submission(&req) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": err
        })));
    }
    
    // Create form submission with timestamp (format: DD/MM/YYYY HH.MM.SS)
    let timestamp = chrono::Local::now().format("%d/%m/%Y %H.%M.%S").to_string();
    let submission = FormSubmission {
        timestamp,
        alliance: req.alliance.clone(),
        custom_alliance: req.custom_alliance.clone(),
        character_name: req.character_name.clone(),
        player_id: req.player_id.clone(),
        submission_type: req.submission_type.clone(),
        wants_construction: req.wants_construction,
        construction_speedups: req.construction_speedups,
        construction_truegold: req.construction_truegold,
        construction_time_slots: req.construction_time_slots.clone(),
        wants_research: req.wants_research,
        research_speedups: req.research_speedups,
        research_truegold_dust: req.research_truegold_dust,
        research_time_slots: req.research_time_slots.clone(),
        wants_troops: req.wants_troops,
        troops_speedups: req.troops_speedups,
        troops_time_slots: req.troops_time_slots.clone(),
        additional_notes: req.additional_notes.clone(),
        suggestions: req.suggestions.clone(),
    };
    
    // Export to CSV (save in current_forms folder with form code)
    let current_forms_dir = format!("{}/current_forms", state.data_dir);
    std::fs::create_dir_all(&current_forms_dir)?;
    let csv_path = format!("{}/{}_submissions.csv", current_forms_dir, code);
    let csv_path = Path::new(&csv_path);
    
    if let Err(e) = export_submission_to_csv(
        &submission,
        csv_path,
        (&config.construction_times.start_time, config.construction_times.end_time.as_deref()),
        (&config.research_times.start_time, config.research_times.end_time.as_deref()),
        (&config.troops_times.start_time, config.troops_times.end_time.as_deref()),
    ) {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save submission: {}", e)
        })));
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Form submitted successfully"
    })))
}

// Create form endpoint (admin only)
#[derive(Deserialize)]
pub struct CreateFormRequest {
    pub name: Option<String>, // Optional form name
    pub alliances: Vec<String>,
    pub construction_times: DayTimeConfig,
    pub research_times: DayTimeConfig,
    pub troops_times: DayTimeConfig,
    #[serde(default)]
    pub predetermined_slots: Vec<PredeterminedSlot>, // Predetermined slot assignments
    #[serde(default)]
    pub intro_text: Option<String>, // Optional introduction text
}

async fn create_form(
    path: web::Path<(String, u32)>,
    session: Session,
    body: web::Json<CreateFormRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();
    
    // Verify session authentication
    let session_account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    let session_server_number: u32 = match session.get("server_number") {
        Ok(Some(num)) => num,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    
    // Verify account name and server number match session
    if session_account_name.to_lowercase() != url_account_name || session_server_number != server_number {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }
    
    // Verify account exists
    let accounts = state.accounts.lock().unwrap();
    if !accounts.contains_key(&url_account_name) {
        drop(accounts);
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Account not found"
        })));
    }
    drop(accounts);
    
    // Validate alliances (must have at least one)
    if body.alliances.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "At least one alliance must be specified"
        })));
    }
    
    // Generate unique code - check both in-memory forms and files on disk
    let mut code = generate_form_code();
    let mut max_attempts = 100; // Prevent infinite loop
    loop {
        // Check in-memory forms
        let forms = state.forms.lock().unwrap();
        let in_memory = forms.contains_key(&code);
        drop(forms);
        
        // Check if file exists on disk (current_forms folder)
        let current_forms_file = format!("{}/current_forms/{}.json", state.data_dir, code);
        let file_exists = Path::new(&current_forms_file).exists();
        
        // Check if file exists in old_forms folder (scan all subdirectories)
        let old_forms_dir = format!("{}/old_forms", state.data_dir);
        let mut old_file_exists = false;
        if Path::new(&old_forms_dir).exists() {
            if let Ok(entries) = std::fs::read_dir(&old_forms_dir) {
                for entry in entries.flatten() {
                    if let Ok(entry_path) = entry.path().canonicalize() {
                        if entry_path.is_dir() {
                            let old_form_file = entry_path.join(format!("{}.json", code));
                            if old_form_file.exists() {
                                old_file_exists = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        if !in_memory && !file_exists && !old_file_exists {
            // Code is unique, break the loop
            break;
        }
        
        // Code collision detected, generate new one
        code = generate_form_code();
        max_attempts -= 1;
        if max_attempts <= 0 {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate unique form code after multiple attempts. Please try again."
            })));
        }
    }
    
    // Create form data
    let mut alliances = body.alliances.clone();
    if !alliances.contains(&"Non of the above".to_string()) {
        alliances.push("Non of the above".to_string());
    }
    
    let config = FormConfig {
        alliances,
        construction_times: body.construction_times.clone(),
        research_times: body.research_times.clone(),
        troops_times: body.troops_times.clone(),
        predetermined_slots: body.predetermined_slots.clone(),
        intro_text: body.intro_text.clone(),
    };
    
    let form_name = body.name.clone().unwrap_or_else(|| {
        format!("Form {} {}", url_account_name, server_number)
    });
    let created_at = chrono::Utc::now().to_rfc3339();
    
    let form_data = FormData {
        code: code.clone(),
        account_name: url_account_name.clone(),
        server_number,
        name: form_name,
        created_at,
        config: FormConfig {
            alliances: body.alliances.clone(),
            construction_times: body.construction_times.clone(),
            research_times: body.research_times.clone(),
            troops_times: body.troops_times.clone(),
            predetermined_slots: body.predetermined_slots.clone(),
            intro_text: body.intro_text.clone(),
        },
    };
    
    // Archive old forms for this account/server before creating new one
    archive_old_forms(&state.data_dir, &url_account_name, server_number)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to archive old forms: {}", e)))?;
    
    // Save new form
    let mut forms = state.forms.lock().unwrap();
    // Remove old forms for this account/server from memory
    forms.retain(|_, fd| !(fd.account_name == url_account_name && fd.server_number == server_number));
    forms.insert(code.clone(), form_data.clone());
    drop(forms);
    
    save_form(&state.data_dir, &form_data)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save form: {}", e)))?;
    
    // Update current forms mapping
    let mut current_forms = state.current_forms.lock().unwrap();
    let key = format!("{}:{}", url_account_name, server_number);
    current_forms.insert(key, code.clone());
    save_current_forms(&state.data_dir, &current_forms)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save current forms mapping: {}", e)))?;
    drop(current_forms);
    
    // Build form URL - use relative path since we don't have HttpRequest
    let form_url = format!("/form/{}", code);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Form created successfully",
        "code": code,
        "url": form_url
    })))
}

// Get form config by code (public)
async fn get_form_config_by_code(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    
    let forms = state.forms.lock().unwrap();
    if let Some(form_data) = forms.get(&code) {
        let config = form_data.config.clone();
        drop(forms);
        Ok(HttpResponse::Ok().json(config))
    } else {
        drop(forms);
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form not found"
        })))
    }
}

// Get form statistics by code (public - shows only time slot popularity)
#[derive(Serialize)]
pub struct FormStatsResponse {
    construction_start_time: String,
    research_start_time: String,
    troops_start_time: String,
    construction_time_slot_popularity: HashMap<String, FormTimeSlotStats>,
    research_time_slot_popularity: HashMap<String, FormTimeSlotStats>,
    troops_time_slot_popularity: HashMap<String, FormTimeSlotStats>,
}

async fn get_form_stats_by_code(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    
    // Get form data to find account_name and server_number, and get config
    let forms = state.forms.lock().unwrap();
    let form_data = forms.get(&code).cloned();
    drop(forms);
    
    let config = if let Some(fd) = form_data {
        fd.config
    } else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form not found"
        })));
    };
    
    // Read form submissions CSV file (using form code)
    let current_forms_dir = format!("{}/current_forms", state.data_dir);
    let csv_path = format!("{}/{}_submissions.csv", current_forms_dir, code);
    
    // Generate time slots for each day type based on form configuration
    let construction_slots = calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref());
    let research_slots = calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref());
    let troops_slots = calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref());
    
    // Convert to slices for passing to load_appointments
    let construction_slots_ref: Vec<(u8, String)> = construction_slots.clone();
    let research_slots_ref: Vec<(u8, String)> = research_slots.clone();
    let troops_slots_ref: Vec<(u8, String)> = troops_slots.clone();
    
    // Parse CSV file using load_appointments with custom time slot mappings
    let entries = match load_appointments(
        &csv_path,
        Some(&construction_slots_ref),
        Some(&research_slots_ref),
        Some(&troops_slots_ref),
    ) {
        Ok(e) => e,
        Err(e) => {
            // If file doesn't exist or can't be parsed, return empty stats
            eprintln!("Error loading form submissions CSV from {}: {}", csv_path, e);
            return Ok(HttpResponse::Ok().json(FormStatsResponse {
                construction_start_time: "00:00".to_string(),
                research_start_time: "00:00".to_string(),
                troops_start_time: "00:00".to_string(),
                construction_time_slot_popularity: HashMap::new(),
                research_time_slot_popularity: HashMap::new(),
                troops_time_slot_popularity: HashMap::new(),
            }));
        }
    };
    
    // Initialize time slot popularity for Construction
    let mut construction_time_slot_popularity: HashMap<String, FormTimeSlotStats> = HashMap::new();
    for (_, time) in &construction_slots {
        construction_time_slot_popularity.insert(time.clone(), FormTimeSlotStats {
            requests: 0,
        });
    }
    
    // Initialize time slot popularity for Research
    let mut research_time_slot_popularity: HashMap<String, FormTimeSlotStats> = HashMap::new();
    for (_, time) in &research_slots {
        research_time_slot_popularity.insert(time.clone(), FormTimeSlotStats {
            requests: 0,
        });
    }
    
    // Initialize time slot popularity for Troops
    let mut troops_time_slot_popularity: HashMap<String, FormTimeSlotStats> = HashMap::new();
    for (_, time) in &troops_slots {
        troops_time_slot_popularity.insert(time.clone(), FormTimeSlotStats {
            requests: 0,
        });
    }
    
    // Create maps from slot number to time string for each day type
    let construction_slot_to_time: HashMap<u8, String> = construction_slots.iter().map(|(s, t)| (*s, t.clone())).collect();
    let research_slot_to_time: HashMap<u8, String> = research_slots.iter().map(|(s, t)| (*s, t.clone())).collect();
    let troops_slot_to_time: HashMap<u8, String> = troops_slots.iter().map(|(s, t)| (*s, t.clone())).collect();
    
    // Count actual selections
    for entry in entries {
        // Count construction time slot popularity
        for slot in &entry.construction_available_slots {
            if let Some(time) = construction_slot_to_time.get(slot) {
                if let Some(slot_stats) = construction_time_slot_popularity.get_mut(time) {
                    slot_stats.requests += 1;
                }
            }
        }
        
        // Count research time slot popularity
        for slot in &entry.research_available_slots {
            if let Some(time) = research_slot_to_time.get(slot) {
                if let Some(slot_stats) = research_time_slot_popularity.get_mut(time) {
                    slot_stats.requests += 1;
                }
            }
        }
        
        // Count troops time slot popularity
        for slot in &entry.troops_available_slots {
            if let Some(time) = troops_slot_to_time.get(slot) {
                if let Some(slot_stats) = troops_time_slot_popularity.get_mut(time) {
                    slot_stats.requests += 1;
                }
            }
        }
    }
    
    Ok(HttpResponse::Ok().json(FormStatsResponse {
        construction_start_time: config.construction_times.start_time.clone(),
        research_start_time: config.research_times.start_time.clone(),
        troops_start_time: config.troops_times.start_time.clone(),
        construction_time_slot_popularity,
        research_time_slot_popularity,
        troops_time_slot_popularity,
    }))
}

// Public form statistics page handler
async fn public_form_stats_page(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    
    // Verify form exists
    let forms = state.forms.lock().unwrap();
    if !forms.contains_key(&code) {
        drop(forms);
        return Ok(HttpResponse::NotFound().body("Form not found"));
    }
    drop(forms);
    
    let html = include_str!("../templates/form_stats.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Get current form info for account (admin - to display current form link)
async fn get_current_form_info(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();
    
    // Try session authentication first, fallback to password authentication
    let authenticated = {
        // Check session
        let session_account_name: Option<String> = session.get("account_name").ok().flatten();
        let session_server_number: Option<u32> = session.get("server_number").ok().flatten();
        
        if let (Some(session_account_name), Some(session_server_number)) = (session_account_name, session_server_number) {
            // Verify the account_name and server_number match
            session_account_name == url_account_name && session_server_number == server_number
        } else {
            // Fallback: check password header (for admin page)
            if let Some(password_header) = req.headers().get("X-Password") {
                if let Ok(password) = password_header.to_str() {
                    let accounts = state.accounts.lock().unwrap();
                    if let Some(account) = accounts.get(&url_account_name) {
                        account.password == password && account.server_number == server_number
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
    };
    
    if !authenticated {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        })));
    }
    
    // Get current form - first try from mapping, then check files in current_forms folder
    let forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let mut current_form = get_current_form(&forms, &current_forms, &url_account_name, server_number);
    drop(forms);
    drop(current_forms);
    
    // If not found in mapping or forms HashMap, check files in current_forms folder directly
    if current_form.is_none() {
        let current_forms_dir = format!("{}/current_forms", state.data_dir);
        eprintln!("Checking current_forms directory: {}", current_forms_dir);
        eprintln!("Looking for form with account_name: '{}', server_number: {}", url_account_name, server_number);
        
        if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Only process .json files (not CSV files)
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        // Skip files that end with _submissions (those are CSV files, not form JSON)
                        if file_name.contains("_submissions") {
                            continue;
                        }
                        
                        eprintln!("Checking file: {}", file_name);
                        
                        // Try to load the form JSON file
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(mut form_data) = serde_json::from_str::<FormData>(&content) {
                                // Normalize account_name to lowercase for comparison
                                let form_account_name = form_data.account_name.to_lowercase();
                                eprintln!("Found form: account_name='{}', server_number={}, code='{}'", 
                                    form_account_name, form_data.server_number, form_data.code);
                                
                                // Check if this form belongs to the requested account/server
                                if form_account_name == url_account_name && form_data.server_number == server_number {
                                    eprintln!("Match found! Returning form: {}", form_data.code);
                                    // Ensure account_name is lowercase in the returned form
                                    form_data.account_name = form_account_name;
                                    current_form = Some(form_data);
                                    break;
                                } else {
                                    eprintln!("No match: form_account_name='{}' != url_account_name='{}' OR server_number={} != {}", 
                                        form_account_name, url_account_name, form_data.server_number, server_number);
                                }
                            } else {
                                eprintln!("Failed to parse JSON from file: {}", file_name);
                            }
                        } else {
                            eprintln!("Failed to read file: {}", file_name);
                        }
                    }
                }
            }
        } else {
            eprintln!("Failed to read directory: {}", current_forms_dir);
        }
    } else {
        eprintln!("Found form in mapping/forms HashMap");
    }
    
    if let Some(form) = current_form {
        // Build form URL
        let host = req.headers().get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost:8080");
        let protocol = if host.contains("localhost") { "http" } else { "https" };
        let form_url = format!("{}://{}/form/{}", protocol, host, form.code);
        
        // Count submissions from CSV file
        // The CSV header is multiline, so we count actual data rows by looking for timestamp pattern
        // Data rows start with timestamp format: DD/MM/YYYY HH.MM.SS
        // We check if a line starts with the timestamp pattern (2 digits/2 digits/4 digits)
        let submissions_count = {
            let csv_path = format!("{}/current_forms/{}_submissions.csv", state.data_dir, form.code);
            if Path::new(&csv_path).exists() {
                if let Ok(content) = std::fs::read_to_string(&csv_path) {
                    // Count lines that start with a timestamp (DD/MM/YYYY format)
                    // This pattern matches data rows, not header lines
                    content.lines()
                        .filter(|line| {
                            let trimmed = line.trim();
                            // Check if line starts with DD/MM/YYYY pattern (timestamp)
                            trimmed.len() >= 10 && 
                            trimmed.chars().take(2).all(|c| c.is_ascii_digit()) &&
                            trimmed.chars().nth(2) == Some('/') &&
                            trimmed.chars().skip(3).take(2).all(|c| c.is_ascii_digit()) &&
                            trimmed.chars().nth(5) == Some('/') &&
                            trimmed.chars().skip(6).take(4).all(|c| c.is_ascii_digit())
                        })
                        .count()
                } else {
                    0
                }
            } else {
                0
            }
        };
        
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "form": {
                "code": form.code,
                "name": form.name,
                "created_at": form.created_at,
                "url": form_url,
                "submissions_count": submissions_count,
                "config": {
                    "alliances": form.config.alliances,
                    "construction_times": form.config.construction_times,
                    "research_times": form.config.research_times,
                    "troops_times": form.config.troops_times,
                    "predetermined_slots": form.config.predetermined_slots,
                    "intro_text": form.config.intro_text
                }
            }
        })))
    } else {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "form": null
        })))
    }
}

// Get player info by ID from form submissions
async fn get_player_by_id(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number, player_id) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();
    
    // Verify session authentication
    let session_account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    let session_server_number: u32 = match session.get("server_number") {
        Ok(Some(num)) => num,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    
    // Verify account name and server number match session
    if session_account_name.to_lowercase() != url_account_name || session_server_number != server_number {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authorized"
        })));
    }
    
    // Get current form to find CSV path
    let forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let csv_path = if let Some(current_form) = get_current_form(&forms, &current_forms, &url_account_name, server_number) {
        format!("{}/current_forms/{}_submissions.csv", state.data_dir, current_form.code)
    } else {
        drop(forms);
        drop(current_forms);
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "No current form found"
        })));
    };
    drop(forms);
    drop(current_forms);
    
    // Load submissions and find player by ID
    if !Path::new(&csv_path).exists() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form submissions not found"
        })));
    }
    
    // Load appointments (which includes player info)
    let entries = match load_appointments(&csv_path, None, None, None) {
        Ok(e) => e,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to load form submissions"
            })));
        }
    };
    
    // Find player by ID
    if let Some(entry) = entries.iter().find(|e| e.player_id == player_id) {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "player": {
                "player_id": entry.player_id,
                "name": entry.name,
                "alliance": entry.alliance
            }
        })))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Player ID not found in form submissions"
        })))
    }
}

// Download current form CSV submissions
async fn download_form_csv(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();
    
    // Verify session authentication
    let session_account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    let session_server_number: u32 = match session.get("server_number") {
        Ok(Some(num)) => num,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    
    // Verify account name and server number match session
    if session_account_name.to_lowercase() != url_account_name || session_server_number != server_number {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }
    
    // Get current form to find CSV file
    let forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let mut current_form = get_current_form(&forms, &current_forms, &url_account_name, server_number);
    drop(forms);
    drop(current_forms);
    
    // If not found in mapping, check files in current_forms folder
    if current_form.is_none() {
        let current_forms_dir = format!("{}/current_forms", state.data_dir);
        if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.contains("_submissions") {
                            continue;
                        }
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(mut form_data) = serde_json::from_str::<FormData>(&content) {
                                let form_account_name = form_data.account_name.to_lowercase();
                                if form_account_name == url_account_name && form_data.server_number == server_number {
                                    form_data.account_name = form_account_name;
                                    current_form = Some(form_data);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    if let Some(form) = current_form {
        let csv_path = format!("{}/current_forms/{}_submissions.csv", state.data_dir, form.code);
        if Path::new(&csv_path).exists() {
            if let Ok(csv_content) = std::fs::read_to_string(&csv_path) {
                let filename = format!("{}_submissions_{}.csv", form.code, 
                    chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                return Ok(HttpResponse::Ok()
                    .content_type("text/csv")
                    .append_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
                    .body(csv_content));
            }
        }
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "CSV file not found"
        })));
    } else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "No current form found"
        })));
    }
}

// Get previous form config for account (admin - to load when creating new form)
async fn get_previous_form_config(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();
    
    // Verify session authentication
    let session_account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    let session_server_number: u32 = match session.get("server_number") {
        Ok(Some(num)) => num,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    
    // Verify the account name and server number match
    if session_account_name != url_account_name || session_server_number != server_number {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Access denied"
        })));
    }
    
    let account_name = url_account_name;
    
    // Find the most recent form for this account (get the one with latest created_at)
    let forms = state.forms.lock().unwrap();
    let mut previous_form: Option<FormData> = None;
    for form_data in forms.values() {
        if form_data.account_name == account_name && form_data.server_number == server_number {
            match &previous_form {
                None => previous_form = Some(form_data.clone()),
                Some(current) => {
                    // Compare by created_at to get most recent
                    if let (Ok(current_time), Ok(new_time)) = (
                        chrono::DateTime::parse_from_rfc3339(&current.created_at),
                        chrono::DateTime::parse_from_rfc3339(&form_data.created_at)
                    ) {
                        if new_time > current_time {
                            previous_form = Some(form_data.clone());
                        }
                    } else {
                        // If parsing fails, just use the new one
                        previous_form = Some(form_data.clone());
                    }
                }
            }
        }
    }
    drop(forms);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "config": previous_form.map(|f| f.config)
    })))
}

// Home page
async fn index() -> Result<HttpResponse> {
    let html = include_str!("../templates/index.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// List all servers page
async fn servers_list_page() -> Result<HttpResponse> {
    let html = include_str!("../templates/servers_list.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// View-only schedule page (public, no admin/stats buttons)
async fn view_schedule_page(_path: web::Path<(String, u32)>) -> Result<HttpResponse> {
    let html = include_str!("../templates/view_schedule.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// Dashboard page (for logged-in users - requires authentication)
async fn dashboard_page(path: web::Path<String>, session: Session) -> Result<HttpResponse> {
    let url_account_name = path.into_inner().to_lowercase();
    
    // Check if user is logged in
    let session_account_name: Option<String> = session.get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    
    // Verify the account_name in URL matches the logged-in account
    match session_account_name {
        Some(account_name) if account_name == url_account_name => {
            // User is authenticated and accessing their own dashboard
            let html = include_str!("../templates/dashboard.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
        Some(_) => {
            // User is logged in but trying to access someone else's dashboard
            Ok(HttpResponse::Forbidden().content_type("text/html").body(
                "<html><body><h1>Access Denied</h1><p>You can only access your own dashboard.</p><a href='/'>Go Home</a></body></html>"
            ))
        }
        None => {
            // User is not logged in
            Ok(HttpResponse::Unauthorized().content_type("text/html").body(
                "<html><body><h1>Unauthorized</h1><p>Please log in to access the dashboard.</p><a href='/'>Go Home</a></body></html>"
            ))
        }
    }
}

// Get session info endpoint (for dashboard to get account/server info)
async fn get_session_info(session: Session) -> Result<HttpResponse> {
    let account_name: Option<String> = session.get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    let server_number: Option<u32> = session.get("server_number")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    
    if let (Some(account_name), Some(server_number)) = (account_name, server_number) {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "account_name": account_name,
            "server_number": server_number
        })))
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        })))
    }
}

// Logout endpoint
async fn logout_api(session: Session) -> Result<HttpResponse> {
    session.purge();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    })))
}

// API endpoint to list all servers
async fn list_servers(state: web::Data<AppState>) -> Result<HttpResponse> {
    let accounts = state.accounts.lock().unwrap();
    let mut servers: Vec<ServerInfo> = accounts.values()
        .map(|acc| ServerInfo {
            account_name: acc.account_name.clone(),
            server_number: acc.server_number,
        })
        .collect();
    drop(accounts);
    
    // Sort by account name, then server number
    servers.sort_by(|a, b| {
        a.account_name.cmp(&b.account_name)
            .then_with(|| a.server_number.cmp(&b.server_number))
    });
    
    Ok(HttpResponse::Ok().json(servers))
}

// Generate schedule endpoint (from form submissions)
async fn generate_schedule_api(session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    // Get account_name and server_number from session
    let account_name: String = match session.get("account_name") {
        Ok(Some(name)) => name,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    let server_number: u32 = match session.get("server_number") {
        Ok(Some(num)) => num,
        Ok(None) => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not logged in"
            })));
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to read session"
            })));
        }
    };
    
    let account_name = account_name.to_lowercase();
    let key = schedule_key(&account_name, server_number);
    
    // Get current form to find CSV path
    let (form_csv_path, form_config, form_code) = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        if let Some(current_form) = get_current_form(&forms, &current_forms, &account_name, server_number) {
            let csv_path = format!("{}/current_forms/{}_submissions.csv", state.data_dir, current_form.code.clone());
            (csv_path, Some(current_form.config.clone()), Some(current_form.code.clone()))
        } else {
            // Try old location for migration
            let csv_path = format!("{}/{}_{}_form_submissions.csv", state.data_dir, account_name, server_number);
            (csv_path, None, None)
        }
    };
    
    // Verify we have a current form
    if form_code.is_none() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No current form found. Please create a form first."
        })));
    }
    
    if !Path::new(&form_csv_path).exists() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No form submissions found. Please create a form and have players submit responses first."
        })));
    }
    
    let (construction_slots, research_slots, troops_slots) = if let Some(config) = &form_config {
        (
            Some(calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref())),
            Some(calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref())),
            Some(calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref())),
        )
    } else {
        (None, None, None)
    };
    
    // Load form submissions
    let entries = match load_appointments(
        &form_csv_path,
        construction_slots.as_ref().map(|v| v.as_slice()),
        research_slots.as_ref().map(|v| v.as_slice()),
        troops_slots.as_ref().map(|v| v.as_slice()),
    ) {
        Ok(e) => e,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to load form submissions: {}", e)
            })));
        }
    };
    
    if entries.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No valid form submissions found."
        })));
    }
    
    // Helper function to convert time string to slot number using form's time configuration
    let time_to_slot = |time_str: &str, time_slots: &[(u8, String)]| -> Option<u8> {
        let clean_time = time_str.trim();
        time_slots.iter()
            .find(|(_, time)| time.trim() == clean_time)
            .map(|(slot, _)| *slot)
    };
    
    // Apply predetermined slots to schedules
    let mut construction_schedule = schedule_construction_day(&entries);
    let mut research_schedule = schedule_research_day(&entries, &construction_schedule);
    let mut troops_schedule = schedule_troops_day(&entries);
    
    // Process predetermined slots if form config has them
    if let Some(config) = &form_config {
        if !config.predetermined_slots.is_empty() {
            // Get time slot mappings
            let construction_slots_vec = construction_slots.as_ref().cloned().unwrap_or_default();
            let research_slots_vec = research_slots.as_ref().cloned().unwrap_or_default();
            let troops_slots_vec = troops_slots.as_ref().cloned().unwrap_or_default();
            
            // Track which players have predetermined slots to avoid double-scheduling
            let mut predetermined_player_ids = HashSet::new();
            
            for pred_slot in &config.predetermined_slots {
                let slot_num = match pred_slot.day.as_str() {
                    "construction" => {
                        time_to_slot(&pred_slot.time, &construction_slots_vec)
                    },
                    "research" => {
                        time_to_slot(&pred_slot.time, &research_slots_vec)
                    },
                    "troops" => {
                        time_to_slot(&pred_slot.time, &troops_slots_vec)
                    },
                    _ => None,
                };
                
                if let Some(slot) = slot_num {
                    // Create a synthetic player_id for predetermined slots (use name + alliance as ID)
                    let player_id = format!("PRED-{}-{}", pred_slot.alliance, pred_slot.name);
                    predetermined_player_ids.insert(player_id.clone());
                    
                    // Create scheduled appointment for predetermined slot
                    let appointment = ScheduledAppointment {
                        player_id,
                        name: pred_slot.name.clone(),
                        alliance: pred_slot.alliance.clone(),
                        slot,
                        priority_score: 9999, // High priority to ensure it's kept
                    };
                    
                    // Apply to appropriate schedule
                    match pred_slot.day.as_str() {
                        "construction" => {
                            construction_schedule.appointments.insert(slot, appointment);
                        },
                        "research" => {
                            research_schedule.appointments.insert(slot, appointment);
                        },
                        "troops" => {
                            troops_schedule.appointments.insert(slot, appointment);
                        },
                        _ => {},
                    }
                }
            }
            
            // Remove any entries from form submissions that conflict with predetermined slots
            // (if a player was manually assigned a slot, remove them from normal scheduling)
            let entries_filtered: Vec<AppointmentEntry> = entries.iter()
                .filter(|entry| {
                    // Check if this entry matches a predetermined slot (same alliance + name)
                    !config.predetermined_slots.iter().any(|pred| {
                        pred.alliance == entry.alliance && pred.name == entry.name
                    })
                })
                .cloned()
                .collect();
            
            // Re-generate schedules with filtered entries (but keep predetermined slots)
            if entries_filtered.len() != entries.len() {
                // Only regenerate if we filtered anything
                construction_schedule = schedule_construction_day(&entries_filtered);
                research_schedule = schedule_research_day(&entries_filtered, &construction_schedule);
                troops_schedule = schedule_troops_day(&entries_filtered);
                
                // Re-apply predetermined slots (they may have been overwritten)
                for pred_slot in &config.predetermined_slots {
                    let slot_num = match pred_slot.day.as_str() {
                        "construction" => {
                            time_to_slot(&pred_slot.time, &construction_slots_vec)
                        },
                        "research" => {
                            time_to_slot(&pred_slot.time, &research_slots_vec)
                        },
                        "troops" => {
                            time_to_slot(&pred_slot.time, &troops_slots_vec)
                        },
                        _ => None,
                    };
                    
                    if let Some(slot) = slot_num {
                        let player_id = format!("PRED-{}-{}", pred_slot.alliance, pred_slot.name);
                        let appointment = ScheduledAppointment {
                            player_id,
                            name: pred_slot.name.clone(),
                            alliance: pred_slot.alliance.clone(),
                            slot,
                            priority_score: 9999,
                        };
                        
                        match pred_slot.day.as_str() {
                            "construction" => {
                                construction_schedule.appointments.insert(slot, appointment);
                            },
                            "research" => {
                                research_schedule.appointments.insert(slot, appointment);
                            },
                            "troops" => {
                                troops_schedule.appointments.insert(slot, appointment);
                            },
                            _ => {},
                        }
                    }
                }
            }
        }
    }
    
    // Create schedule data
    let schedule_data = ScheduleData {
        construction_schedule: Some(construction_schedule.clone()),
        research_schedule: Some(research_schedule.clone()),
        troops_schedule: Some(troops_schedule.clone()),
        entries: Some(entries.clone()),
    };
    
    // Save to state
    let mut schedules = state.schedules.lock().unwrap();
    schedules.insert(key.clone(), schedule_data.clone());
    drop(schedules);
    
    // Save to disk
    if let Err(e) = save_schedule(&state.data_dir, &account_name, server_number, &schedule_data) {
        eprintln!("Warning: Failed to save schedule to disk: {}", e);
    }
    
    // Also regenerate and save statistics after generating schedule
    // (This ensures stats are up-to-date with the schedule)
    let _ = get_stats(web::Path::from((account_name.clone(), server_number)), state.clone()).await;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Schedule generated successfully from form submissions!"
    })))
}

// Update schedule slot endpoint
#[derive(Deserialize)]
struct UpdateSlotRequest {
    time: String,
    player: Option<String>, // Format: "[alliance] name" or null to clear
}

async fn update_schedule_slot(
    path: web::Path<(String, u32, String)>,
    req: web::Json<UpdateSlotRequest>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number, day_str) = path.into_inner();
    let account_name = account_name.to_lowercase();
    
    // Check authentication
    if let (Some(session_account), Some(session_server)) = (
        session.get::<String>("account_name")?,
        session.get::<u32>("server_number")?
    ) {
        if session_account != account_name || session_server != server_number {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not authorized"
            })));
        }
    } else {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        })));
    }
    
    // Load schedule
    let key = schedule_key(&account_name, server_number);
    let mut schedule_data = {
        let schedules = state.schedules.lock().unwrap();
        schedules.get(&key).cloned()
            .or_else(|| load_schedule(&state.data_dir, &account_name, server_number))
    };
    
    if schedule_data.is_none() {
        schedule_data = Some(ScheduleData {
            construction_schedule: Some(DaySchedule {
                appointments: HashMap::new(),
                unassigned: Vec::new(),
            }),
            research_schedule: Some(DaySchedule {
                appointments: HashMap::new(),
                unassigned: Vec::new(),
            }),
            troops_schedule: Some(DaySchedule {
                appointments: HashMap::new(),
                unassigned: Vec::new(),
            }),
            entries: None,
        });
    }
    
    let mut schedule_data = schedule_data.unwrap();
    
    // Get form config for time slot mapping
    let form_config = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        get_current_form(&forms, &current_forms, &account_name, server_number)
            .map(|f| f.config.clone())
    };
    
    // Convert time to slot number
    let time_slots: Vec<(u8, String)> = match (day_str.as_str(), form_config.as_ref()) {
        ("construction", Some(config)) => {
            calculate_time_slots(&config.construction_times.start_time, config.construction_times.end_time.as_deref())
        },
        ("research", Some(config)) => {
            calculate_time_slots(&config.research_times.start_time, config.research_times.end_time.as_deref())
        },
        ("troops", Some(config)) => {
            calculate_time_slots(&config.troops_times.start_time, config.troops_times.end_time.as_deref())
        },
        _ => {
            (1..=49).map(|slot| (slot, slot_to_time(slot))).collect()
        }
    };
    
    let slot_num = time_slots.iter()
        .find(|(_, time)| time == &req.time)
        .map(|(slot, _)| *slot);
    
    if slot_num.is_none() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid time slot"
        })));
    }
    
    let slot = slot_num.unwrap();
    
    // Get or create the appropriate day schedule
    let day_schedule = match day_str.as_str() {
        "construction" => {
            if schedule_data.construction_schedule.is_none() {
                schedule_data.construction_schedule = Some(DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                });
            }
            schedule_data.construction_schedule.as_mut().unwrap()
        },
        "research" => {
            if schedule_data.research_schedule.is_none() {
                schedule_data.research_schedule = Some(DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                });
            }
            schedule_data.research_schedule.as_mut().unwrap()
        },
        "troops" => {
            if schedule_data.troops_schedule.is_none() {
                schedule_data.troops_schedule = Some(DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                });
            }
            schedule_data.troops_schedule.as_mut().unwrap()
        },
        _ => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid day"
        }))),
    };
    
    // Parse player name (format: "[alliance] name")
    if let Some(ref player_str) = req.player {
        let player_str = player_str.trim();
        if !player_str.is_empty() {
            // Parse "[alliance] name" format
            let (alliance, name) = if let Some(start) = player_str.find('[') {
                if let Some(end) = player_str.find(']') {
                    let alliance = player_str[start+1..end].to_string();
                    let name = player_str[end+1..].trim().to_string();
                    (alliance, name)
                } else {
                    // No closing bracket, treat whole thing as name
                    ("".to_string(), player_str.to_string())
                }
            } else {
                // No bracket, treat whole thing as name
                ("".to_string(), player_str.to_string())
            };
            
            let appointment = ScheduledAppointment {
                player_id: format!("MANUAL-{}-{}", alliance, name),
                name,
                alliance,
                slot,
                priority_score: 0,
            };
            
            day_schedule.appointments.insert(slot, appointment);
        } else {
            // Empty string, remove the slot
            day_schedule.appointments.remove(&slot);
        }
    } else {
        // None, remove the slot
        day_schedule.appointments.remove(&slot);
    }
    
    // Update the schedule in state
    {
        let mut schedules = state.schedules.lock().unwrap();
        schedules.insert(key.clone(), schedule_data.clone());
    }
    
    // Save to disk
    if let Err(e) = save_schedule(&state.data_dir, &account_name, server_number, &schedule_data) {
        eprintln!("Warning: Failed to save schedule to disk: {}", e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": "Failed to save schedule"
        })));
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Slot updated successfully"
    })))
}

// Get form submissions endpoint
async fn get_form_submissions(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();
    
    // Check authentication
    if let (Some(session_account), Some(session_server)) = (
        session.get::<String>("account_name")?,
        session.get::<u32>("server_number")?
    ) {
        if session_account != account_name || session_server != server_number {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Not authorized"
            })));
        }
    } else {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authenticated"
        })));
    }
    
    // Get current form
    let current_form = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        get_current_form(&forms, &current_forms, &account_name, server_number)
    };
    
    if current_form.is_none() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "No current form found"
        })));
    }
    
    let current_form = current_form.unwrap();
    let form_csv_path = format!("{}/current_forms/{}_submissions.csv", state.data_dir, current_form.code);
    
    if !Path::new(&form_csv_path).exists() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "submissions": []
        })));
    }
    
    // Read CSV file
    let mut reader = csv::Reader::from_path(&form_csv_path)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to read CSV: {}", e)))?;
    
    let headers = reader.headers()
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to read CSV headers: {}", e)))?
        .clone();
    
    let mut submissions = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to parse CSV record: {}", e)))?;
        
        // Skip header rows (check if first field is a timestamp pattern DD/MM/YYYY)
        let first_field = record.get(0).unwrap_or("");
        if !first_field.contains('/') || first_field.len() < 8 {
            continue; // Skip header rows
        }
        
        let mut submission = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            let header = headers.get(i)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("field_{}", i));
            submission.insert(header, serde_json::Value::String(field.to_string()));
        }
        submissions.push(serde_json::Value::Object(submission));
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "submissions": submissions
    })))
}

// Login endpoint (new - uses account name + password only, sets session cookie)
async fn login_api(req: web::Json<LoginRequest>, session: Session, state: web::Data<AppState>) -> Result<HttpResponse> {
    let account_name = req.account_name.as_ref()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Account name required"))?
        .trim()
        .to_lowercase();
    
    let accounts = state.accounts.lock().unwrap();
    if let Some(account) = accounts.get(&account_name) {
        if account.password == req.password {
            // Store account_name and server_number in session
            session.insert("account_name", &account.account_name)
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to set session: {}", e)))?;
            session.insert("server_number", account.server_number)
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to set session: {}", e)))?;
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "account_name": account.account_name,
                "server_number": account.server_number
            })))
        } else {
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "error": "Invalid password"
            })))
        }
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Account not found"
        })))
    }
}

pub async fn start_server(port: u16, _admin_password: String) -> std::io::Result<()> {
    let data_dir = "data".to_string();
    std::fs::create_dir_all(&data_dir)?;
    
    let accounts = load_accounts(&data_dir);
    let forms = load_forms(&data_dir);
    let current_forms = load_current_forms(&data_dir);
    
    let app_state = web::Data::new(AppState {
        accounts: Mutex::new(accounts),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms),
        current_forms: Mutex::new(current_forms),
        data_dir,
    });
    
    // Generate a random secret key for session cookies
    // In production, this should be a fixed secret stored securely
    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(
                SessionMiddleware::new(CookieSessionStore::default(), secret_key.clone())
            )
            .wrap(middleware::Logger::default())
            .service(Files::new("/static", "static").show_files_listing())
            .route("/", web::get().to(index))
            .route("/create-account", web::get().to(create_account_page))
            .route("/api/create-account", web::post().to(create_account))
            .route("/api/login", web::post().to(login_api))
            .route("/api/logout", web::post().to(logout_api))
            .route("/api/session", web::get().to(get_session_info))
            .route("/api/generate-schedule", web::post().to(generate_schedule_api))
            .route("/servers", web::get().to(servers_list_page))
            .route("/api/servers", web::get().to(list_servers))
            .route("/dashboard/{account_name}", web::get().to(dashboard_page))
            // View-only schedule route (public, no admin/stats buttons)
            .service(web::resource("/view/{account_name}/{server}").route(web::get().to(view_schedule_page)))
            // Public form routes (must come before generic {account_name}/{server} routes)
            .service(web::resource("/form/{code}").route(web::get().to(public_form_page)))
            .service(web::resource("/form/{code}/stats").route(web::get().to(public_form_stats_page)))
            .service(web::resource("/form/{code}/api/config").route(web::get().to(get_form_config_by_code)))
            .service(web::resource("/form/{code}/api/stats").route(web::get().to(get_form_stats_by_code)))
            .service(web::resource("/form/{code}/api/submit").route(web::post().to(submit_form_by_code)))
            // Account-specific routes - main schedule view at /{account_name}/{server}
            .service(web::resource("/{account_name}/{server}").route(web::get().to(schedules_page)))
            .service(web::resource("/{account_name}/{server}/stats").route(web::get().to(stats_page)))
            .service(web::resource("/{account_name}/{server}/admin").route(web::get().to(admin_page)))
            // Admin form management routes
            .service(web::resource("/{account_name}/{server}/api/form/create").to(create_form))
            .service(web::resource("/{account_name}/{server}/api/form/current").route(web::get().to(get_current_form_info)))
            .service(web::resource("/{account_name}/{server}/api/form/previous").route(web::get().to(get_previous_form_config)))
            .service(web::resource("/{account_name}/{server}/api/form/download-csv").route(web::get().to(download_form_csv)))
            .service(web::resource("/{account_name}/{server}/api/form/player/{player_id}").route(web::get().to(get_player_by_id)))
            .service(web::resource("/{account_name}/{server}/api/login").route(web::post().to(account_login)))
            .service(web::resource("/{account_name}/{server}/api/upload").to(account_upload))
            .service(web::resource("/{account_name}/{server}/api/stats").route(web::get().to(get_stats)))
            .service(web::resource("/{account_name}/{server}/api/schedule/{day}").route(web::get().to(get_schedule)))
            .service(web::resource("/{account_name}/{server}/api/schedule/{day}/slot").route(web::put().to(update_schedule_slot)))
            .service(web::resource("/{account_name}/{server}/api/form/submissions").route(web::get().to(get_form_submissions)))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
