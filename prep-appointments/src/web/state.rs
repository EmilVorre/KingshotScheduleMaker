//! Shared types and application state for the web server.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::AppointmentEntry;
use crate::schedule::DaySchedule;

// ============ Schedule data ============

use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleData {
    pub construction_schedule: Option<DaySchedule>,
    pub research_schedule: Option<DaySchedule>,
    pub troops_schedule: Option<DaySchedule>,
    pub entries: Option<Vec<AppointmentEntry>>,
    #[serde(default)]
    pub scheduled_player_ids: Option<Vec<String>>,
}

/// Derives the set of scheduled player IDs from schedule appointments
pub fn derive_scheduled_player_ids(data: &ScheduleData) -> HashSet<String> {
    let mut ids = HashSet::new();
    for appt in data
        .construction_schedule
        .as_ref()
        .iter()
        .flat_map(|s| s.appointments.values())
    {
        ids.insert(appt.player_id.clone());
    }
    for appt in data
        .research_schedule
        .as_ref()
        .iter()
        .flat_map(|s| s.appointments.values())
    {
        ids.insert(appt.player_id.clone());
    }
    for appt in data
        .troops_schedule
        .as_ref()
        .iter()
        .flat_map(|s| s.appointments.values())
    {
        ids.insert(appt.player_id.clone());
    }
    ids
}

/// Returns the set of scheduled player IDs, deriving from appointments if not stored
pub fn get_scheduled_player_ids(data: &ScheduleData) -> HashSet<String> {
    data.scheduled_player_ids
        .as_ref()
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_else(|| derive_scheduled_player_ids(data))
}

/// Application state shared across all handlers
pub struct AppState {
    pub accounts: Mutex<HashMap<String, Account>>,
    pub schedules: Mutex<HashMap<String, ScheduleData>>,
    pub forms: Mutex<HashMap<String, FormData>>,
    pub current_forms: Mutex<HashMap<String, String>>,
    pub data_dir: String,
    pub oauth_state_cache: super::oauth_state::OAuthStateCache,
    pub pending_oauth_cache: super::oauth_state::PendingOAuthCache,
}

// ============ Account ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_name: String,
    pub server_number: u32,
    #[serde(default)]
    pub password: String,
    pub in_game_name: String,
    #[serde(default)]
    pub player_id: Option<String>,
    /// OAuth provider: "discord" or "google"
    #[serde(default)]
    pub oauth_provider: Option<String>,
    /// OAuth user ID from the provider
    #[serde(default)]
    pub oauth_id: Option<String>,
    /// Admin privileges: can access admin resources and manage other admins
    #[serde(default)]
    pub admin: bool,
    /// Alliance access: can use Alliance Organisation tabs (approved via application)
    #[serde(default)]
    pub alliance_access: bool,
    /// Internal alliance ID (6 alphanumeric), set when application is approved
    #[serde(default)]
    pub alliance_id: Option<String>,
    /// Alliance tag from approved application
    #[serde(default)]
    pub alliance_tag: Option<String>,
    /// Alliance name from approved application
    #[serde(default)]
    pub alliance_name: Option<String>,
    /// 12-char alphanumeric friend code for sharing alliance access
    #[serde(default)]
    pub friend_code: Option<String>,
}

/// Generate a unique 12-character alphanumeric friend code
pub fn generate_friend_code() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

// ============ Form types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayTimeConfig {
    pub start_time: String,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredeterminedSlot {
    pub day: String,
    pub time: String,
    #[serde(default)]
    pub player_id: Option<String>,
    #[serde(default)]
    pub alliance: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionTruegoldMode {
    PreTruegold,
    TruegoldUnlocked,
    WarAcademyUnlocked,
    TemperedTruegoldUnlocked,
}

impl Default for ConstructionTruegoldMode {
    fn default() -> Self {
        ConstructionTruegoldMode::TruegoldUnlocked
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub alliances: Vec<String>,
    #[serde(default = "default_true")]
    pub include_non_of_above: bool,
    #[serde(default)]
    pub construction_truegold_mode: ConstructionTruegoldMode,
    pub construction_times: DayTimeConfig,
    pub research_times: DayTimeConfig,
    pub troops_times: DayTimeConfig,
    /// Logical day slots for Construction / Research (e.g. \"monday\", \"tuesday\", \"friday_full\", \"friday_sat\")
    #[serde(default)]
    pub construction_day_slot: Option<String>,
    #[serde(default)]
    pub research_day_slot: Option<String>,
    #[serde(default)]
    pub predetermined_slots: Vec<PredeterminedSlot>,
    #[serde(default)]
    pub intro_text: Option<String>,
    #[serde(default)]
    pub support_person_name: Option<String>,
    /// Kingdom ID used to validate applicants: only players whose kid matches this can submit.
    #[serde(default)]
    pub kingdom_id: String,
}

impl Default for FormConfig {
    fn default() -> Self {
        FormConfig {
            alliances: vec![],
            include_non_of_above: true,
            construction_truegold_mode: ConstructionTruegoldMode::default(),
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
            construction_day_slot: None,
            research_day_slot: None,
            predetermined_slots: vec![],
            intro_text: None,
            support_person_name: None,
            kingdom_id: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormData {
    pub code: String,
    pub account_name: String,
    pub server_number: u32,
    pub name: String,
    pub created_at: String,
    /// ISO date (YYYY-MM-DD) when form should be moved to old_forms. Default: created_at + 14 days.
    #[serde(default)]
    pub delete_date: Option<String>,
    pub config: FormConfig,
}

// ============ Request/Response types ============

#[derive(Deserialize)]
pub struct CreateAccountRequest {
    pub account_name: String,
    pub server_number: u32,
    #[serde(default)]
    pub password: Option<String>,
    pub in_game_name: String,
    #[serde(default)]
    pub player_id: Option<String>,
}

#[derive(Serialize)]
pub struct CreateAccountResponse {
    pub success: bool,
    pub message: String,
    pub schedule_url: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub account_name: Option<String>,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub account_name: Option<String>,
    #[serde(default)]
    pub server_number: Option<u32>,
    #[serde(default)]
    pub in_game_name: Option<String>,
}

#[derive(Deserialize)]
pub struct KingshotLookupRequest {
    pub player_id: String,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub account_name: String,
    pub server_number: u32,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    pub alliance_counts: HashMap<String, AllianceStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_slot_popularity: Option<HashMap<String, TimeSlotStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construction_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub research_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub troops_start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construction_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub research_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub troops_time_slot_popularity: Option<HashMap<String, FormTimeSlotStats>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AllianceStats {
    pub construction_requests: u32,
    pub research_requests: u32,
    pub troops_requests: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimeSlotStats {
    pub construction_requests: u32,
    pub research_requests: u32,
    pub troops_requests: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FormTimeSlotStats {
    pub requests: u32,
}

#[derive(Serialize)]
pub struct ScheduleResponse {
    pub day_name: String,
    pub appointments: Vec<ScheduleSlot>,
}

#[derive(Serialize)]
pub struct ScheduleSlot {
    pub time: String,
    pub player: Option<String>,
    pub is_empty: bool,
}

// ============ Form request types (used by forms module) ============

#[derive(Deserialize)]
pub struct CreateFormRequest {
    pub name: Option<String>,
    pub alliances: Vec<String>,
    #[serde(default = "default_true")]
    pub include_non_of_above: bool,
    #[serde(default)]
    pub construction_truegold_mode: ConstructionTruegoldMode,
    pub construction_times: DayTimeConfig,
    pub research_times: DayTimeConfig,
    pub troops_times: DayTimeConfig,
    /// Logical day slots selected when creating the form (optional, for display/translation)
    #[serde(default)]
    pub construction_day_slot: Option<String>,
    #[serde(default)]
    pub research_day_slot: Option<String>,
    #[serde(default)]
    pub predetermined_slots: Vec<PredeterminedSlot>,
    #[serde(default)]
    pub intro_text: Option<String>,
    #[serde(default)]
    pub support_person_name: Option<String>,
    #[serde(default)]
    pub kingdom_id: String,
}

#[derive(Deserialize)]
pub struct UpdateFormConfigRequest {
    pub predetermined_slots: Vec<PredeterminedSlot>,
}

#[derive(Serialize)]
pub struct FormStatsResponse {
    pub construction_start_time: String,
    pub research_start_time: String,
    pub troops_start_time: String,
    pub construction_time_slot_popularity: HashMap<String, FormTimeSlotStats>,
    pub research_time_slot_popularity: HashMap<String, FormTimeSlotStats>,
    pub troops_time_slot_popularity: HashMap<String, FormTimeSlotStats>,
}
