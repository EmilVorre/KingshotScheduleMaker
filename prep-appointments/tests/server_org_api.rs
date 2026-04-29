//! Server Organisation + Tyrant form API tests (JSON backend).

use actix_web::{http::StatusCode, test, web, App};
use std::collections::HashMap;
use std::sync::Mutex;

use prep_appointments::web::{server_org, AppState};

fn test_state(data_dir: String) -> web::Data<AppState> {
    web::Data::new(AppState {
        accounts: Mutex::new(HashMap::new()),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(HashMap::new()),
        current_forms: Mutex::new(HashMap::new()),
        data_dir,
        pg: None,
        oauth_hmac_key: vec![0u8; 32],
    })
}

#[actix_web::test]
async fn tyrant_public_config_not_found() {
    let dir = std::env::temp_dir().join(format!("srv_org_nf_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let app = test::init_service(
        App::new().app_data(test_state(data_dir)).service(
            web::resource("/tyrant-form/{code}/api/config").route(web::get().to(server_org::tyrant_public_config)),
        ),
    )
    .await;

    let req = test::TestRequest::get().uri("/tyrant-form/noSuchCode/api/config").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn tyrant_submit_invalid_player_id() {
    let dir = std::env::temp_dir().join(format!("srv_org_sub_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let bundle_json = serde_json::json!({
        "workspaces": [{
            "id": "ws1",
            "display_name": "Test",
            "kingshot_server_number": 1,
            "owner_account_key": "o",
            "created_at": "2026-01-01T00:00:00Z"
        }],
        "members": [],
        "invites": [],
        "tyrant_forms": [{
            "id": "f1",
            "workspace_id": "ws1",
            "public_code": "ABCDEFGHIJKL",
            "config": { "alliances": ["[X]"], "include_non_of_above": false },
            "created_at": "2026-01-01T00:00:00Z"
        }],
        "tyrant_submissions": []
    });
    std::fs::write(
        format!("{}/domain_documents_server_org.json", data_dir),
        serde_json::to_string(&serde_json::json!({"bundle": bundle_json})).unwrap(),
    )
    .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(test_state(data_dir.clone()))
            .service(
                web::resource("/tyrant-form/{code}/api/submit")
                    .route(web::post().to(server_org::tyrant_public_submit)),
            ),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/tyrant-form/ABCDEFGHIJKL/api/submit")
        .set_json(serde_json::json!({
            "player_id": "abc",
            "player_name": "x",
            "alliance": "[X]",
            "archer": { "level_band": "level_1_9", "tg_band": "below_tg5" },
            "cavalry": { "level_band": "level_1_9", "tg_band": "below_tg5" },
            "infantry": { "level_band": "level_1_9", "tg_band": "below_tg5" },
            "auto_help_month_card_active": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    std::fs::remove_dir_all(&dir).ok();
}
