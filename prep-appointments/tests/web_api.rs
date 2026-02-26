//! Integration tests for web API handlers.

use actix_web::{test, web, App};
use std::collections::HashMap;
use std::sync::Mutex;

use prep_appointments::form::FormSubmissionRequest;
use prep_appointments::web::{forms, AppState, FormConfig, FormData};

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
            config,
        },
    );
    web::Data::new(AppState {
        accounts: Mutex::new(HashMap::new()),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms),
        current_forms: Mutex::new(HashMap::new()),
        data_dir,
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

    let csv_path =
        std::path::Path::new(&data_dir).join("current_forms/TESTCODE123_submissions.csv");
    assert!(csv_path.exists(), "CSV file should be created");
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
