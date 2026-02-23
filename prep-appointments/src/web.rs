mod auth;
pub mod forms;
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

pub async fn start_server(port: u16, _admin_password: String) -> std::io::Result<()> {
    let data_dir = "data".to_string();
    std::fs::create_dir_all(&data_dir)?;

    let accounts = persistence::load_accounts(&data_dir);
    let forms = persistence::load_forms(&data_dir);
    let current_forms = persistence::load_current_forms(&data_dir);

    let app_state = web::Data::new(state::AppState {
        accounts: Mutex::new(accounts),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(forms),
        current_forms: Mutex::new(current_forms),
        data_dir,
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
            .route("/", web::get().to(pages::index))
            .route("/info", web::get().to(pages::info_page))
            .route("/create-account", web::get().to(pages::create_account_page))
            .route("/api/create-account", web::post().to(auth::create_account))
            .route("/api/login", web::post().to(auth::login_api))
            .route("/api/logout", web::post().to(auth::logout_api))
            .route("/api/session", web::get().to(auth::get_session_info))
            .route(
                "/api/generate-schedule",
                web::post().to(schedule::generate_schedule_api),
            )
            .route("/servers", web::get().to(pages::servers_list_page))
            .route("/api/servers", web::get().to(auth::list_servers))
            .route(
                "/dashboard/{account_name}",
                web::get().to(pages::dashboard_page),
            )
            .service(
                web::resource("/view/{account_name}/{server}")
                    .route(web::get().to(pages::view_schedule_page)),
            )
            .service(web::resource("/form/{code}").route(web::get().to(pages::public_form_page)))
            .service(
                web::resource("/form/{code}/stats")
                    .route(web::get().to(pages::public_form_stats_page)),
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
                web::resource("/{account_name}/{server}")
                    .route(web::get().to(pages::schedules_page)),
            )
            .service(
                web::resource("/{account_name}/{server}/stats")
                    .route(web::get().to(pages::stats_page)),
            )
            .service(
                web::resource("/{account_name}/{server}/admin")
                    .route(web::get().to(pages::admin_page)),
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
                web::resource("/{account_name}/{server}/api/form/submissions")
                    .route(web::get().to(forms::get_form_submissions)),
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
