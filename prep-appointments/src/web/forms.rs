//! Form management handlers: create, config, submissions, player lookup, etc.

use actix_session::Session;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use std::collections::HashMap;

use crate::form::{validate_submission, FormSubmission, FormSubmissionRequest};
use crate::kingshot_api;
use crate::parser::load_appointments_from_submissions;

use super::persistence::{
    archive_old_forms, count_form_submissions, generate_form_code, get_current_form,
    has_player_submission, list_old_forms, load_form_submissions, reopen_old_form,
    save_current_forms, save_form, save_form_submission,
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

    if let Err(e) = save_form_submission(&state.data_dir, &code, &submission).await {
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
        if !in_memory {
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
            construction_day_slot: body.construction_day_slot.clone(),
            research_day_slot: body.research_day_slot.clone(),
            predetermined_slots: body.predetermined_slots.clone(),
            intro_text: body.intro_text.clone(),
            support_person_name: body.support_person_name.clone(),
            kingdom_id: body.kingdom_id.trim().to_string(),
        },
    };

    if let Err(e) = archive_old_forms(&state.data_dir, &url_account_name, server_number).await {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to archive old forms: {}", e)
        })));
    }

    {
        let mut forms = state.forms.lock().unwrap();
        forms.retain(|_, fd| {
            !(fd.account_name == url_account_name && fd.server_number == server_number)
        });
        forms.insert(code.clone(), form_data.clone());
    }

    if let Err(e) = save_form(&state.data_dir, &form_data).await {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save form: {}", e)
        })));
    }

    let cf_snapshot = {
        let mut current_forms = state.current_forms.lock().unwrap();
        let key = format!("{}:{}", url_account_name, server_number);
        current_forms.insert(key, code.clone());
        current_forms.clone()
    };
    if let Err(e) = save_current_forms(&state.data_dir, &cf_snapshot).await {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to save current forms: {}", e)
        })));
    }

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

    let key = format!("{}:{}", url_account_name, server_number);
    let form_code = {
        let current_forms = state.current_forms.lock().unwrap();
        match current_forms.get(&key) {
            Some(code) => code.clone(),
            None => {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "error": "No current form found"
                })))
            }
        }
    };

    let mut form_data = {
        let forms = state.forms.lock().unwrap();
        match forms.get(&form_code).cloned() {
            Some(f) => f,
            None => {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "success": false,
                    "error": "Form not found"
                })))
            }
        }
    };

    form_data.config.predetermined_slots = body.predetermined_slots.clone();

    save_form(&state.data_dir, &form_data).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to save form: {}", e))
    })?;

    {
        let mut forms = state.forms.lock().unwrap();
        forms.insert(form_code.clone(), form_data);
    }

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
    let form_code = if let Some(fd) = form_data {
        fd.code
    } else {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "has_submitted": false
        })));
    };
    let has_submitted = has_player_submission(&state.data_dir, &form_code, player_id).await;
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

    let submissions = load_form_submissions(&state.data_dir, &code).await;
    let entries = load_appointments_from_submissions(
        &submissions,
        Some(&construction_slots_ref),
        Some(&research_slots_ref),
        Some(&troops_slots_ref),
    );

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
        let submissions_count = count_form_submissions(&state.data_dir, &form.code).await;

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

    let old_forms = list_old_forms(&state.data_dir, &url_account_name, server_number).await;
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
        .ok_or_else(|| actix_web::error::ErrorBadRequest("archive_name required in body"))?;

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
    )
    .await
    {
        Ok(f) => f,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })));
        }
    };

    {
        let mut forms = state.forms.lock().unwrap();
        forms.insert(form_data.code.clone(), form_data.clone());
    }

    let cf_snapshot = {
        let mut current_forms = state.current_forms.lock().unwrap();
        let key = format!("{}:{}", url_account_name, server_number);
        current_forms.insert(key, form_data.code.clone());
        current_forms.clone()
    };
    save_current_forms(&state.data_dir, &cf_snapshot)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to save: {}", e))
        })?;

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
    let form_code = if let Some(current_form) =
        get_current_form(&forms, &current_forms, &url_account_name, server_number)
    {
        current_form.code
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

    let submissions = load_form_submissions(&state.data_dir, &form_code).await;

    if let Some(entry) = submissions.iter().find(|e| e.player_id == player_id) {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "player": {
                "player_id": entry.player_id,
                "name": entry.character_name,
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
    let current_form = get_current_form(&forms, &current_forms, &url_account_name, server_number);
    drop(forms);
    drop(current_forms);

    let Some(form) = current_form else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "No current form found"
        })));
    };

    let submissions = load_form_submissions(&state.data_dir, &form.code).await;
    if submissions.is_empty() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "No submissions found"
        })));
    }

    let mut writer = csv::Writer::from_writer(Vec::<u8>::new());
    writer
        .write_record([
            "timestamp",
            "alliance",
            "custom_alliance",
            "character_name",
            "player_id",
            "submission_type",
            "wants_construction",
            "construction_speedups",
            "construction_truegold",
            "construction_tempered_truegold",
            "construction_time_slots",
            "wants_research",
            "research_speedups",
            "research_truegold_dust",
            "research_time_slots",
            "wants_troops",
            "troops_speedups",
            "troops_time_slots",
            "additional_notes",
            "suggestions",
        ])
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("CSV write failed: {}", e))
        })?;
    for s in submissions {
        writer
            .write_record([
                s.timestamp,
                s.alliance,
                s.custom_alliance.unwrap_or_default(),
                s.character_name,
                s.player_id,
                s.submission_type,
                s.wants_construction.to_string(),
                s.construction_speedups.unwrap_or(0).to_string(),
                s.construction_truegold.unwrap_or(0).to_string(),
                s.construction_tempered_truegold.unwrap_or(0).to_string(),
                s.construction_time_slots
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                s.wants_research.to_string(),
                s.research_speedups.unwrap_or(0).to_string(),
                s.research_truegold_dust.unwrap_or(0).to_string(),
                s.research_time_slots
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                s.wants_troops.to_string(),
                s.troops_speedups.unwrap_or(0).to_string(),
                s.troops_time_slots
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                s.additional_notes.unwrap_or_default(),
                s.suggestions.unwrap_or_default(),
            ])
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!("CSV write failed: {}", e))
            })?;
    }
    let bytes = writer.into_inner().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("CSV finalize failed: {}", e))
    })?;
    let csv_content = String::from_utf8(bytes).map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("CSV encoding failed: {}", e))
    })?;
    let filename = format!(
        "{}_submissions_{}.csv",
        form.code,
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    Ok(HttpResponse::Ok()
        .content_type("text/csv")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        ))
        .body(csv_content))
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
    let raw_submissions = load_form_submissions(&state.data_dir, &current_form.code).await;
    if raw_submissions.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "submissions": []
        })));
    }

    let mut submissions = Vec::new();
    for s in raw_submissions {
        submissions.push(serde_json::json!({
            "timestamp": s.timestamp,
            "alliance": s.alliance,
            "custom_alliance": s.custom_alliance,
            "character_name": s.character_name,
            "player_id": s.player_id,
            "submission_type": s.submission_type,
            "wants_construction": s.wants_construction,
            "construction_speedups": s.construction_speedups,
            "construction_truegold": s.construction_truegold,
            "construction_tempered_truegold": s.construction_tempered_truegold,
            "construction_time_slots": s.construction_time_slots,
            "wants_research": s.wants_research,
            "research_speedups": s.research_speedups,
            "research_truegold_dust": s.research_truegold_dust,
            "research_time_slots": s.research_time_slots,
            "wants_troops": s.wants_troops,
            "troops_speedups": s.troops_speedups,
            "troops_time_slots": s.troops_time_slots,
            "additional_notes": s.additional_notes,
            "suggestions": s.suggestions
        }));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "submissions": submissions
    })))
}
