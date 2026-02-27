mod admin;
mod auth;
mod avatar;
mod feedback;
pub mod forms;
mod oauth;
pub mod oauth_state;
mod pages;
mod persistence;
mod schedule;
mod state;

use actix_files::Files;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, middleware, web, App, HttpServer};
use std::collections::HashMap;
use std::sync::Mutex;

// Re-export for external use
pub use persistence::*;
pub use state::*;

pub async fn start_server(port: u16) -> std::io::Result<()> {
    let data_dir = "data".to_string();
    std::fs::create_dir_all(&data_dir)?;

    let accounts = persistence::load_accounts(&data_dir);
    let forms = persistence::load_forms(&data_dir);
    let mut current_forms = persistence::load_current_forms(&data_dir);
    current_forms.retain(|_, code| forms.contains_key(code));
    persistence::save_current_forms(&data_dir, &current_forms).ok();

    let app_state = web::Data::new(state::AppState {
        accounts: Mutex::new(accounts),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms),
        current_forms: Mutex::new(current_forms),
        data_dir,
        oauth_state_cache: oauth_state::OAuthStateCache::new(),
        pending_oauth_cache: oauth_state::PendingOAuthCache::new(),
    });

    let secret_key = Key::generate();

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(middleware::Logger::default())
            .service(Files::new("/static", "static").show_files_listing())
            .service(Files::new("/assets", "static/dist/assets"))
            .route("/", web::get().to(pages::spa_index))
            .route("/api/create-account", web::post().to(auth::create_account))
            .route("/api/login", web::post().to(auth::login_api))
            .route("/api/logout", web::post().to(auth::logout_api))
            .route("/api/session", web::get().to(auth::get_session_info))
            .route("/api/profile/update", web::put().to(auth::update_profile))
            .route(
                "/api/profile/kingshot-lookup",
                web::post().to(auth::kingshot_lookup_profile),
            )
            .route("/api/auth/callback", web::get().to(oauth::oauth_callback))
            .service(
                web::resource("/api/auth/{provider}").route(web::get().to(oauth::oauth_initiate)),
            )
            .service(
                web::resource("/api/avatar/{player_id}").route(web::get().to(avatar::get_avatar)),
            )
            .route(
                "/api/generate-schedule",
                web::post().to(schedule::generate_schedule_api),
            )
            .route("/api/servers", web::get().to(auth::list_servers))
            .route("/api/admin/accounts", web::get().to(admin::list_accounts))
            .route(
                "/api/admin/accounts/{account_name}/admin",
                web::post().to(admin::set_admin),
            )
            .route("/api/feedback", web::post().to(feedback::submit_feedback))
            .route(
                "/api/admin/feedback",
                web::get().to(feedback::list_feedback),
            )
            .service(
                web::resource("/api/admin/feedback/{id}/archive")
                    .route(web::post().to(feedback::archive_feedback)),
            )
            .service(
                web::resource("/form/{code}/api/config")
                    .route(web::get().to(forms::get_form_config_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/check-submission/{player_id}")
                    .route(web::get().to(forms::check_submission_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/player-lookup/{player_id}")
                    .route(web::get().to(forms::player_lookup_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/stats")
                    .route(web::get().to(forms::get_form_stats_by_code)),
            )
            .service(
                web::resource("/form/{code}/api/submit")
                    .route(web::post().to(forms::submit_form_by_code)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/create")
                    .route(web::post().to(forms::create_form)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/config")
                    .route(web::put().to(forms::update_form_config)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/current")
                    .route(web::get().to(forms::get_current_form_info)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/old")
                    .route(web::get().to(forms::list_old_forms_api)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/reopen")
                    .route(web::post().to(forms::reopen_form_api)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/previous")
                    .route(web::get().to(forms::get_previous_form_config)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/download-csv")
                    .route(web::get().to(forms::download_form_csv)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/player/{player_id}")
                    .route(web::get().to(forms::get_player_by_id)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/login")
                    .route(web::post().to(auth::account_login)),
            )
            .service(web::resource("/{account_name}/{server}/api/upload").to(auth::account_upload))
            .service(
                web::resource("/{account_name}/{server}/api/stats")
                    .route(web::get().to(schedule::get_stats)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/schedule/{day}")
                    .route(web::get().to(schedule::get_schedule)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/schedule/{day}/slot")
                    .route(web::put().to(schedule::update_schedule_slot)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/schedule/clear")
                    .route(web::post().to(schedule::clear_schedule_api)),
            )
            .service(
                web::resource("/{account_name}/{server}/api/form/submissions")
                    .route(web::get().to(forms::get_form_submissions)),
            )
            .default_service(web::to(pages::spa_index))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
