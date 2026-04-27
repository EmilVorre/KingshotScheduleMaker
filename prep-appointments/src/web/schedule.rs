//! Schedule handlers: stats, get schedule, generate schedule, update slot.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::display::format_player_name;
use crate::parser::{load_appointments_from_submissions, AppointmentEntry};
use crate::schedule::types::ScheduledAppointment;
use crate::schedule::{
    calculate_time_slots, schedule_construction_day_with_locked, schedule_research_day_with_locked,
    schedule_troops_day, schedule_troops_day_with_locked, slot_to_time, DaySchedule,
};

use super::persistence::{
    get_current_form, load_form_submissions, load_schedule, load_statistics, save_schedule,
    save_statistics, schedule_key,
};
use super::state::{
    derive_scheduled_player_ids, get_scheduled_player_ids, AllianceStats, AppState,
    FormTimeSlotStats, ScheduleData, ScheduleResponse, ScheduleSlot, StatsResponse, TimeSlotStats,
};

fn day_slot_to_index(slot: Option<&str>) -> i32 {
    match slot {
        Some("monday") => 1,
        Some("tuesday") => 2,
        Some("thursday") => 4,
        Some("friday_full") | Some("friday_sat") => 5,
        _ => 0,
    }
}

/// Whether Construction and Research days should be linked (last slot -> first slot)
/// based on the logical day slots chosen in the form configuration.
fn should_link_construction_research(config: &super::state::FormConfig) -> bool {
    let c_idx = day_slot_to_index(config.construction_day_slot.as_deref());
    let r_idx = day_slot_to_index(config.research_day_slot.as_deref());

    // Legacy / missing values: preserve old behaviour (link enabled).
    if c_idx == 0 || r_idx == 0 {
        return true;
    }

    // Only link when Research is the very next day after Construction (e.g. Monday -> Tuesday).
    r_idx - c_idx == 1
}

fn load_entries_for_current_form(
    state: &web::Data<AppState>,
    account_name: &str,
    server_number: u32,
) -> (
    Option<String>,
    Option<super::state::FormConfig>,
    Vec<AppointmentEntry>,
) {
    let current_form = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        get_current_form(&forms, &current_forms, account_name, server_number)
    };
    let Some(current_form) = current_form else {
        return (None, None, Vec::new());
    };
    let form_code = current_form.code.clone();
    let form_config = Some(current_form.config.clone());
    let submissions = load_form_submissions(&state.data_dir, &form_code);
    let construction_slots = form_config.as_ref().map(|config| {
        calculate_time_slots(
            &config.construction_times.start_time,
            config.construction_times.end_time.as_deref(),
        )
    });
    let research_slots = form_config.as_ref().map(|config| {
        calculate_time_slots(
            &config.research_times.start_time,
            config.research_times.end_time.as_deref(),
        )
    });
    let troops_slots = form_config.as_ref().map(|config| {
        calculate_time_slots(
            &config.troops_times.start_time,
            config.troops_times.end_time.as_deref(),
        )
    });
    let entries = load_appointments_from_submissions(
        &submissions,
        construction_slots.as_deref(),
        research_slots.as_deref(),
        troops_slots.as_deref(),
    );
    (Some(form_code), form_config, entries)
}

/// Stats endpoint
pub async fn get_stats(
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

    let (form_code, _form_config, form_entries) =
        load_entries_for_current_form(&state, &account_name, server_number);
    if form_code.is_some() && !form_entries.is_empty() {
        let form_config = {
            let forms = state.forms.lock().unwrap();
            let current_forms = state.current_forms.lock().unwrap();
            get_current_form(&forms, &current_forms, &account_name, server_number)
                .map(|f| f.config.clone())
        };

        let (construction_slots, research_slots, troops_slots) = if let Some(config) = &form_config
        {
            construction_start_time = Some(config.construction_times.start_time.clone());
            research_start_time = Some(config.research_times.start_time.clone());
            troops_start_time = Some(config.troops_times.start_time.clone());
            (
                Some(calculate_time_slots(
                    &config.construction_times.start_time,
                    config.construction_times.end_time.as_deref(),
                )),
                Some(calculate_time_slots(
                    &config.research_times.start_time,
                    config.research_times.end_time.as_deref(),
                )),
                Some(calculate_time_slots(
                    &config.troops_times.start_time,
                    config.troops_times.end_time.as_deref(),
                )),
            )
        } else {
            (None, None, None)
        };

        if let (Some(ref cs), Some(ref rs), Some(ref ts)) =
            (&construction_slots, &research_slots, &troops_slots)
        {
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

        let construction_slot_to_time: HashMap<u8, String> = construction_slots
            .as_ref()
            .map(|slots| slots.iter().map(|(s, t)| (*s, t.clone())).collect())
            .unwrap_or_default();
        let research_slot_to_time: HashMap<u8, String> = research_slots
            .as_ref()
            .map(|slots| slots.iter().map(|(s, t)| (*s, t.clone())).collect())
            .unwrap_or_default();
        let troops_slot_to_time: HashMap<u8, String> = troops_slots
            .as_ref()
            .map(|slots| slots.iter().map(|(s, t)| (*s, t.clone())).collect())
            .unwrap_or_default();

        {
            for entry in form_entries {
                let stats = alliance_counts
                    .entry(entry.alliance.clone())
                    .or_insert_with(|| AllianceStats {
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

                if let Some(ref mut cons_map) = construction_time_slot_popularity {
                    for slot in &entry.construction_available_slots {
                        if let Some(time) = construction_slot_to_time.get(slot) {
                            if let Some(slot_stats) = cons_map.get_mut(time) {
                                slot_stats.requests += 1;
                            }
                        }
                    }
                }

                if let Some(ref mut res_map) = research_time_slot_popularity {
                    for slot in &entry.research_available_slots {
                        if let Some(time) = research_slot_to_time.get(slot) {
                            if let Some(slot_stats) = res_map.get_mut(time) {
                                slot_stats.requests += 1;
                            }
                        }
                    }
                }

                if let Some(ref mut troops_map) = troops_time_slot_popularity {
                    for slot in &entry.troops_available_slots {
                        if let Some(time) = troops_slot_to_time.get(slot) {
                            if let Some(slot_stats) = troops_map.get_mut(time) {
                                slot_stats.requests += 1;
                            }
                        }
                    }
                }

                for slot in &entry.construction_available_slots {
                    let time = if let Some(ref slots) = construction_slots {
                        slots
                            .iter()
                            .find(|(s, _)| *s == *slot)
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| slot_to_time(*slot))
                    } else {
                        slot_to_time(*slot)
                    };
                    let slot_stats =
                        time_slot_popularity
                            .entry(time.clone())
                            .or_insert_with(|| TimeSlotStats {
                                construction_requests: 0,
                                research_requests: 0,
                                troops_requests: 0,
                            });
                    slot_stats.construction_requests += 1;
                }

                for slot in &entry.research_available_slots {
                    let time = if let Some(ref slots) = research_slots {
                        slots
                            .iter()
                            .find(|(s, _)| *s == *slot)
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| slot_to_time(*slot))
                    } else {
                        slot_to_time(*slot)
                    };
                    let slot_stats =
                        time_slot_popularity
                            .entry(time)
                            .or_insert_with(|| TimeSlotStats {
                                construction_requests: 0,
                                research_requests: 0,
                                troops_requests: 0,
                            });
                    slot_stats.research_requests += 1;
                }

                for slot in &entry.troops_available_slots {
                    let time = if let Some(ref slots) = troops_slots {
                        slots
                            .iter()
                            .find(|(s, _)| *s == *slot)
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| slot_to_time(*slot))
                    } else {
                        slot_to_time(*slot)
                    };
                    let slot_stats =
                        time_slot_popularity
                            .entry(time)
                            .or_insert_with(|| TimeSlotStats {
                                construction_requests: 0,
                                research_requests: 0,
                                troops_requests: 0,
                            });
                    slot_stats.troops_requests += 1;
                }
            }
        }
    } else {
        let schedules = state.schedules.lock().unwrap();
        if let Some(schedule_data) = schedules.get(&key) {
            if let Some(ref entries) = schedule_data.entries {
                for entry in entries {
                    let stats = alliance_counts
                        .entry(entry.alliance.clone())
                        .or_insert_with(|| AllianceStats {
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

                    for slot in &entry.construction_available_slots {
                        let time = slot_to_time(*slot);
                        let slot_stats =
                            time_slot_popularity.entry(time.clone()).or_insert_with(|| {
                                TimeSlotStats {
                                    construction_requests: 0,
                                    research_requests: 0,
                                    troops_requests: 0,
                                }
                            });
                        slot_stats.construction_requests += 1;
                    }

                    for slot in &entry.research_available_slots {
                        let time = slot_to_time(*slot);
                        let slot_stats =
                            time_slot_popularity
                                .entry(time)
                                .or_insert_with(|| TimeSlotStats {
                                    construction_requests: 0,
                                    research_requests: 0,
                                    troops_requests: 0,
                                });
                        slot_stats.research_requests += 1;
                    }

                    for slot in &entry.troops_available_slots {
                        let time = slot_to_time(*slot);
                        let slot_stats =
                            time_slot_popularity
                                .entry(time)
                                .or_insert_with(|| TimeSlotStats {
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

    let stats_response = StatsResponse {
        alliance_counts: alliance_counts.clone(),
        time_slot_popularity: if time_slot_popularity.is_empty() {
            None
        } else {
            Some(time_slot_popularity.clone())
        },
        construction_start_time,
        research_start_time,
        troops_start_time,
        construction_time_slot_popularity,
        research_time_slot_popularity,
        troops_time_slot_popularity,
    };

    if let Err(e) = save_statistics(
        &state.data_dir,
        &account_name,
        server_number,
        &stats_response,
    ) {
        eprintln!("Warning: Failed to save statistics to disk: {}", e);
    }

    Ok(HttpResponse::Ok().json(stats_response))
}

/// Public schedule by form code (no auth). Resolves form code to account+server, then loads schedule.
pub async fn get_schedule_by_form_code(
    path: web::Path<(String, String, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, form_code, day_str) = path.into_inner();
    let account_name = account_name.to_lowercase();

    let (account_name, server_number) = {
        let forms = state.forms.lock().unwrap();
        let form = forms.get(&form_code).cloned();
        drop(forms);
        match form {
            Some(f) if f.account_name.to_lowercase() == account_name => {
                (f.account_name.to_lowercase(), f.server_number)
            }
            _ => {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Form not found or account mismatch"
                })))
            }
        }
    };

    get_schedule_inner(&state, &account_name, server_number, &day_str).await
}

/// Schedule endpoint (account+server, used by dashboard)
pub async fn get_schedule(
    path: web::Path<(String, u32, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number, day_str) = path.into_inner();
    let account_name = account_name.to_lowercase();
    get_schedule_inner(&state, &account_name, server_number, &day_str).await
}

async fn get_schedule_inner(
    state: &web::Data<AppState>,
    account_name: &str,
    server_number: u32,
    day_str: &str,
) -> Result<HttpResponse> {
    let key = schedule_key(account_name, server_number);

    if let Some(schedule_data) = load_schedule(&state.data_dir, &account_name, server_number) {
        let mut schedules = state.schedules.lock().unwrap();
        schedules.insert(key.clone(), schedule_data.clone());
        drop(schedules);

        let form_config = {
            let forms = state.forms.lock().unwrap();
            let current_forms = state.current_forms.lock().unwrap();
            get_current_form(&forms, &current_forms, &account_name, server_number)
                .map(|f| f.config.clone())
        };

        let schedule = match day_str {
            "construction" => schedule_data.construction_schedule.clone(),
            "research" => schedule_data.research_schedule.clone(),
            "troops" => schedule_data.troops_schedule.clone(),
            _ => {
                return Ok(
                    HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid day"}))
                )
            }
        };

        if let Some(schedule) = schedule {
            let time_slots: Vec<(u8, String)> = match (day_str, form_config.as_ref()) {
                ("construction", Some(config)) => calculate_time_slots(
                    &config.construction_times.start_time,
                    config.construction_times.end_time.as_deref(),
                ),
                ("research", Some(config)) => calculate_time_slots(
                    &config.research_times.start_time,
                    config.research_times.end_time.as_deref(),
                ),
                ("troops", Some(config)) => calculate_time_slots(
                    &config.troops_times.start_time,
                    config.troops_times.end_time.as_deref(),
                ),
                _ => (1..=49).map(|slot| (slot, slot_to_time(slot))).collect(),
            };

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

            let day_name = match day_str {
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

    let form_config = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        get_current_form(&forms, &current_forms, &account_name, server_number)
            .map(|f| f.config.clone())
    };

    let time_slots: Vec<(u8, String)> = match (day_str, form_config.as_ref()) {
        ("construction", Some(config)) => calculate_time_slots(
            &config.construction_times.start_time,
            config.construction_times.end_time.as_deref(),
        ),
        ("research", Some(config)) => calculate_time_slots(
            &config.research_times.start_time,
            config.research_times.end_time.as_deref(),
        ),
        ("troops", Some(config)) => calculate_time_slots(
            &config.troops_times.start_time,
            config.troops_times.end_time.as_deref(),
        ),
        _ => (1..=49).map(|slot| (slot, slot_to_time(slot))).collect(),
    };

    let day_name = match day_str {
        "construction" => "Construction Day",
        "research" => "Research Day",
        "troops" => "Troops Training Day",
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid day"})))
        }
    };

    let schedule_opt = {
        let schedules = state.schedules.lock().unwrap();
        if let Some(schedule_data) = schedules.get(&key) {
            match day_str {
                "construction" => schedule_data.construction_schedule.as_ref().cloned(),
                "research" => schedule_data.research_schedule.as_ref().cloned(),
                "troops" => schedule_data.troops_schedule.as_ref().cloned(),
                _ => None,
            }
        } else {
            None
        }
    };

    let schedule = if let Some(s) = schedule_opt {
        s
    } else {
        let (_form_code, config_for_loading, entries) =
            load_entries_for_current_form(&state, account_name, server_number);
        if !entries.is_empty() {
            let (construction_slots, _research_slots, _troops_slots) =
                if let Some(config) = &config_for_loading {
                    (
                        Some(calculate_time_slots(
                            &config.construction_times.start_time,
                            config.construction_times.end_time.as_deref(),
                        )),
                        Some(calculate_time_slots(
                            &config.research_times.start_time,
                            config.research_times.end_time.as_deref(),
                        )),
                        Some(calculate_time_slots(
                            &config.troops_times.start_time,
                            config.troops_times.end_time.as_deref(),
                        )),
                    )
                } else {
                    (None, None, None)
                };

            let last_slot_override = construction_slots
                .as_ref()
                .and_then(|slots| slots.iter().map(|(s, _)| *s).max());
            let construction_schedule = schedule_construction_day_with_locked(
                &entries,
                &HashSet::new(),
                last_slot_override,
                None,
            );
            let research_schedule = schedule_research_day_with_locked(
                &entries,
                &construction_schedule,
                &HashSet::new(),
                true,
            );
            let troops_schedule = schedule_troops_day(&entries);

            let scheduled_ids: Vec<String> = {
                let mut ids = HashSet::new();
                for appt in construction_schedule.appointments.values() {
                    ids.insert(appt.player_id.clone());
                }
                for appt in research_schedule.appointments.values() {
                    ids.insert(appt.player_id.clone());
                }
                for appt in troops_schedule.appointments.values() {
                    ids.insert(appt.player_id.clone());
                }
                ids.into_iter().collect()
            };
            let schedule_data = ScheduleData {
                construction_schedule: Some(construction_schedule.clone()),
                research_schedule: Some(research_schedule.clone()),
                troops_schedule: Some(troops_schedule.clone()),
                entries: Some(entries.clone()),
                scheduled_player_ids: Some(scheduled_ids),
            };

            let mut schedules = state.schedules.lock().unwrap();
            schedules.insert(key.clone(), schedule_data.clone());
            drop(schedules);

            if let Err(e) = save_schedule(
                &state.data_dir,
                &account_name,
                server_number,
                &schedule_data,
            ) {
                eprintln!("Warning: Failed to save schedule to disk: {}", e);
            }

            match day_str {
                "construction" => construction_schedule,
                "research" => research_schedule,
                "troops" => troops_schedule,
                _ => {
                    return Ok(HttpResponse::BadRequest()
                        .json(serde_json::json!({"error": "Invalid day"})))
                }
            }
        } else {
            DaySchedule {
                appointments: HashMap::new(),
                unassigned: Vec::new(),
            }
        }
    };

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

#[derive(Deserialize)]
pub struct GenerateScheduleRequest {
    #[serde(default)]
    pub append: bool,
    /// When set, only generate this day. Crossover slot (Construction last = Research slot 1) works both ways.
    #[serde(default)]
    pub day: Option<String>,
}

/// Generate schedule endpoint (from form submissions)
pub async fn generate_schedule_api(
    payload: Option<web::Json<GenerateScheduleRequest>>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let append = payload.as_ref().map(|p| p.append).unwrap_or(false);
    let day_filter = payload
        .as_ref()
        .and_then(|p| p.day.as_ref())
        .map(|s| s.to_lowercase());
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

    let (form_code, form_config, entries_from_submissions) =
        load_entries_for_current_form(&state, &account_name, server_number);

    if form_code.is_none() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No current form found. Please create a form first."
        })));
    }

    if entries_from_submissions.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No form submissions found. Please create a form and have players submit responses first."
        })));
    }

    let (construction_slots, research_slots, troops_slots, link_construction_research_days) =
        if let Some(config) = &form_config {
            (
                Some(calculate_time_slots(
                    &config.construction_times.start_time,
                    config.construction_times.end_time.as_deref(),
                )),
                Some(calculate_time_slots(
                    &config.research_times.start_time,
                    config.research_times.end_time.as_deref(),
                )),
                Some(calculate_time_slots(
                    &config.troops_times.start_time,
                    config.troops_times.end_time.as_deref(),
                )),
                should_link_construction_research(config),
            )
        } else {
            (None, None, None, true)
        };

    let entries = entries_from_submissions;

    if entries.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "No valid form submissions found."
        })));
    }

    let existing_schedule = if append || day_filter.is_some() {
        let maybe_cached = {
            let schedules = state.schedules.lock().unwrap();
            schedules.get(&key).cloned()
        };
        maybe_cached.or_else(|| load_schedule(&state.data_dir, &account_name, server_number))
    } else {
        None
    };

    let (
        entries_to_use,
        existing_construction_slots,
        existing_research_slots,
        existing_troops_slots,
        existing_appointments,
    ) = if let Some(ref existing) = existing_schedule {
        let existing_construction_slots: HashSet<u8> = existing
            .construction_schedule
            .as_ref()
            .map(|s| s.appointments.keys().copied().collect())
            .unwrap_or_default();
        let existing_research_slots: HashSet<u8> = existing
            .research_schedule
            .as_ref()
            .map(|s| s.appointments.keys().copied().collect())
            .unwrap_or_default();
        let existing_troops_slots: HashSet<u8> = existing
            .troops_schedule
            .as_ref()
            .map(|s| s.appointments.keys().copied().collect())
            .unwrap_or_default();

        let scheduled_player_ids = get_scheduled_player_ids(existing);
        let entries_filtered: Vec<AppointmentEntry> = entries
            .iter()
            .filter(|e| !scheduled_player_ids.contains(&e.player_id))
            .cloned()
            .collect();

        (
            entries_filtered,
            existing_construction_slots,
            existing_research_slots,
            existing_troops_slots,
            (
                existing.construction_schedule.clone(),
                existing.research_schedule.clone(),
                existing.troops_schedule.clone(),
            ),
        )
    } else {
        (
            entries.clone(),
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            (None, None, None),
        )
    };

    if append && existing_schedule.is_some() && entries_to_use.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "All form submissions are already in the schedule. No new assignments to add."
        })));
    }

    // Day-by-day generation: only generate the requested day and merge into existing schedule
    if let Some(ref day) = day_filter {
        let day = day.as_str();
        if !["construction", "research", "troops"].contains(&day) {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": format!("Invalid day '{}'. Use 'construction', 'research', or 'troops'.", day)
            })));
        }

        let construction_slots_vec = construction_slots.as_ref().cloned().unwrap_or_default();
        let last_slot_override = construction_slots_vec.iter().map(|(s, _)| *s).max();

        let entries_for_day: Vec<AppointmentEntry> = match day {
            "construction" => {
                let mut in_other = HashSet::new();
                for s in [
                    existing_appointments.1.as_ref(),
                    existing_appointments.2.as_ref(),
                ] {
                    if let Some(sched) = s {
                        for a in sched.appointments.values() {
                            in_other.insert(a.player_id.clone());
                        }
                    }
                }
                let existing_construction_ids: HashSet<String> = existing_appointments
                    .0
                    .as_ref()
                    .map(|s| {
                        s.appointments
                            .values()
                            .map(|a| a.player_id.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let mut e: Vec<_> = entries_to_use
                    .iter()
                    .filter(|e| {
                        e.wants_construction
                            && !e.construction_available_slots.is_empty()
                            && !in_other.contains(&e.player_id)
                            && !existing_construction_ids.contains(&e.player_id)
                    })
                    .cloned()
                    .collect();
                if let Some(ref r) = existing_appointments.1 {
                    if let Some(r1) = r.appointments.get(&1) {
                        if !existing_construction_ids.contains(&r1.player_id)
                            && !e.iter().any(|x| x.player_id == r1.player_id)
                        {
                            if let Some(entry) =
                                entries.iter().find(|x| x.player_id == r1.player_id)
                            {
                                e.push(entry.clone());
                            }
                        }
                    }
                }
                e
            }
            "research" => {
                let mut in_other = HashSet::new();
                for s in [
                    existing_appointments.0.as_ref(),
                    existing_appointments.2.as_ref(),
                ] {
                    if let Some(sched) = s {
                        for a in sched.appointments.values() {
                            in_other.insert(a.player_id.clone());
                        }
                    }
                }
                let last_slot = last_slot_override.unwrap_or(49);
                let existing_research_ids: HashSet<String> = existing_appointments
                    .1
                    .as_ref()
                    .map(|s| {
                        s.appointments
                            .values()
                            .map(|a| a.player_id.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let mut e: Vec<_> = entries_to_use
                    .iter()
                    .filter(|e| {
                        e.wants_research
                            && !e.research_available_slots.is_empty()
                            && !in_other.contains(&e.player_id)
                            && !existing_research_ids.contains(&e.player_id)
                    })
                    .cloned()
                    .collect();
                if let Some(ref c) = existing_appointments.0 {
                    if let Some(clast) = c.appointments.get(&last_slot) {
                        if !existing_research_ids.contains(&clast.player_id)
                            && !e.iter().any(|x| x.player_id == clast.player_id)
                        {
                            if let Some(entry) =
                                entries.iter().find(|x| x.player_id == clast.player_id)
                            {
                                e.push(entry.clone());
                            }
                        }
                    }
                }
                e
            }
            "troops" => {
                let mut in_other = HashSet::new();
                for s in [
                    existing_appointments.0.as_ref(),
                    existing_appointments.1.as_ref(),
                ] {
                    if let Some(sched) = s {
                        for a in sched.appointments.values() {
                            in_other.insert(a.player_id.clone());
                        }
                    }
                }
                entries_to_use
                    .iter()
                    .filter(|e| {
                        e.wants_troops
                            && !e.troops_available_slots.is_empty()
                            && !in_other.contains(&e.player_id)
                    })
                    .cloned()
                    .collect()
            }
            _ => unreachable!(),
        };

        let (new_construction, new_research, new_troops) = match day {
            "construction" => {
                let sched = schedule_construction_day_with_locked(
                    &entries_for_day,
                    &existing_construction_slots,
                    last_slot_override,
                    existing_appointments.1.as_ref(),
                );
                (Some(sched), None, None)
            }
            "research" => {
                let sched = schedule_research_day_with_locked(
                    &entries_for_day,
                    existing_appointments.0.as_ref().unwrap_or(&DaySchedule {
                        appointments: HashMap::new(),
                        unassigned: Vec::new(),
                    }),
                    &existing_research_slots,
                    link_construction_research_days,
                );
                (None, Some(sched), None)
            }
            "troops" => {
                let sched =
                    schedule_troops_day_with_locked(&entries_for_day, &existing_troops_slots);
                (None, None, Some(sched))
            }
            _ => unreachable!(),
        };

        let merge_day = |existing: Option<&DaySchedule>, new: DaySchedule| {
            let mut merged = existing.map(|e| e.appointments.clone()).unwrap_or_default();
            for (slot, appt) in new.appointments {
                if !merged.contains_key(&slot) {
                    merged.insert(slot, appt);
                }
            }
            DaySchedule {
                appointments: merged,
                unassigned: new.unassigned,
            }
        };

        let construction_schedule = match (new_construction.as_ref(), day, append) {
            (Some(n), "construction", true) => {
                merge_day(existing_appointments.0.as_ref(), n.clone())
            }
            (Some(n), "construction", false) => n.clone(),
            _ => existing_appointments
                .0
                .clone()
                .unwrap_or_else(|| DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                }),
        };
        let research_schedule = match (new_research.as_ref(), day, append) {
            (Some(n), "research", true) => merge_day(existing_appointments.1.as_ref(), n.clone()),
            (Some(n), "research", false) => n.clone(),
            _ => existing_appointments
                .1
                .clone()
                .unwrap_or_else(|| DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                }),
        };
        let troops_schedule = match (new_troops.as_ref(), day, append) {
            (Some(n), "troops", true) => merge_day(existing_appointments.2.as_ref(), n.clone()),
            (Some(n), "troops", false) => n.clone(),
            _ => existing_appointments
                .2
                .clone()
                .unwrap_or_else(|| DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                }),
        };

        let scheduled_ids: Vec<String> = {
            let mut ids = HashSet::new();
            for appt in construction_schedule.appointments.values() {
                ids.insert(appt.player_id.clone());
            }
            for appt in research_schedule.appointments.values() {
                ids.insert(appt.player_id.clone());
            }
            for appt in troops_schedule.appointments.values() {
                ids.insert(appt.player_id.clone());
            }
            ids.into_iter().collect()
        };
        let schedule_data = ScheduleData {
            construction_schedule: Some(construction_schedule.clone()),
            research_schedule: Some(research_schedule.clone()),
            troops_schedule: Some(troops_schedule.clone()),
            entries: Some(entries.clone()),
            scheduled_player_ids: Some(scheduled_ids),
        };

        let mut schedules = state.schedules.lock().unwrap();
        schedules.insert(key.clone(), schedule_data.clone());
        drop(schedules);

        if let Err(e) = save_schedule(
            &state.data_dir,
            &account_name,
            server_number,
            &schedule_data,
        ) {
            eprintln!("Warning: Failed to save schedule to disk: {}", e);
        }

        let _ = get_stats(
            web::Path::from((account_name.clone(), server_number)),
            state.clone(),
        )
        .await;

        let day_name = day
            .replace("construction", "Construction")
            .replace("research", "Research")
            .replace("troops", "Troops");
        let msg = if append {
            format!(
                "{} schedule appended successfully. Empty slots filled.",
                day_name
            )
        } else {
            format!("{} schedule replaced successfully.", day_name)
        };
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": msg
        })));
    }

    let time_to_slot = |time_str: &str, time_slots: &[(u8, String)]| -> Option<u8> {
        let clean_time = time_str.trim();

        if !time_slots.is_empty() {
            if let Some(slot) = time_slots
                .iter()
                .find(|(_, time)| time.trim() == clean_time)
                .map(|(slot, _)| *slot)
            {
                return Some(slot);
            }
        }

        if clean_time == "00:00" {
            return Some(1);
        }

        let parts: Vec<&str> = clean_time.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let hours: u32 = match parts[0].parse() {
            Ok(h) => h,
            Err(_) => return None,
        };
        let minutes: u32 = match parts[1].parse() {
            Ok(m) => m,
            Err(_) => return None,
        };

        let total_minutes = hours * 60 + minutes;

        if total_minutes == 0 {
            Some(1)
        } else if total_minutes == 15 {
            Some(2)
        } else if total_minutes == 45 {
            Some(3)
        } else if total_minutes > 45 {
            let slot = 3 + ((total_minutes - 45) / 30);
            if slot <= 49 {
                Some(slot as u8)
            } else {
                None
            }
        } else {
            None
        }
    };

    let mut construction_predetermined_slots = HashSet::new();
    let mut research_predetermined_slots = HashSet::new();
    let mut troops_predetermined_slots = HashSet::new();

    let (construction_schedule, research_schedule, troops_schedule) = if let Some(config) =
        &form_config
    {
        if !config.predetermined_slots.is_empty() {
            let construction_slots_vec = construction_slots.as_ref().cloned().unwrap_or_default();
            let research_slots_vec = research_slots.as_ref().cloned().unwrap_or_default();
            let troops_slots_vec = troops_slots.as_ref().cloned().unwrap_or_default();

            let mut invalid_slots: Vec<String> = Vec::new();
            let mut resolved_slots: Vec<(String, u8, String, String, String)> = Vec::new();
            let mut seen_slots: HashMap<String, String> = HashMap::new();

            for pred_slot in &config.predetermined_slots {
                let (player_id, alliance, name) = if let Some(ref pid) = pred_slot.player_id {
                    if !pid.trim().is_empty() {
                        let (a, n) = entries
                            .iter()
                            .find(|e| e.player_id == pid.trim())
                            .map(|e| (e.alliance.clone(), e.name.clone()))
                            .unwrap_or((pred_slot.alliance.clone(), pred_slot.name.clone()));
                        (pid.trim().to_string(), a, n)
                    } else {
                        let entry = entries.iter().find(|e| {
                            e.alliance
                                .trim()
                                .eq_ignore_ascii_case(&pred_slot.alliance.trim())
                                && e.name.trim().eq_ignore_ascii_case(&pred_slot.name.trim())
                        });
                        match entry {
                            Some(e) => (e.player_id.clone(), e.alliance.clone(), e.name.clone()),
                            None => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                "success": false,
                                "error": format!(
                                    "Predetermined slot for {} {}: Player ID required. Enter player ID in the form, or ensure {} {} has submitted the form.",
                                    pred_slot.day, pred_slot.time, pred_slot.alliance, pred_slot.name
                                )
                            }))),
                        }
                    }
                } else {
                    let entry = entries.iter().find(|e| {
                        e.alliance
                            .trim()
                            .eq_ignore_ascii_case(&pred_slot.alliance.trim())
                            && e.name.trim().eq_ignore_ascii_case(&pred_slot.name.trim())
                    });
                    match entry {
                        Some(e) => (e.player_id.clone(), e.alliance.clone(), e.name.clone()),
                        None => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "success": false,
                            "error": format!(
                                "Predetermined slot for {} {}: Could not resolve player ID for {} {}. They must have submitted the form, or use player ID.",
                                pred_slot.day, pred_slot.time, pred_slot.alliance, pred_slot.name
                            )
                        }))),
                    }
                };

                let slot_key = format!("{}:{}", pred_slot.day, pred_slot.time.trim());
                if let Some(prev_id) = seen_slots.get(&slot_key) {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "success": false,
                        "error": format!(
                            "Conflict: Multiple players predetermined for {} {} (player IDs {} and {})",
                            pred_slot.day, pred_slot.time, prev_id, player_id
                        )
                    })));
                }
                seen_slots.insert(slot_key, player_id.clone());

                let slot_num = match pred_slot.day.as_str() {
                    "construction" => time_to_slot(&pred_slot.time, &construction_slots_vec),
                    "research" => time_to_slot(&pred_slot.time, &research_slots_vec),
                    "troops" => time_to_slot(&pred_slot.time, &troops_slots_vec),
                    _ => None,
                };

                match slot_num {
                    Some(slot) => {
                        resolved_slots.push((
                            pred_slot.day.clone(),
                            slot,
                            player_id,
                            alliance,
                            name,
                        ));
                    }
                    None => {
                        invalid_slots
                            .push(format!("{} {} ({})", pred_slot.day, pred_slot.time, name));
                    }
                }
            }

            if !invalid_slots.is_empty() {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": format!(
                        "Invalid or unrecognized time slot(s) for predetermined assignments: {}",
                        invalid_slots.join("; ")
                    )
                })));
            }

            let research_slot1_from_resolved = resolved_slots
                .iter()
                .filter(|(day, slot, _, _, _)| day == "research" && *slot == 1)
                .count();
            if research_slot1_from_resolved > 1 {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": "Only one player can have research slot 1 predetermined. Multiple players were configured for research slot 1."
                })));
            }

            let last_construction_slot = construction_slots_vec
                .iter()
                .map(|(s, _)| *s)
                .max()
                .unwrap_or(49);

            let mut research_slot1_players: Vec<String> = Vec::new();
            let mut construction_last_slot_players: Vec<String> = Vec::new();

            for (day, slot, player_id, _alliance, _name) in &resolved_slots {
                match day.as_str() {
                    "construction" => {
                        construction_predetermined_slots.insert(*slot);
                        if *slot == last_construction_slot {
                            construction_last_slot_players.push(player_id.clone());
                        }
                    }
                    "research" => {
                        research_predetermined_slots.insert(*slot);
                        if *slot == 1 {
                            research_slot1_players.push(player_id.clone());
                        }
                    }
                    "troops" => {
                        troops_predetermined_slots.insert(*slot);
                    }
                    _ => {}
                }
            }

            let mut effective_research_slot1: HashSet<String> = HashSet::new();
            for id in &research_slot1_players {
                effective_research_slot1.insert(id.clone());
            }
            for id in &construction_last_slot_players {
                effective_research_slot1.insert(id.clone());
            }
            if effective_research_slot1.len() > 1 {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "error": "Conflict: Only one player can have the research slot 1 + construction last slot link. You have multiple players for research slot 1 and/or construction last slot."
                })));
            }

            if append {
                let check_conflict = |existing_slots: &HashSet<u8>,
                                      existing_appts: Option<&DaySchedule>,
                                      slot: u8,
                                      player_id: &str| {
                    if !existing_slots.contains(&slot) {
                        return false;
                    }
                    let Some(appt) = existing_appts.and_then(|s| s.appointments.get(&slot)) else {
                        return false;
                    };
                    appt.player_id != player_id
                };
                for (day, slot, player_id, _alliance, _name) in &resolved_slots {
                    let conflict = match day.as_str() {
                        "construction" => check_conflict(
                            &existing_construction_slots,
                            existing_appointments.0.as_ref(),
                            *slot,
                            player_id,
                        ),
                        "research" => check_conflict(
                            &existing_research_slots,
                            existing_appointments.1.as_ref(),
                            *slot,
                            player_id,
                        ),
                        "troops" => check_conflict(
                            &existing_troops_slots,
                            existing_appointments.2.as_ref(),
                            *slot,
                            player_id,
                        ),
                        _ => false,
                    };
                    if conflict {
                        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "success": false,
                            "error": format!(
                                "Append conflict: Predetermined slot {} {} for player {} is already filled by a different player in the existing schedule. Clear the slot manually or generate without append.",
                                day, slot, player_id
                            )
                        })));
                    }
                }
                if !effective_research_slot1.is_empty() {
                    let pred_id = effective_research_slot1.iter().next().unwrap();
                    let existing_r1 = existing_appointments
                        .1
                        .as_ref()
                        .and_then(|s| s.appointments.get(&1));
                    let existing_last = existing_appointments
                        .0
                        .as_ref()
                        .and_then(|s| s.appointments.get(&last_construction_slot));
                    if let Some(ex_r1) = existing_r1 {
                        if ex_r1.player_id != *pred_id {
                            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                "success": false,
                                "error": "Append conflict: Existing schedule has a different player in research slot 1. The research slot 1 + construction last slot link requires one player for both. Clear research slot 1 and construction last slot in the existing schedule first, or generate without append."
                            })));
                        }
                    }
                    if let Some(ex_last) = existing_last {
                        if ex_last.player_id != *pred_id {
                            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                "success": false,
                                "error": "Append conflict: Existing schedule has a different player in construction last slot. The research slot 1 + construction last slot link requires one player for both. Clear research slot 1 and construction last slot in the existing schedule first, or generate without append."
                            })));
                        }
                    }
                }
            }

            construction_predetermined_slots.extend(&existing_construction_slots);
            research_predetermined_slots.extend(&existing_research_slots);
            troops_predetermined_slots.extend(&existing_troops_slots);

            if !construction_last_slot_players.is_empty() {
                research_predetermined_slots.insert(1);
            }

            if !research_slot1_players.is_empty() {
                for player_id in &research_slot1_players {
                    let already_has_construction = resolved_slots
                        .iter()
                        .any(|(day, _, pid, _, _)| *day == "construction" && pid == player_id);

                    if !already_has_construction {
                        construction_predetermined_slots.insert(last_construction_slot);
                    }
                }
            }

            let construction_pred_player_ids: HashSet<String> = resolved_slots
                .iter()
                .filter(|(day, _, _, _, _)| *day == "construction")
                .map(|(_, _, pid, _, _)| pid.clone())
                .chain(research_slot1_players.iter().cloned())
                .collect();
            let research_pred_player_ids: HashSet<String> = resolved_slots
                .iter()
                .filter(|(day, _, _, _, _)| *day == "research")
                .map(|(_, _, pid, _, _)| pid.clone())
                .chain(construction_last_slot_players.iter().cloned())
                .collect();
            let troops_pred_player_ids: HashSet<String> = resolved_slots
                .iter()
                .filter(|(day, _, _, _, _)| *day == "troops")
                .map(|(_, _, pid, _, _)| pid.clone())
                .collect();

            let construction_entries_filtered: Vec<AppointmentEntry> = entries_to_use
                .iter()
                .filter(|entry| !construction_pred_player_ids.contains(&entry.player_id))
                .cloned()
                .collect();
            let research_entries_filtered: Vec<AppointmentEntry> = entries_to_use
                .iter()
                .filter(|entry| !research_pred_player_ids.contains(&entry.player_id))
                .cloned()
                .collect();
            let troops_entries_filtered: Vec<AppointmentEntry> = entries_to_use
                .iter()
                .filter(|entry| !troops_pred_player_ids.contains(&entry.player_id))
                .cloned()
                .collect();

            let mut construction_schedule = schedule_construction_day_with_locked(
                &construction_entries_filtered,
                &construction_predetermined_slots,
                Some(last_construction_slot),
                None,
            );
            let mut research_schedule = schedule_research_day_with_locked(
                &research_entries_filtered,
                &construction_schedule,
                &research_predetermined_slots,
                link_construction_research_days,
            );
            let mut troops_schedule = schedule_troops_day_with_locked(
                &troops_entries_filtered,
                &troops_predetermined_slots,
            );

            for (day, slot, player_id, alliance, name) in &resolved_slots {
                let appointment = ScheduledAppointment {
                    player_id: player_id.clone(),
                    name: name.clone(),
                    alliance: alliance.clone(),
                    slot: *slot,
                    priority_score: 9999,
                };

                match day.as_str() {
                    "construction" => {
                        construction_schedule
                            .appointments
                            .insert(*slot, appointment.clone());
                        if *slot == last_construction_slot {
                            let already_has_research = resolved_slots
                                .iter()
                                .any(|(d, _, pid, _, _)| *d == "research" && pid == player_id);
                            if !already_has_research {
                                let research_appointment = ScheduledAppointment {
                                    player_id: player_id.clone(),
                                    name: name.clone(),
                                    alliance: alliance.clone(),
                                    slot: 1,
                                    priority_score: 9999,
                                };
                                research_schedule
                                    .appointments
                                    .insert(1, research_appointment);
                            }
                        }
                    }
                    "research" => {
                        research_schedule
                            .appointments
                            .insert(*slot, appointment.clone());
                        if *slot == 1 {
                            let already_has_construction = resolved_slots
                                .iter()
                                .any(|(d, _, pid, _, _)| *d == "construction" && pid == player_id);
                            if !already_has_construction {
                                construction_schedule
                                    .appointments
                                    .retain(|_, appt| appt.player_id != *player_id);
                                let construction_appointment = ScheduledAppointment {
                                    player_id: player_id.clone(),
                                    name: name.clone(),
                                    alliance: alliance.clone(),
                                    slot: last_construction_slot,
                                    priority_score: 9999,
                                };
                                construction_schedule
                                    .appointments
                                    .insert(last_construction_slot, construction_appointment);
                            }
                        }
                    }
                    "troops" => {
                        troops_schedule.appointments.insert(*slot, appointment);
                    }
                    _ => {}
                }
            }

            (construction_schedule, research_schedule, troops_schedule)
        } else {
            let last_slot_override = construction_slots
                .as_ref()
                .and_then(|slots| slots.iter().map(|(s, _)| *s).max());
            let construction_schedule = schedule_construction_day_with_locked(
                &entries_to_use,
                &existing_construction_slots,
                last_slot_override,
                existing_appointments.1.as_ref(),
            );
            let research_schedule = schedule_research_day_with_locked(
                &entries_to_use,
                &construction_schedule,
                &existing_research_slots,
                link_construction_research_days,
            );
            let troops_schedule =
                schedule_troops_day_with_locked(&entries_to_use, &existing_troops_slots);
            (construction_schedule, research_schedule, troops_schedule)
        }
    } else {
        let construction_schedule = schedule_construction_day_with_locked(
            &entries_to_use,
            &existing_construction_slots,
            None,
            existing_appointments.1.as_ref(),
        );
        let research_schedule = schedule_research_day_with_locked(
            &entries_to_use,
            &construction_schedule,
            &existing_research_slots,
            link_construction_research_days,
        );
        let troops_schedule =
            schedule_troops_day_with_locked(&entries_to_use, &existing_troops_slots);
        (construction_schedule, research_schedule, troops_schedule)
    };

    let (construction_schedule, research_schedule, troops_schedule) = {
        let merge_day = |existing: Option<&DaySchedule>, new: DaySchedule| {
            let mut merged = existing.map(|e| e.appointments.clone()).unwrap_or_default();
            for (slot, appt) in new.appointments {
                if !merged.contains_key(&slot) {
                    merged.insert(slot, appt);
                }
            }
            DaySchedule {
                appointments: merged,
                unassigned: new.unassigned,
            }
        };
        (
            merge_day(existing_appointments.0.as_ref(), construction_schedule),
            merge_day(existing_appointments.1.as_ref(), research_schedule),
            merge_day(existing_appointments.2.as_ref(), troops_schedule),
        )
    };

    let scheduled_ids: Vec<String> = {
        let mut ids = HashSet::new();
        for appt in construction_schedule.appointments.values() {
            ids.insert(appt.player_id.clone());
        }
        for appt in research_schedule.appointments.values() {
            ids.insert(appt.player_id.clone());
        }
        for appt in troops_schedule.appointments.values() {
            ids.insert(appt.player_id.clone());
        }
        ids.into_iter().collect()
    };
    let schedule_data = ScheduleData {
        construction_schedule: Some(construction_schedule.clone()),
        research_schedule: Some(research_schedule.clone()),
        troops_schedule: Some(troops_schedule.clone()),
        entries: Some(entries.clone()),
        scheduled_player_ids: Some(scheduled_ids),
    };

    let mut schedules = state.schedules.lock().unwrap();
    schedules.insert(key.clone(), schedule_data.clone());
    drop(schedules);

    if let Err(e) = save_schedule(
        &state.data_dir,
        &account_name,
        server_number,
        &schedule_data,
    ) {
        eprintln!("Warning: Failed to save schedule to disk: {}", e);
    }

    let _ = get_stats(
        web::Path::from((account_name.clone(), server_number)),
        state.clone(),
    )
    .await;

    let actually_merged = append && existing_schedule.is_some();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": if actually_merged {
            "Schedule appended successfully! New assignments added to empty slots."
        } else if append {
            "No existing schedule found. Generated new schedule from form submissions."
        } else {
            "Schedule generated successfully from form submissions!"
        }
    })))
}

#[derive(Deserialize)]
pub struct UpdateSlotRequest {
    pub time: String,
    pub player: Option<String>,
}

/// Update schedule slot endpoint
pub async fn update_schedule_slot(
    path: web::Path<(String, u32, String)>,
    req: web::Json<UpdateSlotRequest>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number, day_str) = path.into_inner();
    let account_name = account_name.to_lowercase();

    if let (Some(session_account), Some(session_server)) = (
        session.get::<String>("account_name")?,
        session.get::<u32>("server_number")?,
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

    let key = schedule_key(&account_name, server_number);
    let mut schedule_data = {
        let schedules = state.schedules.lock().unwrap();
        schedules
            .get(&key)
            .cloned()
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
            scheduled_player_ids: None,
        });
    }

    let mut schedule_data = schedule_data.unwrap();

    let form_config = {
        let forms = state.forms.lock().unwrap();
        let current_forms = state.current_forms.lock().unwrap();
        get_current_form(&forms, &current_forms, &account_name, server_number)
            .map(|f| f.config.clone())
    };

    let time_slots: Vec<(u8, String)> = match (day_str.as_ref(), form_config.as_ref()) {
        ("construction", Some(config)) => calculate_time_slots(
            &config.construction_times.start_time,
            config.construction_times.end_time.as_deref(),
        ),
        ("research", Some(config)) => calculate_time_slots(
            &config.research_times.start_time,
            config.research_times.end_time.as_deref(),
        ),
        ("troops", Some(config)) => calculate_time_slots(
            &config.troops_times.start_time,
            config.troops_times.end_time.as_deref(),
        ),
        _ => (1..=49).map(|slot| (slot, slot_to_time(slot))).collect(),
    };

    let slot_num = time_slots
        .iter()
        .find(|(_, time)| time == &req.time)
        .map(|(slot, _)| *slot);

    if slot_num.is_none() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid time slot"
        })));
    }

    let slot = slot_num.unwrap();

    let day_schedule = match day_str.as_ref() {
        "construction" => {
            if schedule_data.construction_schedule.is_none() {
                schedule_data.construction_schedule = Some(DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                });
            }
            schedule_data.construction_schedule.as_mut().unwrap()
        }
        "research" => {
            if schedule_data.research_schedule.is_none() {
                schedule_data.research_schedule = Some(DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                });
            }
            schedule_data.research_schedule.as_mut().unwrap()
        }
        "troops" => {
            if schedule_data.troops_schedule.is_none() {
                schedule_data.troops_schedule = Some(DaySchedule {
                    appointments: HashMap::new(),
                    unassigned: Vec::new(),
                });
            }
            schedule_data.troops_schedule.as_mut().unwrap()
        }
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid day"
            })))
        }
    };

    if let Some(ref player_str) = req.player {
        let player_str = player_str.trim();
        if !player_str.is_empty() {
            let (alliance, name) = if let Some(start) = player_str.find('[') {
                if let Some(end) = player_str.find(']') {
                    let alliance = player_str[start + 1..end].to_string();
                    let name = player_str[end + 1..].trim().to_string();
                    (alliance, name)
                } else {
                    ("".to_string(), player_str.to_string())
                }
            } else {
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
            day_schedule.appointments.remove(&slot);
        }
    } else {
        day_schedule.appointments.remove(&slot);
    }

    let scheduled_ids: Vec<String> = {
        let mut ids = HashSet::new();
        for appt in schedule_data
            .construction_schedule
            .as_ref()
            .iter()
            .flat_map(|s| s.appointments.values())
        {
            ids.insert(appt.player_id.clone());
        }
        for appt in schedule_data
            .research_schedule
            .as_ref()
            .iter()
            .flat_map(|s| s.appointments.values())
        {
            ids.insert(appt.player_id.clone());
        }
        for appt in schedule_data
            .troops_schedule
            .as_ref()
            .iter()
            .flat_map(|s| s.appointments.values())
        {
            ids.insert(appt.player_id.clone());
        }
        ids.into_iter().collect()
    };
    schedule_data.scheduled_player_ids = Some(scheduled_ids);

    {
        let mut schedules = state.schedules.lock().unwrap();
        schedules.insert(key.clone(), schedule_data.clone());
    }

    if let Err(e) = save_schedule(
        &state.data_dir,
        &account_name,
        server_number,
        &schedule_data,
    ) {
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

#[derive(Deserialize)]
pub struct ClearScheduleRequest {
    #[serde(default)]
    pub day: Option<String>,
}

/// Clear schedule endpoint - clear all days or a single day
pub async fn clear_schedule_api(
    path: web::Path<(String, u32)>,
    payload: Option<web::Json<ClearScheduleRequest>>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, server_number) = path.into_inner();
    let account_name = account_name.to_lowercase();

    if let (Some(session_account), Some(session_server)) = (
        session.get::<String>("account_name")?,
        session.get::<u32>("server_number")?,
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

    let day_filter = payload.and_then(|p| p.day.clone());

    let key = schedule_key(&account_name, server_number);
    let schedule_data = {
        let schedules = state.schedules.lock().unwrap();
        schedules
            .get(&key)
            .cloned()
            .or_else(|| load_schedule(&state.data_dir, &account_name, server_number))
    };

    if schedule_data.is_none() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Schedule already empty"
        })));
    }

    let mut schedule_data = schedule_data.unwrap();
    let empty_day = DaySchedule {
        appointments: HashMap::new(),
        unassigned: Vec::new(),
    };

    match day_filter.as_deref() {
        Some("construction") => {
            schedule_data.construction_schedule = Some(empty_day);
        }
        Some("research") => {
            schedule_data.research_schedule = Some(empty_day);
        }
        Some("troops") => {
            schedule_data.troops_schedule = Some(empty_day);
        }
        _ => {
            schedule_data.construction_schedule = Some(empty_day.clone());
            schedule_data.research_schedule = Some(empty_day.clone());
            schedule_data.troops_schedule = Some(empty_day);
        }
    }

    schedule_data.scheduled_player_ids = Some(
        derive_scheduled_player_ids(&schedule_data)
            .into_iter()
            .collect::<Vec<_>>(),
    );

    {
        let mut schedules = state.schedules.lock().unwrap();
        schedules.insert(key.clone(), schedule_data.clone());
    }

    if let Err(e) = save_schedule(
        &state.data_dir,
        &account_name,
        server_number,
        &schedule_data,
    ) {
        eprintln!("Warning: Failed to save schedule: {}", e);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": "Failed to save schedule"
        })));
    }

    let msg = match day_filter.as_deref() {
        Some(d) => format!("{} schedule cleared", d),
        _ => "All schedules cleared".to_string(),
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": msg
    })))
}
