//! Persistence layer: load/save for accounts, forms, schedules, and statistics.
//!
//! The Postgres path uses an async `deadpool-postgres` pool initialised once
//! at startup. The JSON path keeps the existing on-disk file layout and runs
//! inline (file reads are microseconds and the dev/test backend never needs
//! pooling). All public helpers are `async` so handlers can `.await` them
//! uniformly regardless of backend.

use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use tokio_postgres::types::ToSql;

use super::db::PgPool;
use super::state::{Account, FormData, ScheduleData, StatsResponse};
use crate::form::submission::FormSubmission;

#[derive(Clone, Debug)]
enum StorageBackend {
    Json,
    Postgres,
}

static STORAGE_BACKEND: OnceLock<StorageBackend> = OnceLock::new();
static PG_POOL: OnceLock<PgPool> = OnceLock::new();

fn storage_backend() -> &'static StorageBackend {
    STORAGE_BACKEND.get_or_init(|| {
        let backend = std::env::var("STORAGE_BACKEND").unwrap_or_else(|_| "json".to_string());
        if backend.eq_ignore_ascii_case("postgres") {
            let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
            if database_url.is_empty() {
                eprintln!(
                    "STORAGE_BACKEND=postgres but DATABASE_URL is empty, falling back to json"
                );
                StorageBackend::Json
            } else {
                StorageBackend::Postgres
            }
        } else {
            StorageBackend::Json
        }
    })
}

/// True when the storage backend is configured to use Postgres.
pub fn is_postgres_backend() -> bool {
    matches!(storage_backend(), StorageBackend::Postgres)
}

/// Install the global pool. Must be called once during startup before any
/// persistence call when the backend is Postgres.
pub fn init_pg_pool(pool: PgPool) {
    let _ = PG_POOL.set(pool);
}

/// Returns the global pool, if installed.
pub fn pg_pool() -> Option<&'static PgPool> {
    PG_POOL.get()
}

fn require_pool() -> Result<&'static PgPool, Box<dyn std::error::Error + Send + Sync>> {
    pg_pool().ok_or_else(|| "Postgres pool not initialised".into())
}

fn io_other<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

/// Returns the schedule key for an account/server
pub fn schedule_key(account_name: &str, server_number: u32) -> String {
    format!("{}:{}", account_name, server_number)
}

/// Gets the current form for an account/server
pub fn get_current_form(
    forms: &HashMap<String, FormData>,
    current_forms: &HashMap<String, String>,
    account_name: &str,
    server_number: u32,
) -> Option<FormData> {
    let account_name_lower = account_name.to_lowercase();
    let key = schedule_key(&account_name_lower, server_number);
    if let Some(form_code) = current_forms.get(&key) {
        forms.get(form_code).cloned()
    } else {
        forms
            .values()
            .filter(|f| {
                f.account_name.to_lowercase() == account_name_lower
                    && f.server_number == server_number
            })
            .max_by_key(|f| &f.created_at)
            .cloned()
    }
}

/// Generate a unique 12-character alphanumeric form code
pub fn generate_form_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..12)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ============ accounts ============

pub async fn load_accounts(data_dir: &str) -> HashMap<String, Account> {
    if is_postgres_backend() {
        return pg_load_accounts().await.unwrap_or_else(|e| {
            eprintln!("Failed to load accounts from postgres: {e}");
            HashMap::new()
        });
    }
    let accounts_path = format!("{}/accounts.json", data_dir);
    if Path::new(&accounts_path).exists() {
        if let Ok(content) = std::fs::read_to_string(&accounts_path) {
            if let Ok(accounts) = serde_json::from_str::<HashMap<String, Account>>(&content) {
                return accounts;
            }
        }
    }
    HashMap::new()
}

pub async fn save_accounts(
    data_dir: &str,
    accounts: &HashMap<String, Account>,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_accounts(accounts).await.map_err(io_other);
    }
    std::fs::create_dir_all(data_dir)?;
    let accounts_path = format!("{}/accounts.json", data_dir);
    let content = serde_json::to_string_pretty(accounts)?;
    std::fs::write(&accounts_path, content)?;
    Ok(())
}

async fn pg_load_accounts(
) -> Result<HashMap<String, Account>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let mut map = HashMap::new();
    for row in client
        .query("SELECT account_key, payload FROM accounts", &[])
        .await?
    {
        let key: String = row.get(0);
        let payload: Value = row.get(1);
        let account: Account = serde_json::from_value(payload)?;
        map.insert(key, account);
    }
    Ok(map)
}

async fn pg_save_accounts(
    accounts: &HashMap<String, Account>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = require_pool()?.client().await?;
    let tx = client.transaction().await?;
    tx.execute("DELETE FROM accounts", &[]).await?;
    for (k, v) in accounts {
        let payload = serde_json::to_value(v)?;
        let params: &[&(dyn ToSql + Sync)] = &[k, &payload];
        tx.execute(
            "INSERT INTO accounts (account_key, payload) VALUES ($1, $2)",
            params,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ============ current forms map ============

pub async fn load_current_forms(data_dir: &str) -> HashMap<String, String> {
    if is_postgres_backend() {
        return pg_load_current_forms().await.unwrap_or_else(|e| {
            eprintln!("Failed to load current forms from postgres: {e}");
            HashMap::new()
        });
    }
    let path = format!("{}/current_forms_map.json", data_dir);
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mapping) = serde_json::from_str::<HashMap<String, String>>(&content) {
                return mapping;
            }
        }
    }
    HashMap::new()
}

pub async fn save_current_forms(
    data_dir: &str,
    current_forms: &HashMap<String, String>,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_current_forms(current_forms).await.map_err(io_other);
    }
    std::fs::create_dir_all(data_dir)?;
    let path = format!("{}/current_forms_map.json", data_dir);
    let content = serde_json::to_string_pretty(current_forms)?;
    std::fs::write(&path, content)?;
    Ok(())
}

async fn pg_load_current_forms(
) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let mut map = HashMap::new();
    for row in client
        .query("SELECT schedule_key, form_code FROM current_forms_map", &[])
        .await?
    {
        map.insert(row.get(0), row.get(1));
    }
    Ok(map)
}

async fn pg_save_current_forms(
    current_forms: &HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = require_pool()?.client().await?;
    let tx = client.transaction().await?;
    tx.execute("DELETE FROM current_forms_map", &[]).await?;
    for (k, v) in current_forms {
        let params: &[&(dyn ToSql + Sync)] = &[k, v];
        tx.execute(
            "INSERT INTO current_forms_map (schedule_key, form_code) VALUES ($1, $2)",
            params,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ============ schedules ============

pub async fn save_schedule(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
    schedule_data: &ScheduleData,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_schedule(account_name, server_number, schedule_data)
            .await
            .map_err(io_other);
    }
    let schedules_dir = format!("{}/schedules/{}", data_dir, account_name);
    std::fs::create_dir_all(&schedules_dir)?;
    let path = format!("{}/{}.json", schedules_dir, server_number);
    let content = serde_json::to_string_pretty(schedule_data)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub async fn load_schedule(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> Option<ScheduleData> {
    if is_postgres_backend() {
        return pg_load_schedule(account_name, server_number)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to load schedule from postgres: {e}");
                None
            });
    }
    let path = format!(
        "{}/schedules/{}/{}.json",
        data_dir, account_name, server_number
    );
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<ScheduleData>(&content) {
                Ok(schedule_data) => return Some(schedule_data),
                Err(e) => {
                    eprintln!("Failed to deserialize schedule from {}: {}", path, e);
                    return None;
                }
            }
        } else {
            eprintln!("Failed to read schedule file: {}", path);
        }
    }
    None
}

async fn pg_save_schedule(
    account_name: &str,
    server_number: u32,
    schedule_data: &ScheduleData,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let payload = serde_json::to_value(schedule_data)?;
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&account_name, &server, &payload];
    client
        .execute(
            "INSERT INTO schedules (account_name, server_number, payload, updated_at) VALUES ($1, $2, $3, NOW())
         ON CONFLICT (account_name, server_number) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_load_schedule(
    account_name: &str,
    server_number: u32,
) -> Result<Option<ScheduleData>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&account_name, &server];
    if let Some(row) = client
        .query_opt(
            "SELECT payload FROM schedules WHERE account_name = $1 AND server_number = $2",
            params,
        )
        .await?
    {
        let payload: Value = row.get(0);
        return Ok(Some(serde_json::from_value(payload)?));
    }
    Ok(None)
}

// ============ statistics ============

pub async fn save_statistics(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
    stats: &StatsResponse,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_statistics(account_name, server_number, stats)
            .await
            .map_err(io_other);
    }
    let stats_dir = format!("{}/statistics/{}", data_dir, account_name);
    std::fs::create_dir_all(&stats_dir)?;
    let path = format!("{}/{}.json", stats_dir, server_number);
    let content = serde_json::to_string_pretty(stats)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub async fn load_statistics(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> Option<StatsResponse> {
    if is_postgres_backend() {
        return pg_load_statistics(account_name, server_number)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to load statistics from postgres: {e}");
                None
            });
    }
    let path = format!(
        "{}/statistics/{}/{}.json",
        data_dir, account_name, server_number
    );
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<StatsResponse>(&content) {
                Ok(stats) => return Some(stats),
                Err(e) => {
                    eprintln!("Failed to deserialize statistics from {}: {}", path, e);
                    return None;
                }
            }
        } else {
            eprintln!("Failed to read statistics file: {}", path);
        }
    }
    None
}

async fn pg_save_statistics(
    account_name: &str,
    server_number: u32,
    stats: &StatsResponse,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let payload = serde_json::to_value(stats)?;
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&account_name, &server, &payload];
    client
        .execute(
            "INSERT INTO statistics (account_name, server_number, payload, updated_at) VALUES ($1, $2, $3, NOW())
         ON CONFLICT (account_name, server_number) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_load_statistics(
    account_name: &str,
    server_number: u32,
) -> Result<Option<StatsResponse>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&account_name, &server];
    if let Some(row) = client
        .query_opt(
            "SELECT payload FROM statistics WHERE account_name = $1 AND server_number = $2",
            params,
        )
        .await?
    {
        let payload: Value = row.get(0);
        return Ok(Some(serde_json::from_value(payload)?));
    }
    Ok(None)
}

// ============ forms ============

fn move_expired_form_to_old(
    _data_dir: &str,
    form_data: &FormData,
    current_forms_dir: &str,
    old_forms_dir: &str,
) -> std::io::Result<bool> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let delete_date = match &form_data.delete_date {
        Some(d) if d.as_str() <= today.as_str() => d,
        _ => return Ok(false),
    };
    let _ = delete_date;

    let code = &form_data.code;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let json_path = format!("{}/{}.json", current_forms_dir, code);
    let old_json_path = format!(
        "{}/{}_{}_{}_{}.json",
        old_forms_dir, form_data.account_name, form_data.server_number, code, timestamp
    );
    if Path::new(&json_path).exists() {
        std::fs::copy(&json_path, &old_json_path)?;
        std::fs::remove_file(&json_path)?;
    }
    let csv_path = format!("{}/{}_submissions.csv", current_forms_dir, code);
    if Path::new(&csv_path).exists() {
        let old_csv_path = format!(
            "{}/{}_{}_{}_{}_submissions.csv",
            old_forms_dir, form_data.account_name, form_data.server_number, code, timestamp
        );
        std::fs::copy(&csv_path, &old_csv_path)?;
        std::fs::remove_file(&csv_path)?;
    }
    Ok(true)
}

pub async fn load_forms(data_dir: &str) -> HashMap<String, FormData> {
    if is_postgres_backend() {
        return pg_load_forms().await.unwrap_or_else(|e| {
            eprintln!("Failed to load forms from postgres: {e}");
            HashMap::new()
        });
    }
    let current_forms_dir = format!("{}/current_forms", data_dir);
    let mut forms = HashMap::new();

    if !Path::new(&current_forms_dir).exists() {
        let old_forms_path = format!("{}/forms.json", data_dir);
        if Path::new(&old_forms_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&old_forms_path) {
                if let Ok(old_forms) = serde_json::from_str::<HashMap<String, FormData>>(&content) {
                    std::fs::create_dir_all(&current_forms_dir).ok();
                    std::fs::create_dir_all(format!("{}/old_forms", data_dir)).ok();

                    for (code, mut form_data) in old_forms {
                        if form_data.name.is_empty() {
                            form_data.name = format!(
                                "Form {} {}",
                                form_data.account_name, form_data.server_number
                            );
                        }
                        if form_data.created_at.is_empty() {
                            form_data.created_at = chrono::Local::now().to_rfc3339();
                        }

                        let account_name = form_data.account_name.clone();
                        let server_number = form_data.server_number;

                        let form_path = format!("{}/{}.json", current_forms_dir, code);
                        if let Ok(content) = serde_json::to_string_pretty(&form_data) {
                            std::fs::write(&form_path, content).ok();
                            forms.insert(code.clone(), form_data.clone());

                            let old_csv_path = format!(
                                "{}/{}_{}_form_submissions.csv",
                                data_dir, account_name, server_number
                            );
                            if Path::new(&old_csv_path).exists() {
                                let new_csv_path =
                                    format!("{}/{}_submissions.csv", current_forms_dir, code);
                                std::fs::copy(&old_csv_path, &new_csv_path).ok();
                            }
                        }
                    }
                    std::fs::remove_file(&old_forms_path).ok();
                }
            }
        }
        return forms;
    }

    let old_forms_dir = format!("{}/old_forms", data_dir);
    std::fs::create_dir_all(&old_forms_dir).ok();

    if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(form_data) = serde_json::from_str::<FormData>(&content) {
                            if move_expired_form_to_old(
                                data_dir,
                                &form_data,
                                &current_forms_dir,
                                &old_forms_dir,
                            )
                            .unwrap_or(false)
                            {
                                continue;
                            }
                            forms.insert(form_data.code.clone(), form_data);
                        }
                    }
                }
            }
        }
    }

    forms
}

pub async fn save_form(data_dir: &str, form_data: &FormData) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_form(form_data).await.map_err(io_other);
    }
    let current_forms_dir = format!("{}/current_forms", data_dir);
    std::fs::create_dir_all(&current_forms_dir)?;
    let form_path = format!("{}/{}.json", current_forms_dir, form_data.code);
    let content = serde_json::to_string_pretty(form_data)?;
    std::fs::write(&form_path, content)?;
    Ok(())
}

pub async fn archive_old_forms(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_archive_old_forms(account_name, server_number)
            .await
            .map_err(io_other);
    }
    let current_forms_dir = format!("{}/current_forms", data_dir);
    let old_forms_dir = format!("{}/old_forms", data_dir);
    std::fs::create_dir_all(&old_forms_dir)?;

    if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();

        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(form_data) = serde_json::from_str::<FormData>(&content) {
                            if form_data.account_name == account_name
                                && form_data.server_number == server_number
                            {
                                let code = &form_data.code;

                                let old_form_json_path = format!(
                                    "{}/{}_{}_{}_{}.json",
                                    old_forms_dir, account_name, server_number, code, timestamp
                                );
                                std::fs::copy(entry.path(), &old_form_json_path)?;
                                std::fs::remove_file(entry.path())?;

                                let csv_file_name = format!("{}_submissions.csv", code);
                                let csv_path = format!("{}/{}", current_forms_dir, csv_file_name);
                                if Path::new(&csv_path).exists() {
                                    let old_csv_path = format!(
                                        "{}/{}_{}_{}_{}_submissions.csv",
                                        old_forms_dir, account_name, server_number, code, timestamp
                                    );
                                    std::fs::copy(&csv_path, &old_csv_path)?;
                                    std::fs::remove_file(&csv_path)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn list_old_forms(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> Vec<(String, FormData)> {
    if is_postgres_backend() {
        return pg_list_old_forms(account_name, server_number)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to list old forms from postgres: {e}");
                Vec::new()
            });
    }
    let old_forms_dir = format!("{}/old_forms", data_dir);
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&old_forms_dir) {
        let prefix = format!("{}_{}_", account_name, server_number);
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") && file_name.starts_with(&prefix) {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(form_data) = serde_json::from_str::<FormData>(&content) {
                            if form_data.account_name == account_name
                                && form_data.server_number == server_number
                            {
                                let archive_name = file_name
                                    .strip_suffix(".json")
                                    .unwrap_or(file_name)
                                    .to_string();
                                result.push((archive_name, form_data));
                            }
                        }
                    }
                }
            }
        }
    }
    result.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
    result
}

async fn pg_load_forms(
) -> Result<HashMap<String, FormData>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let mut map = HashMap::new();
    for row in client
        .query(
            "SELECT code, payload FROM forms WHERE archived = FALSE",
            &[],
        )
        .await?
    {
        let code: String = row.get(0);
        let payload: Value = row.get(1);
        let form: FormData = serde_json::from_value(payload)?;
        map.insert(code, form);
    }
    Ok(map)
}

async fn pg_save_form(
    form_data: &FormData,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let payload = serde_json::to_value(form_data)?;
    let server = form_data.server_number as i32;
    let params: &[&(dyn ToSql + Sync)] =
        &[&form_data.code, &form_data.account_name, &server, &payload];
    client
        .execute(
            "INSERT INTO forms (code, account_name, server_number, archived, archive_name, payload, updated_at)
         VALUES ($1, $2, $3, FALSE, NULL, $4, NOW())
         ON CONFLICT (code) DO UPDATE SET
           account_name = EXCLUDED.account_name,
           server_number = EXCLUDED.server_number,
           archived = FALSE,
           archive_name = NULL,
           payload = EXCLUDED.payload,
           updated_at = NOW()",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_archive_old_forms(
    account_name: &str,
    server_number: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&account_name, &server, &timestamp];
    client
        .execute(
            "UPDATE forms
         SET archived = TRUE, archive_name = CONCAT(code, '_', $3), updated_at = NOW()
         WHERE account_name = $1 AND server_number = $2 AND archived = FALSE",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_list_old_forms(
    account_name: &str,
    server_number: u32,
) -> Result<Vec<(String, FormData)>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let mut out = Vec::new();
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&account_name, &server];
    for row in client
        .query(
            "SELECT archive_name, payload FROM forms
         WHERE account_name = $1 AND server_number = $2 AND archived = TRUE
         ORDER BY updated_at DESC",
            params,
        )
        .await?
    {
        let archive_name: Option<String> = row.get(0);
        let payload: Value = row.get(1);
        let form: FormData = serde_json::from_value(payload)?;
        out.push((archive_name.unwrap_or_else(|| form.code.clone()), form));
    }
    Ok(out)
}

// ============ feedback ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackEntry {
    pub id: String,
    pub r#type: String,
    pub text: String,
    pub created_at: String,
    #[serde(default)]
    pub archived: bool,
}

pub async fn load_feedback(data_dir: &str) -> Vec<FeedbackEntry> {
    if is_postgres_backend() {
        return pg_load_feedback().await.unwrap_or_else(|e| {
            eprintln!("Failed to load feedback from postgres: {e}");
            Vec::new()
        });
    }
    let path = format!("{}/feedback.json", data_dir);
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(entries) = serde_json::from_str::<Vec<FeedbackEntry>>(&content) {
                return entries;
            }
        }
    }
    Vec::new()
}

pub async fn save_feedback(data_dir: &str, feedback: &[FeedbackEntry]) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_feedback(feedback).await.map_err(io_other);
    }
    std::fs::create_dir_all(data_dir)?;
    let path = format!("{}/feedback.json", data_dir);
    let content = serde_json::to_string_pretty(feedback)?;
    std::fs::write(&path, content)?;
    Ok(())
}

async fn pg_load_feedback() -> Result<Vec<FeedbackEntry>, Box<dyn std::error::Error + Send + Sync>>
{
    let client = require_pool()?.client().await?;
    let mut entries = Vec::new();
    for row in client
        .query("SELECT payload FROM feedback ORDER BY created_at DESC", &[])
        .await?
    {
        let payload: Value = row.get(0);
        let entry: FeedbackEntry = serde_json::from_value(payload)?;
        entries.push(entry);
    }
    Ok(entries)
}

async fn pg_save_feedback(
    feedback: &[FeedbackEntry],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = require_pool()?.client().await?;
    let tx = client.transaction().await?;
    tx.execute("DELETE FROM feedback", &[]).await?;
    for entry in feedback {
        let payload = serde_json::to_value(entry)?;
        let params: &[&(dyn ToSql + Sync)] = &[&entry.id, &entry.created_at, &payload];
        tx.execute(
            "INSERT INTO feedback (id, created_at, payload) VALUES ($1, $2, $3)",
            params,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn reopen_old_form(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
    archive_name: &str,
) -> std::io::Result<FormData> {
    if is_postgres_backend() {
        return pg_reopen_old_form(account_name, server_number, archive_name)
            .await
            .map_err(io_other);
    }
    let old_forms_dir = format!("{}/old_forms", data_dir);
    let current_forms_dir = format!("{}/current_forms", data_dir);
    std::fs::create_dir_all(&current_forms_dir)?;

    let json_path = format!("{}/{}.json", old_forms_dir, archive_name);
    let content = std::fs::read_to_string(&json_path)?;
    let form_data: FormData = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if form_data.account_name != account_name || form_data.server_number != server_number {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Form does not belong to this account/server",
        ));
    }

    let code = &form_data.code;
    let dest_json = format!("{}/{}.json", current_forms_dir, code);
    std::fs::copy(&json_path, &dest_json)?;

    let csv_src = format!("{}_submissions.csv", archive_name);
    let csv_path = format!("{}/{}", old_forms_dir, csv_src);
    if Path::new(&csv_path).exists() {
        let dest_csv = format!("{}/{}_submissions.csv", current_forms_dir, code);
        std::fs::copy(&csv_path, &dest_csv)?;
    }

    Ok(form_data)
}

async fn pg_reopen_old_form(
    account_name: &str,
    server_number: u32,
    archive_name: &str,
) -> Result<FormData, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let server = server_number as i32;
    let params: &[&(dyn ToSql + Sync)] = &[&archive_name, &account_name, &server];
    let row = client
        .query_opt(
            "SELECT payload FROM forms WHERE archive_name = $1 AND account_name = $2 AND server_number = $3",
            params,
        )
        .await?;
    let Some(row) = row else {
        return Err(format!("Archived form not found: {archive_name}").into());
    };
    let payload: Value = row.get(0);
    let form_data: FormData = serde_json::from_value(payload)?;
    let upd_params: &[&(dyn ToSql + Sync)] = &[&form_data.code];
    client
        .execute(
            "UPDATE forms SET archived = FALSE, archive_name = NULL, updated_at = NOW() WHERE code = $1",
            upd_params,
        )
        .await?;
    Ok(form_data)
}

// ============ domain documents ============

pub async fn load_domain_doc<T: serde::de::DeserializeOwned + Send>(
    data_dir: &str,
    domain: &str,
    doc_key: &str,
) -> Option<T> {
    if is_postgres_backend() {
        return pg_load_domain_doc(domain, doc_key)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to load domain doc from postgres ({domain}/{doc_key}): {e}");
                None
            });
    }
    load_domain_doc_json(data_dir, domain, doc_key)
}

pub async fn save_domain_doc<T: serde::Serialize + Send + Sync>(
    data_dir: &str,
    domain: &str,
    doc_key: &str,
    value: &T,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_domain_doc(domain, doc_key, value)
            .await
            .map_err(io_other);
    }
    save_domain_doc_json(data_dir, domain, doc_key, value)
}

pub async fn delete_domain_doc(data_dir: &str, domain: &str, doc_key: &str) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_delete_domain_doc(domain, doc_key)
            .await
            .map_err(io_other);
    }
    delete_domain_doc_json(data_dir, domain, doc_key)
}

pub async fn list_domain_docs<T: serde::de::DeserializeOwned + Send>(
    data_dir: &str,
    domain: &str,
    key_prefix: Option<&str>,
) -> Vec<(String, T)> {
    if is_postgres_backend() {
        return pg_list_domain_docs(domain, key_prefix)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to list domain docs from postgres ({domain}): {e}");
                Vec::new()
            });
    }
    list_domain_docs_json(data_dir, domain, key_prefix)
}

pub async fn save_form_submission(
    data_dir: &str,
    form_code: &str,
    submission: &FormSubmission,
) -> std::io::Result<()> {
    if is_postgres_backend() {
        return pg_save_form_submission(form_code, submission)
            .await
            .map_err(io_other);
    }
    let mut submissions: Vec<FormSubmission> =
        load_domain_doc(data_dir, "form_submissions", form_code)
            .await
            .unwrap_or_default();
    submissions.push(submission.clone());
    save_domain_doc(data_dir, "form_submissions", form_code, &submissions).await
}

pub async fn load_form_submissions(data_dir: &str, form_code: &str) -> Vec<FormSubmission> {
    if is_postgres_backend() {
        return pg_load_form_submissions(form_code)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Failed to load form submissions from postgres ({form_code}): {e}");
                Vec::new()
            });
    }
    load_domain_doc(data_dir, "form_submissions", form_code)
        .await
        .unwrap_or_default()
}

pub async fn has_player_submission(data_dir: &str, form_code: &str, player_id: &str) -> bool {
    let player_id = player_id.trim();
    if player_id.is_empty() {
        return false;
    }
    load_form_submissions(data_dir, form_code)
        .await
        .iter()
        .any(|s| s.player_id.trim() == player_id)
}

pub async fn count_form_submissions(data_dir: &str, form_code: &str) -> usize {
    load_form_submissions(data_dir, form_code).await.len()
}

fn domain_store_path(data_dir: &str, domain: &str) -> String {
    format!("{}/domain_documents_{}.json", data_dir, domain)
}

fn load_domain_map_json(data_dir: &str, domain: &str) -> HashMap<String, Value> {
    let path = domain_store_path(data_dir, domain);
    if Path::new(&path).exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, Value>>(&content) {
                return map;
            }
        }
    }
    HashMap::new()
}

fn save_domain_map_json(
    data_dir: &str,
    domain: &str,
    map: &HashMap<String, Value>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let path = domain_store_path(data_dir, domain);
    let content = serde_json::to_string_pretty(map)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn load_domain_doc_json<T: serde::de::DeserializeOwned>(
    data_dir: &str,
    domain: &str,
    doc_key: &str,
) -> Option<T> {
    let map = load_domain_map_json(data_dir, domain);
    map.get(doc_key)
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
}

fn save_domain_doc_json<T: serde::Serialize>(
    data_dir: &str,
    domain: &str,
    doc_key: &str,
    value: &T,
) -> std::io::Result<()> {
    let mut map = load_domain_map_json(data_dir, domain);
    let payload = serde_json::to_value(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    map.insert(doc_key.to_string(), payload);
    save_domain_map_json(data_dir, domain, &map)
}

fn delete_domain_doc_json(data_dir: &str, domain: &str, doc_key: &str) -> std::io::Result<()> {
    let mut map = load_domain_map_json(data_dir, domain);
    map.remove(doc_key);
    save_domain_map_json(data_dir, domain, &map)
}

fn list_domain_docs_json<T: serde::de::DeserializeOwned>(
    data_dir: &str,
    domain: &str,
    key_prefix: Option<&str>,
) -> Vec<(String, T)> {
    let map = load_domain_map_json(data_dir, domain);
    map.into_iter()
        .filter(|(k, _)| key_prefix.map(|p| k.starts_with(p)).unwrap_or(true))
        .filter_map(|(k, v)| serde_json::from_value(v).ok().map(|decoded| (k, decoded)))
        .collect()
}

async fn pg_load_domain_doc<T: serde::de::DeserializeOwned + Send>(
    domain: &str,
    doc_key: &str,
) -> Result<Option<T>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let params: &[&(dyn ToSql + Sync)] = &[&domain, &doc_key];
    if let Some(row) = client
        .query_opt(
            "SELECT payload FROM domain_documents WHERE domain = $1 AND doc_key = $2",
            params,
        )
        .await?
    {
        let payload: Value = row.get(0);
        return Ok(Some(serde_json::from_value::<T>(payload)?));
    }
    Ok(None)
}

async fn pg_save_domain_doc<T: serde::Serialize + Send + Sync>(
    domain: &str,
    doc_key: &str,
    value: &T,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let payload = serde_json::to_value(value)?;
    let params: &[&(dyn ToSql + Sync)] = &[&domain, &doc_key, &payload];
    client
        .execute(
            "INSERT INTO domain_documents (domain, doc_key, payload, updated_at) VALUES ($1, $2, $3, NOW())
         ON CONFLICT (domain, doc_key) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_delete_domain_doc(
    domain: &str,
    doc_key: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let params: &[&(dyn ToSql + Sync)] = &[&domain, &doc_key];
    client
        .execute(
            "DELETE FROM domain_documents WHERE domain = $1 AND doc_key = $2",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_list_domain_docs<T: serde::de::DeserializeOwned + Send>(
    domain: &str,
    key_prefix: Option<&str>,
) -> Result<Vec<(String, T)>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let mut out = Vec::new();
    let rows = if let Some(prefix) = key_prefix {
        let like_pattern = format!("{prefix}%");
        let params: &[&(dyn ToSql + Sync)] = &[&domain, &like_pattern];
        client
            .query(
                "SELECT doc_key, payload FROM domain_documents WHERE domain = $1 AND doc_key LIKE $2",
                params,
            )
            .await?
    } else {
        let params: &[&(dyn ToSql + Sync)] = &[&domain];
        client
            .query(
                "SELECT doc_key, payload FROM domain_documents WHERE domain = $1",
                params,
            )
            .await?
    };
    for row in rows {
        let key: String = row.get(0);
        let payload: Value = row.get(1);
        let decoded: T = serde_json::from_value(payload)?;
        out.push((key, decoded));
    }
    Ok(out)
}

async fn pg_save_form_submission(
    form_code: &str,
    submission: &FormSubmission,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let payload = serde_json::to_value(submission)?;
    let params: &[&(dyn ToSql + Sync)] = &[&form_code, &submission.player_id, &payload];
    client
        .execute(
            "INSERT INTO form_submissions (form_code, player_id, row_data) VALUES ($1, $2, $3)",
            params,
        )
        .await?;
    Ok(())
}

async fn pg_load_form_submissions(
    form_code: &str,
) -> Result<Vec<FormSubmission>, Box<dyn std::error::Error + Send + Sync>> {
    let client = require_pool()?.client().await?;
    let mut out = Vec::new();
    let params: &[&(dyn ToSql + Sync)] = &[&form_code];
    for row in client
        .query(
            "SELECT row_data FROM form_submissions WHERE form_code = $1 ORDER BY id ASC",
            params,
        )
        .await?
    {
        let payload: Value = row.get(0);
        let submission: FormSubmission = serde_json::from_value(payload)?;
        out.push(submission);
    }
    Ok(out)
}
