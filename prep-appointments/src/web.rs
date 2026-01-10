use actix_web::{web, App, HttpServer, HttpResponse, Result, HttpRequest, middleware};
use actix_files::Files;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use crate::parser::{load_appointments, AppointmentEntry};
use crate::schedule::{schedule_construction_day, schedule_research_day, schedule_troops_day, DaySchedule, slot_to_time};
use crate::display::format_player_name;

// In-memory storage for schedules (in production, use a database)
pub struct AppState {
    pub construction_schedule: Mutex<Option<DaySchedule>>,
    pub research_schedule: Mutex<Option<DaySchedule>>,
    pub troops_schedule: Mutex<Option<DaySchedule>>,
    pub entries: Mutex<Option<Vec<AppointmentEntry>>>,
    pub admin_password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    password: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    alliance_counts: HashMap<String, AllianceStats>,
    time_slot_popularity: HashMap<String, TimeSlotStats>,
}

#[derive(Serialize)]
pub struct AllianceStats {
    construction_requests: u32,
    research_requests: u32,
    troops_requests: u32,
}

#[derive(Serialize)]
pub struct TimeSlotStats {
    construction_requests: u32,
    research_requests: u32,
    troops_requests: u32,
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

// Admin login endpoint
async fn admin_login(
    req: web::Json<LoginRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    if req.password == state.admin_password {
        Ok(HttpResponse::Ok().json(serde_json::json!({"success": true})))
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({"success": false, "error": "Invalid password"})))
    }
}

// Admin CSV upload endpoint
async fn admin_upload(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    // Check password from header
    let password = req
        .headers()
        .get("X-Admin-Password")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    if password != state.admin_password {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({"success": false, "error": "Unauthorized"})));
    }

    // Save uploaded CSV
    let csv_path = "uploaded_data.csv";
    std::fs::write(csv_path, &body)
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Failed to save file: {}", e)))?;

    // Process the CSV
    match load_appointments(csv_path) {
        Ok(entries) => {
            let construction_schedule = schedule_construction_day(&entries);
            let research_schedule = schedule_research_day(&entries, &construction_schedule);
            let troops_schedule = schedule_troops_day(&entries);

            // Update state
            *state.entries.lock().unwrap() = Some(entries);
            *state.construction_schedule.lock().unwrap() = Some(construction_schedule);
            *state.research_schedule.lock().unwrap() = Some(research_schedule);
            *state.troops_schedule.lock().unwrap() = Some(troops_schedule);

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
async fn get_stats(state: web::Data<AppState>) -> Result<HttpResponse> {
    let entries = state.entries.lock().unwrap();
    
    if let Some(ref entries) = *entries {
        let mut alliance_counts: HashMap<String, AllianceStats> = HashMap::new();
        let mut time_slot_popularity: HashMap<String, TimeSlotStats> = HashMap::new();

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

        Ok(HttpResponse::Ok().json(StatsResponse {
            alliance_counts,
            time_slot_popularity,
        }))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "No data available"})))
    }
}

// Schedule endpoint
async fn get_schedule(
    day: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let day_str = day.as_str();
    let schedule = match day_str {
        "construction" => state.construction_schedule.lock().unwrap(),
        "research" => state.research_schedule.lock().unwrap(),
        "troops" => state.troops_schedule.lock().unwrap(),
        _ => return Ok(HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid day"}))),
    };

    if let Some(ref schedule) = *schedule {
        let day_name = match day_str {
            "construction" => "Construction Day",
            "research" => "Research Day",
            "troops" => "Troops Training Day",
            _ => "Unknown",
        };

        let mut appointments = Vec::new();
        for slot in 1..=49 {
            let time = slot_to_time(slot);
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
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "Schedule not available"})))
    }
}

// HTML page handlers
async fn index() -> Result<HttpResponse> {
    let html = include_str!("../templates/index.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn admin_page() -> Result<HttpResponse> {
    let html = include_str!("../templates/admin.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn stats_page() -> Result<HttpResponse> {
    let html = include_str!("../templates/stats.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

async fn schedules_page() -> Result<HttpResponse> {
    let html = include_str!("../templates/schedules.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

pub async fn start_server(port: u16, admin_password: String) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        construction_schedule: Mutex::new(None),
        research_schedule: Mutex::new(None),
        troops_schedule: Mutex::new(None),
        entries: Mutex::new(None),
        admin_password,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .service(Files::new("/static", "static").show_files_listing())
            .route("/", web::get().to(index))
            .route("/admin", web::get().to(admin_page))
            .route("/stats", web::get().to(stats_page))
            .route("/schedules", web::get().to(schedules_page))
            .route("/api/login", web::post().to(admin_login))
            .route("/api/upload", web::post().to(admin_upload))
            .route("/api/stats", web::get().to(get_stats))
            .service(web::resource("/api/schedule/{day}").route(web::get().to(get_schedule)))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

