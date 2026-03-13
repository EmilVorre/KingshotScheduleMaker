//! Integration tests for alliance organisation APIs (Swordland, Tri Alliance, alliances).

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::{test, web, App};
use std::collections::HashMap;
use std::sync::Mutex;

use prep_appointments::web::{alliances, oauth_state, swordland, tri_alliance, AppState};

fn make_alliance_app_state(data_dir: String) -> web::Data<AppState> {
    let mut accounts = HashMap::new();
    accounts.insert(
        "devtest".to_string(),
        prep_appointments::web::Account {
            account_name: "devtest".to_string(),
            server_number: 1,
            password: String::new(),
            in_game_name: "Dev Tester".to_string(),
            player_id: None,
            oauth_provider: None,
            oauth_id: None,
            admin: true,
            alliance_access: true,
            alliance_id: Some("abc123".to_string()),
            alliance_tag: Some("COB".to_string()),
            alliance_name: Some("Slaughterhouse".to_string()),
            friend_code: Some("DEVTEST12345".to_string()),
        },
    );
    web::Data::new(AppState {
        accounts: Mutex::new(accounts),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(HashMap::new()),
        current_forms: Mutex::new(HashMap::new()),
        data_dir,
        oauth_state_cache: oauth_state::OAuthStateCache::new(),
        pending_oauth_cache: oauth_state::PendingOAuthCache::new(),
    })
}

#[actix_web::test]
async fn test_list_alliances_unauthorized_without_session() {
    let dir = std::env::temp_dir().join("test_list_alliances_unauth");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let secret_key = Key::generate();
    let app_state = make_alliance_app_state(data_dir.clone());
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key,
            ))
            .service(
                web::resource("/{account_name}/{server}/api/alliances")
                    .route(web::get().to(alliances::list_alliances)),
            ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/devtest/1/api/alliances")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_get_swordland_unauthorized_without_session() {
    let dir = std::env::temp_dir().join("test_swordland_unauth");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let secret_key = Key::generate();
    let app_state = make_alliance_app_state(data_dir.clone());
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key,
            ))
            .service(
                web::resource("/{account_name}/{server}/api/alliances/{alliance_slug}/swordland")
                    .route(web::get().to(swordland::get_swordland)),
            ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/devtest/1/api/alliances/slaughterhouse/swordland")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_put_swordland_unauthorized_without_session() {
    let dir = std::env::temp_dir().join("test_put_swordland_unauth");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let secret_key = Key::generate();
    let app_state = make_alliance_app_state(data_dir.clone());
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key,
            ))
            .service(
                web::resource("/{account_name}/{server}/api/alliances/{alliance_slug}/swordland")
                    .route(web::get().to(swordland::get_swordland))
                    .route(web::put().to(swordland::set_swordland)),
            ),
    )
    .await;

    let legions = serde_json::json!([
        {"name": "Legion 1", "member_ids": [], "filler_ids": []},
        {"name": "Legion 2", "member_ids": [], "filler_ids": []}
    ]);
    let req = test::TestRequest::put()
        .uri("/devtest/1/api/alliances/slaughterhouse/swordland")
        .set_json(&serde_json::json!({ "legions": legions }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
    std::fs::remove_dir_all(&dir).ok();
}

#[actix_web::test]
async fn test_get_tri_alliance_unauthorized_without_session() {
    let dir = std::env::temp_dir().join("test_tri_alliance_unauth");
    std::fs::create_dir_all(&dir).ok();
    let data_dir = dir.to_str().unwrap().to_string();

    let secret_key = Key::generate();
    let app_state = make_alliance_app_state(data_dir.clone());
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key,
            ))
            .service(
                web::resource(
                    "/{account_name}/{server}/api/alliances/{alliance_slug}/tri-alliance",
                )
                .route(web::get().to(tri_alliance::get_tri_alliance)),
            ),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/devtest/1/api/alliances/slaughterhouse/tri-alliance")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
    std::fs::remove_dir_all(&dir).ok();
}
