//! HTML page handlers.

use actix_session::Session;
use actix_web::{web, HttpResponse, Result};

use super::state::AppState;

/// Home page
pub async fn index() -> Result<HttpResponse> {
    let html = include_str!("../../templates/index.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Information / How to Use page
pub async fn info_page() -> Result<HttpResponse> {
    let html = include_str!("../../templates/info.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Servers list page
pub async fn servers_list_page() -> Result<HttpResponse> {
    let html = include_str!("../../templates/servers_list.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// View-only schedule page (public)
pub async fn view_schedule_page(_path: web::Path<(String, u32)>) -> Result<HttpResponse> {
    let html = include_str!("../../templates/view_schedule.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Create account page
pub async fn create_account_page() -> Result<HttpResponse> {
    let html = include_str!("../../templates/create_account.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Schedules page
pub async fn schedules_page(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    let accounts = state.accounts.lock().unwrap();
    let account_name_lower = account_name.to_lowercase();
    if !accounts.contains_key(&account_name_lower) {
        return Ok(HttpResponse::NotFound().body("Account not found"));
    }
    drop(accounts);
    let html = include_str!("../../templates/schedules.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Stats page
pub async fn stats_page(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    let accounts = state.accounts.lock().unwrap();
    let account_name_lower = account_name.to_lowercase();
    if !accounts.contains_key(&account_name_lower) {
        return Ok(HttpResponse::NotFound().body("Account not found"));
    }
    drop(accounts);
    let html = include_str!("../../templates/stats.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Admin page
pub async fn admin_page(
    path: web::Path<(String, u32)>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let (account_name, _server_number) = path.into_inner();
    let accounts = state.accounts.lock().unwrap();
    let account_name_lower = account_name.to_lowercase();
    if !accounts.contains_key(&account_name_lower) {
        return Ok(HttpResponse::NotFound().body("Account not found"));
    }
    drop(accounts);
    let html = include_str!("../../templates/admin.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Public form page
pub async fn public_form_page(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    let forms = state.forms.lock().unwrap();
    if !forms.contains_key(&code) {
        drop(forms);
        return Ok(HttpResponse::NotFound().body("Form not found"));
    }
    drop(forms);
    let html = include_str!("../../templates/form.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Public form stats page
pub async fn public_form_stats_page(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let code = path.into_inner();
    let forms = state.forms.lock().unwrap();
    if !forms.contains_key(&code) {
        drop(forms);
        return Ok(HttpResponse::NotFound().body("Form not found"));
    }
    drop(forms);
    let html = include_str!("../../templates/form_stats.html");
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Dashboard page
pub async fn dashboard_page(path: web::Path<String>, session: Session) -> Result<HttpResponse> {
    let url_account_name = path.into_inner().to_lowercase();
    let session_account_name: Option<String> = session
        .get("account_name")
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to read session"))?;
    match session_account_name {
        Some(account_name) if account_name == url_account_name => {
            let html = include_str!("../../templates/dashboard.html");
            Ok(HttpResponse::Ok().content_type("text/html").body(html))
        }
        Some(_) => Ok(HttpResponse::Forbidden().content_type("text/html").body(
            "<html><body><h1>Access Denied</h1><p>You can only access your own dashboard.</p><a href='/'>Go Home</a></body></html>",
        )),
        None => Ok(HttpResponse::Unauthorized().content_type("text/html").body(
            "<html><body><h1>Unauthorized</h1><p>Please log in to access the dashboard.</p><a href='/'>Go Home</a></body></html>",
        )),
    }
}
