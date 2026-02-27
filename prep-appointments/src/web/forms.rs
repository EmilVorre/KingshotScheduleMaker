//! Form management handlers: create, config, submissions, player lookup, etc.

use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::form::{
    export_submission_to_csv, validate_submission, FormSubmission, FormSubmissionRequest,
};
use crate::kingshot_api;
use crate::parser::{has_player_submitted, load_appointments};

use super::persistence::{
    archive_old_forms, generate_form_code, get_current_form, list_old_forms, reopen_old_form,
    save_current_forms, save_form,
};
use super::state::{
    AppState, CreateFormRequest, FormConfig, FormData, FormStatsResponse, FormTimeSlotStats,
    UpdateFormConfigRequest,
};
use crate::schedule::calculate_time_slots;

/// Submit form by code (public)
pub async fn submit_form_by_code(
    path: web::Path<String>,
    req: web::Json<FormSubmissionRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();

    let forms = state.forms.lock().unwrap();
    let form_data = forms.get(&code).cloned();
    drop(forms);

    let (config, _server_number) = if let Some(fd) = form_data {
        (fd.config, fd.server_number)
    } else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form not found"
        })));
    };

    if let Err(err) = validate_submission(&req) {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": err
        })));
    }

    // Verify player is in the kingdom this form is for
    let expected_kingdom = config.kingdom_id.trim();

    if let Ok(player) = kingshot_api::fetch_player(req.player_id.trim()).await {
        if player.kid.trim() != expected_kingdom {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "This player is not in the kingdom this form is for"
            })));
        }
    }

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
        construction_tempered_truegold: req.construction_tempered_truegold,
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

    let current_forms_dir = format!("{}/current_forms", state.data_dir);
    std::fs::create_dir_all(&current_forms_dir)?;
    let csv_path = format!("{}/{}_submissions.csv", current_forms_dir, code);
    let csv_path = Path::new(&csv_path);

    if let Err(e) = export_submission_to_csv(
        &submission,
        csv_path,
        (
            &config.construction_times.start_time,
            config.construction_times.end_time.as_deref(),
        ),
        (
            &config.research_times.start_time,
            config.research_times.end_time.as_deref(),
        ),
        (
            &config.troops_times.start_time,
            config.troops_times.end_time.as_deref(),
        ),
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

/// Create form (admin)
pub async fn create_form(
    path: web::Path<(String, u32)>,
    session: Session,
    body: web::Json<CreateFormRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

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

    if session_account_name.to_lowercase() != url_account_name
        || session_server_number != server_number
    {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let accounts = state.accounts.lock().unwrap();
    if !accounts.contains_key(&url_account_name) {
        drop(accounts);
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Account not found"
        })));
    }
    drop(accounts);

    if body.alliances.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "At least one alliance must be specified"
        })));
    }

    if body.kingdom_id.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Kingdom ID is required"
        })));
    }

    let mut code = generate_form_code();
    let mut max_attempts = 100;
    loop {
        let forms = state.forms.lock().unwrap();
        let in_memory = forms.contains_key(&code);
        drop(forms);

        let current_forms_file = format!("{}/current_forms/{}.json", state.data_dir, code);
        let file_exists = Path::new(&current_forms_file).exists();

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
            break;
        }

        code = generate_form_code();
        max_attempts -= 1;
        if max_attempts <= 0 {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to generate unique form code after multiple attempts. Please try again."
            })));
        }
    }

    let form_name = body
        .name
        .clone()
        .unwrap_or_else(|| format!("Form {} {}", url_account_name, server_number));
    let created_at = chrono::Utc::now().to_rfc3339();
    let delete_date = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(14))
        .map(|d| d.format("%Y-%m-%d").to_string());

    let form_data = FormData {
        code: code.clone(),
        account_name: url_account_name.clone(),
        server_number,
        name: form_name,
        created_at,
        delete_date,
        config: FormConfig {
            alliances: body.alliances.clone(),
            include_non_of_above: body.include_non_of_above,
            construction_truegold_mode: body.construction_truegold_mode.clone(),
            construction_times: body.construction_times.clone(),
            research_times: body.research_times.clone(),
            troops_times: body.troops_times.clone(),
            predetermined_slots: body.predetermined_slots.clone(),
            intro_text: body.intro_text.clone(),
            support_person_name: body.support_person_name.clone(),
            kingdom_id: body.kingdom_id.trim().to_string(),
        },
    };

    archive_old_forms(&state.data_dir, &url_account_name, server_number).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to archive old forms: {}", e))
    })?;

    let mut forms = state.forms.lock().unwrap();
    forms.retain(|_, fd| {
        !(fd.account_name == url_account_name && fd.server_number == server_number)
    });
    forms.insert(code.clone(), form_data.clone());
    drop(forms);

    save_form(&state.data_dir, &form_data).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save form: {}", e))
    })?;

    let mut current_forms = state.current_forms.lock().unwrap();
    let key = format!("{}:{}", url_account_name, server_number);
    current_forms.insert(key, code.clone());
    save_current_forms(&state.data_dir, &current_forms).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save current forms: {}", e))
    })?;
    drop(current_forms);

    let form_url = format!("/form/{}", code);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Form created successfully",
        "code": code,
        "url": form_url
    })))
}

/// Update form config (predetermined slots)
pub async fn update_form_config(
    path: web::Path<(String, u32)>,
    session: Session,
    body: web::Json<UpdateFormConfigRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

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

    if session_account_name.to_lowercase() != url_account_name
        || session_server_number != server_number
    {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let mut forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let key = format!("{}:{}", url_account_name, server_number);

    let form_code = if let Some(code) = current_forms.get(&key) {
        code.clone()
    } else {
        drop(forms);
        drop(current_forms);
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "No current form found"
        })));
    };

    let mut form_data = if let Some(form) = forms.get(&form_code).cloned() {
        form
    } else {
        drop(forms);
        drop(current_forms);
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form not found"
        })));
    };

    drop(current_forms);

    form_data.config.predetermined_slots = body.predetermined_slots.clone();

    save_form(&state.data_dir, &form_data).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save form: {}", e))
    })?;

    forms.insert(form_code.clone(), form_data);
    drop(forms);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Form config updated successfully"
    })))
}

/// Check if player has submitted (public)
pub async fn check_submission_by_code(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (code, player_id) = path.into_inner();
    let player_id = player_id.trim();
    if player_id.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "has_submitted": false
        })));
    }
    let forms = state.forms.lock().unwrap();
    let form_data = forms.get(&code).cloned();
    drop(forms);
    let csv_path = if let Some(fd) = form_data {
        format!(
            "{}/current_forms/{}_submissions.csv",
            state.data_dir, fd.code
        )
    } else {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "has_submitted": false
        })));
    };
    let has_submitted = has_player_submitted(&csv_path, player_id);
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "has_submitted": has_submitted
    })))
}

/// Player lookup by ID (Kingshot API, public).
/// Verifies the player is in the kingdom the form was created for (kingdom_id).
pub async fn player_lookup_by_code(
    path: web::Path<(String, String)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (code, player_id) = path.into_inner();
    let player_id = player_id.trim();

    let expected_kingdom = {
        let forms = state.forms.lock().unwrap();
        forms
            .get(&code)
            .map(|f| f.config.kingdom_id.trim().to_string())
    };

    match kingshot_api::fetch_player(&player_id).await {
        Ok(player) => {
            let castle_level = kingshot_api::stove_lv_to_label(player.stove_lv);

            let kingdom_mismatch = if let Some(ref exp) = expected_kingdom {
                player.kid.trim() != exp.as_str()
            } else {
                false
            };

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": !kingdom_mismatch,
                "name": player.nickname,
                "player_id": player.fid,
                "avatar_image": player.avatar_image,
                "castle_level": castle_level,
                "kingdom": player.kid,
                "kingdom_mismatch": kingdom_mismatch,
                "error": if kingdom_mismatch { Some("This player is not in the kingdom this form is for") } else { None::<&str> }
            })))
        }
        Err(e) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": false,
            "error": e
        }))),
    }
}

/// Get form config by code (public)
pub async fn get_form_config_by_code(
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

/// Get form stats by code (public)
pub async fn get_form_stats_by_code(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();

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

    let current_forms_dir = format!("{}/current_forms", state.data_dir);
    let csv_path = format!("{}/{}_submissions.csv", current_forms_dir, code);

    let construction_slots = calculate_time_slots(
        &config.construction_times.start_time,
        config.construction_times.end_time.as_deref(),
    );
    let research_slots = calculate_time_slots(
        &config.research_times.start_time,
        config.research_times.end_time.as_deref(),
    );
    let troops_slots = calculate_time_slots(
        &config.troops_times.start_time,
        config.troops_times.end_time.as_deref(),
    );

    let construction_slots_ref: Vec<(u8, String)> = construction_slots.clone();
    let research_slots_ref: Vec<(u8, String)> = research_slots.clone();
    let troops_slots_ref: Vec<(u8, String)> = troops_slots.clone();

    let entries = match load_appointments(
        &csv_path,
        Some(&construction_slots_ref),
        Some(&research_slots_ref),
        Some(&troops_slots_ref),
    ) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "Error loading form submissions CSV from {}: {}",
                csv_path, e
            );
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

    let mut construction_time_slot_popularity: HashMap<String, FormTimeSlotStats> = HashMap::new();
    for (_, time) in &construction_slots {
        construction_time_slot_popularity.insert(time.clone(), FormTimeSlotStats { requests: 0 });
    }
    let mut research_time_slot_popularity: HashMap<String, FormTimeSlotStats> = HashMap::new();
    for (_, time) in &research_slots {
        research_time_slot_popularity.insert(time.clone(), FormTimeSlotStats { requests: 0 });
    }
    let mut troops_time_slot_popularity: HashMap<String, FormTimeSlotStats> = HashMap::new();
    for (_, time) in &troops_slots {
        troops_time_slot_popularity.insert(time.clone(), FormTimeSlotStats { requests: 0 });
    }

    let construction_slot_to_time: HashMap<u8, String> = construction_slots
        .iter()
        .map(|(s, t)| (*s, t.clone()))
        .collect();
    let research_slot_to_time: HashMap<u8, String> = research_slots
        .iter()
        .map(|(s, t)| (*s, t.clone()))
        .collect();
    let troops_slot_to_time: HashMap<u8, String> =
        troops_slots.iter().map(|(s, t)| (*s, t.clone())).collect();

    for entry in entries {
        for slot in &entry.construction_available_slots {
            if let Some(time) = construction_slot_to_time.get(slot) {
                if let Some(slot_stats) = construction_time_slot_popularity.get_mut(time) {
                    slot_stats.requests += 1;
                }
            }
        }
        for slot in &entry.research_available_slots {
            if let Some(time) = research_slot_to_time.get(slot) {
                if let Some(slot_stats) = research_time_slot_popularity.get_mut(time) {
                    slot_stats.requests += 1;
                }
            }
        }
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

/// Get current form info for account (admin)
pub async fn get_current_form_info(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

    let authenticated = {
        let session_account_name: Option<String> = session.get("account_name").ok().flatten();
        let session_server_number: Option<u32> = session.get("server_number").ok().flatten();

        if let (Some(session_account_name), Some(session_server_number)) =
            (session_account_name, session_server_number)
        {
            session_account_name == url_account_name && session_server_number == server_number
        } else {
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

    let forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let current_form = get_current_form(&forms, &current_forms, &url_account_name, server_number);
    drop(forms);
    drop(current_forms);

    if let Some(form) = current_form {
        let form_url = format!("/form/{}", form.code);
        let form_csv_path = format!(
            "{}/current_forms/{}_submissions.csv",
            state.data_dir, form.code
        );
        let submissions_count = if Path::new(&form_csv_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&form_csv_path) {
                content.lines().filter(|l| l.contains('/')).count()
            } else {
                0
            }
        } else {
            0
        };

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "form": {
                "code": form.code,
                "name": form.name,
                "created_at": form.created_at,
                "delete_date": form.delete_date,
                "url": form_url,
                "submissions_count": submissions_count,
                "config": {
                    "alliances": form.config.alliances,
                    "include_non_of_above": form.config.include_non_of_above,
                    "construction_times": form.config.construction_times,
                    "research_times": form.config.research_times,
                    "troops_times": form.config.troops_times,
                    "predetermined_slots": form.config.predetermined_slots,
                    "intro_text": form.config.intro_text,
                    "support_person_name": form.config.support_person_name,
                    "kingdom_id": form.config.kingdom_id
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

/// List old/archived forms for account (admin)
pub async fn list_old_forms_api(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

    let authenticated = {
        let session_account_name: Option<String> = session.get("account_name").ok().flatten();
        let session_server_number: Option<u32> = session.get("server_number").ok().flatten();

        if let (Some(session_account_name), Some(session_server_number)) =
            (session_account_name, session_server_number)
        {
            session_account_name == url_account_name && session_server_number == server_number
        } else {
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

    let old_forms = list_old_forms(&state.data_dir, &url_account_name, server_number);
    let items: Vec<serde_json::Value> = old_forms
        .into_iter()
        .map(|(archive_name, form)| {
            serde_json::json!({
                "archive_name": archive_name,
                "code": form.code,
                "name": form.name,
                "created_at": form.created_at,
                "delete_date": form.delete_date
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "old_forms": items
    })))
}

/// Reopen an archived form (admin)
pub async fn reopen_form_api(
    path: web::Path<(String, u32)>,
    session: Session,
    body: Option<web::Json<serde_json::Value>>,
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

    let archive_name = body
        .as_ref()
        .and_then(|j| j.get("archive_name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            actix_web::error::ErrorBadRequest("archive_name required in body")
        })?;

    let authenticated = {
        let session_account_name: Option<String> = session.get("account_name").ok().flatten();
        let session_server_number: Option<u32> = session.get("server_number").ok().flatten();

        if let (Some(session_account_name), Some(session_server_number)) =
            (session_account_name, session_server_number)
        {
            session_account_name == url_account_name && session_server_number == server_number
        } else {
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

    let form_data = match reopen_old_form(
        &state.data_dir,
        &url_account_name,
        server_number,
        archive_name,
    ) {
        Ok(f) => f,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })));
        }
    };

    let mut forms = state.forms.lock().unwrap();
    forms.insert(form_data.code.clone(), form_data.clone());
    drop(forms);

    let mut current_forms = state.current_forms.lock().unwrap();
    let key = format!("{}:{}", url_account_name, server_number);
    current_forms.insert(key, form_data.code.clone());
    save_current_forms(&state.data_dir, &current_forms).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
    })?;
    drop(current_forms);

    let form_url = format!("/form/{}", form_data.code);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Form reopened successfully",
        "code": form_data.code,
        "url": form_url
    })))
}

/// Get player by ID from form submissions
pub async fn get_player_by_id(
    path: web::Path<(String, u32, String)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number, player_id) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

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

    if session_account_name.to_lowercase() != url_account_name
        || session_server_number != server_number
    {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Not authorized"
        })));
    }

    let forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let csv_path = if let Some(current_form) =
        get_current_form(&forms, &current_forms, &url_account_name, server_number)
    {
        format!(
            "{}/current_forms/{}_submissions.csv",
            state.data_dir, current_form.code
        )
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

    if !Path::new(&csv_path).exists() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Form submissions not found"
        })));
    }

    let entries = match load_appointments(&csv_path, None, None, None) {
        Ok(e) => e,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to load form submissions"
            })));
        }
    };

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

/// Download form CSV
pub async fn download_form_csv(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

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

    if session_account_name.to_lowercase() != url_account_name
        || session_server_number != server_number
    {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        })));
    }

    let forms = state.forms.lock().unwrap();
    let current_forms = state.current_forms.lock().unwrap();
    let mut current_form =
        get_current_form(&forms, &current_forms, &url_account_name, server_number);
    drop(forms);
    drop(current_forms);

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
                                if form_account_name == url_account_name
                                    && form_data.server_number == server_number
                                {
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
        let csv_path = format!(
            "{}/current_forms/{}_submissions.csv",
            state.data_dir, form.code
        );
        if Path::new(&csv_path).exists() {
            if let Ok(csv_content) = std::fs::read_to_string(&csv_path) {
                let filename = format!(
                    "{}_submissions_{}.csv",
                    form.code,
                    chrono::Utc::now().format("%Y%m%d_%H%M%S")
                );
                return Ok(HttpResponse::Ok()
                    .content_type("text/csv")
                    .append_header((
                        "Content-Disposition",
                        format!("attachment; filename=\"{}\"", filename),
                    ))
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

/// Get previous form config (admin)
pub async fn get_previous_form_config(
    path: web::Path<(String, u32)>,
    session: Session,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (url_account_name, server_number) = path.into_inner();
    let url_account_name = url_account_name.to_lowercase();

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

    if session_account_name != url_account_name || session_server_number != server_number {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "success": false,
            "error": "Access denied"
        })));
    }

    let account_name = url_account_name;

    let forms = state.forms.lock().unwrap();
    let mut previous_form: Option<FormData> = None;
    for form_data in forms.values() {
        if form_data.account_name == account_name && form_data.server_number == server_number {
            match &previous_form {
                None => previous_form = Some(form_data.clone()),
                Some(current) => {
                    if let (Ok(current_time), Ok(new_time)) = (
                        chrono::DateTime::parse_from_rfc3339(&current.created_at),
                        chrono::DateTime::parse_from_rfc3339(&form_data.created_at),
                    ) {
                        if new_time > current_time {
                            previous_form = Some(form_data.clone());
                        }
                    } else {
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

/// Get form submissions
pub async fn get_form_submissions(
    path: web::Path<(String, u32)>,
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
    let form_csv_path = format!(
        "{}/current_forms/{}_submissions.csv",
        state.data_dir, current_form.code
    );

    if !Path::new(&form_csv_path).exists() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "submissions": []
        })));
    }

    let mut reader = csv::Reader::from_path(&form_csv_path).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to read CSV: {}", e))
    })?;

    let headers = reader
        .headers()
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to read CSV headers: {}", e))
        })?
        .clone();

    let mut submissions = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to parse CSV record: {}", e))
        })?;

        let first_field = record.get(0).unwrap_or("");
        if !first_field.contains('/') || first_field.len() < 8 {
            continue;
        }

        let mut submission = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            let header = headers
                .get(i)
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
