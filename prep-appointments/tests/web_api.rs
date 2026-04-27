//! Integration tests for web API handlers.

use actix_web::{test, web, App};
use std::collections::HashMap;
use std::sync::Mutex;

use prep_appointments::form::FormSubmissionRequest;
use prep_appointments::schedule::types::ScheduledAppointment;
use prep_appointments::schedule::DaySchedule;
use prep_appointments::web::{
    forms, load_form_submissions, oauth_state, save_schedule, schedule, AppState, FormConfig,
    FormData, ScheduleData,
};

fn make_test_app_state(data_dir: String) -> web::Data<AppState> {
    let mut config = FormConfig::default();
    config.kingdom_id = "1".to_string();
    let mut forms = HashMap::new();
    forms.insert(
        "TESTCODE123".to_string(),
        FormData {
            code: "TESTCODE123".to_string(),
            account_name: "testacct".to_string(),
            server_number: 1,
            name: "Test Form".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            delete_date: None,
            config,
        },
    );
    web::Data::new(AppState {
        accounts: Mutex::new(HashMap::new()),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms),
        current_forms: Mutex::new(HashMap::new()),
        data_dir,
        oauth_state_cache: oauth_state::OAuthStateCache::new(),
        pending_oauth_cache: oauth_state::PendingOAuthCache::new(),
    })
}

#[actix_web::test]
async fn test_get_form_config_by_code_found() {
    let dir = std::env::temp_dir().join("test_web_api_config");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let app_state = make_test_app_state(data_dir.clone());
    let app = test::init_service(
        App::new().app_data(app_state).service(
            web::resource("/form/{code}/api/config")
                .route(web::get().to(forms::get_form_config_by_code)),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/form/TESTCODE123/api/config")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_get_form_config_by_code_not_found() {
    let dir = std::env::temp_dir().join("test_web_api_config_404");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let app_state = make_test_app_state(data_dir);
    let app = test::init_service(
        App::new().app_data(app_state).service(
            web::resource("/form/{code}/api/config")
                .route(web::get().to(forms::get_form_config_by_code)),
        ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/form/NOTEXIST/api/config")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_submit_form_by_code_success() {
    let dir = std::env::temp_dir().join("test_web_api_submit");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let app_state = make_test_app_state(data_dir.clone());
    let app = test::init_service(App::new().app_data(app_state).service(
        web::resource("/form/{code}/api/submit").route(web::post().to(forms::submit_form_by_code)),
    ))
    .await;

    let req_body = FormSubmissionRequest {
        alliance: "TestAlliance".to_string(),
        custom_alliance: None,
        character_name: "TestPlayer".to_string(),
        player_id: "12345678".to_string(),
        submission_type: "New submission".to_string(),
        wants_construction: true,
        construction_speedups: Some(10),
        construction_truegold: Some(5),
        construction_tempered_truegold: None,
        construction_time_slots: vec![1, 2, 3, 4, 5],
        wants_research: false,
        research_speedups: None,
        research_truegold_dust: None,
        research_time_slots: vec![],
        wants_troops: false,
        troops_speedups: None,
        troops_time_slots: vec![],
        additional_notes: None,
        suggestions: None,
    };

    let req = test::TestRequest::post()
        .uri("/form/TESTCODE123/api/submit")
        .set_json(&req_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let submissions = load_form_submissions(&data_dir, "TESTCODE123");
    assert_eq!(
        submissions.len(),
        1,
        "One submission should be persisted for the form"
    );
    assert_eq!(submissions[0].player_id, "12345678");
    assert_eq!(submissions[0].character_name, "TestPlayer");
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_submit_form_by_code_validation_error() {
    let dir = std::env::temp_dir().join("test_web_api_submit_invalid");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let app_state = make_test_app_state(data_dir);
    let app = test::init_service(App::new().app_data(app_state).service(
        web::resource("/form/{code}/api/submit").route(web::post().to(forms::submit_form_by_code)),
    ))
    .await;

    let req_body = FormSubmissionRequest {
        alliance: "TestAlliance".to_string(),
        custom_alliance: None,
        character_name: "".to_string(), // Invalid: empty name
        player_id: "12345678".to_string(),
        submission_type: "New submission".to_string(),
        wants_construction: true,
        construction_speedups: Some(10),
        construction_truegold: Some(5),
        construction_tempered_truegold: None,
        construction_time_slots: vec![1, 2, 3, 4, 5],
        wants_research: false,
        research_speedups: None,
        research_truegold_dust: None,
        research_time_slots: vec![],
        wants_troops: false,
        troops_speedups: None,
        troops_time_slots: vec![],
        additional_notes: None,
        suggestions: None,
    };

    let req = test::TestRequest::post()
        .uri("/form/TESTCODE123/api/submit")
        .set_json(&req_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
}

// ============ Public schedule by form code ============

fn make_schedule_app_state(
    data_dir: String,
    forms_map: HashMap<String, FormData>,
) -> web::Data<AppState> {
    web::Data::new(AppState {
        accounts: Mutex::new(HashMap::new()),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms_map),
        current_forms: Mutex::new(HashMap::new()),
        data_dir,
        oauth_state_cache: oauth_state::OAuthStateCache::new(),
        pending_oauth_cache: oauth_state::PendingOAuthCache::new(),
    })
}

#[actix_web::test]
async fn test_get_schedule_by_form_code_success() {
    let dir = std::env::temp_dir().join("test_public_schedule_ok");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let mut config = FormConfig::default();
    config.kingdom_id = "1".to_string();
    config.construction_times.start_time = "23:45".to_string();
    config.research_times.start_time = "23:45".to_string();
    config.troops_times.start_time = "23:45".to_string();

    let mut forms = HashMap::new();
    forms.insert(
        "SCHEDFORM1".to_string(),
        FormData {
            code: "SCHEDFORM1".to_string(),
            account_name: "testacct".to_string(),
            server_number: 140,
            name: "Schedule Form".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            delete_date: None,
            config: config.clone(),
        },
    );

    let mut cons_appointments = HashMap::new();
    cons_appointments.insert(
        1u8,
        ScheduledAppointment {
            player_id: "12345".to_string(),
            name: "TestPlayer".to_string(),
            alliance: "COB".to_string(),
            slot: 1,
            priority_score: 100,
        },
    );
    let schedule_data = ScheduleData {
        construction_schedule: Some(DaySchedule {
            appointments: cons_appointments,
            unassigned: vec![],
        }),
        research_schedule: None,
        troops_schedule: None,
        entries: None,
        scheduled_player_ids: None,
    };
    save_schedule(&data_dir, "testacct", 140, &schedule_data).unwrap();

    let app_state = make_schedule_app_state(data_dir.clone(), forms);
    let app = test::init_service(App::new().app_data(app_state).route(
        "/api/public-schedule/{account_name}/{form_code}/{day}",
        web::get().to(schedule::get_schedule_by_form_code),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public-schedule/testacct/SCHEDFORM1/construction")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "Expected 200, got {}",
        resp.status()
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["day_name"], "Construction Day");
    assert!(body["appointments"].is_array());
    let appointments = body["appointments"].as_array().unwrap();
    assert!(!appointments.is_empty());
    assert_eq!(appointments[0]["player"], "[COB] TestPlayer");
    assert_eq!(appointments[0]["is_empty"], false);

    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_get_schedule_by_form_code_form_not_found() {
    let dir = std::env::temp_dir().join("test_public_schedule_404");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let mut forms = HashMap::new();
    let mut config = FormConfig::default();
    config.kingdom_id = "1".to_string();
    forms.insert(
        "EXISTS".to_string(),
        FormData {
            code: "EXISTS".to_string(),
            account_name: "testacct".to_string(),
            server_number: 1,
            name: "Exists".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            delete_date: None,
            config,
        },
    );

    let app_state = make_schedule_app_state(data_dir, forms);
    let app = test::init_service(App::new().app_data(app_state).route(
        "/api/public-schedule/{account_name}/{form_code}/{day}",
        web::get().to(schedule::get_schedule_by_form_code),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public-schedule/testacct/NOTEXIST/construction")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn test_get_schedule_by_form_code_account_mismatch() {
    let dir = std::env::temp_dir().join("test_public_schedule_mismatch");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let mut forms = HashMap::new();
    let mut config = FormConfig::default();
    config.kingdom_id = "1".to_string();
    forms.insert(
        "OTHERACCT".to_string(),
        FormData {
            code: "OTHERACCT".to_string(),
            account_name: "otheracct".to_string(),
            server_number: 1,
            name: "Other".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            delete_date: None,
            config,
        },
    );

    let app_state = make_schedule_app_state(data_dir, forms);
    let app = test::init_service(App::new().app_data(app_state).route(
        "/api/public-schedule/{account_name}/{form_code}/{day}",
        web::get().to(schedule::get_schedule_by_form_code),
    ))
    .await;

    // Form code "OTHERACCT" belongs to otheracct, but we're requesting as testacct
    let req = test::TestRequest::get()
        .uri("/api/public-schedule/testacct/OTHERACCT/construction")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn test_get_schedule_by_form_code_invalid_day() {
    let dir = std::env::temp_dir().join("test_public_schedule_invalid_day");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let mut forms = HashMap::new();
    let mut config = FormConfig::default();
    config.kingdom_id = "1".to_string();
    forms.insert(
        "VALID".to_string(),
        FormData {
            code: "VALID".to_string(),
            account_name: "testacct".to_string(),
            server_number: 1,
            name: "Valid".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            delete_date: None,
            config,
        },
    );

    let app_state = make_schedule_app_state(data_dir, forms);
    let app = test::init_service(App::new().app_data(app_state).route(
        "/api/public-schedule/{account_name}/{form_code}/{day}",
        web::get().to(schedule::get_schedule_by_form_code),
    ))
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public-schedule/testacct/VALID/invalid")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
}
