//! Persistence layer: load/save for accounts, forms, schedules, and statistics.

use rand::Rng;
use std::collections::HashMap;
use std::path::Path;

use super::state::{Account, FormData, ScheduleData, StatsResponse};

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

/// Load accounts from file
pub fn load_accounts(data_dir: &str) -> HashMap<String, Account> {
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

/// Save accounts to file
pub fn save_accounts(data_dir: &str, accounts: &HashMap<String, Account>) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let accounts_path = format!("{}/accounts.json", data_dir);
    let content = serde_json::to_string_pretty(accounts)?;
    std::fs::write(&accounts_path, content)?;
    Ok(())
}

/// Load current forms mapping
pub fn load_current_forms(data_dir: &str) -> HashMap<String, String> {
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

/// Save current forms mapping
pub fn save_current_forms(
    data_dir: &str,
    current_forms: &HashMap<String, String>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let path = format!("{}/current_forms_map.json", data_dir);
    let content = serde_json::to_string_pretty(current_forms)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Save schedule to disk
pub fn save_schedule(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
    schedule_data: &ScheduleData,
) -> std::io::Result<()> {
    let schedules_dir = format!("{}/schedules/{}", data_dir, account_name);
    std::fs::create_dir_all(&schedules_dir)?;
    let path = format!("{}/{}.json", schedules_dir, server_number);
    let content = serde_json::to_string_pretty(schedule_data)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Load schedule from disk
pub fn load_schedule(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> Option<ScheduleData> {
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

/// Save statistics to disk
pub fn save_statistics(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
    stats: &StatsResponse,
) -> std::io::Result<()> {
    let stats_dir = format!("{}/statistics/{}", data_dir, account_name);
    std::fs::create_dir_all(&stats_dir)?;
    let path = format!("{}/{}.json", stats_dir, server_number);
    let content = serde_json::to_string_pretty(stats)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Load statistics from disk
pub fn load_statistics(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> Option<StatsResponse> {
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

/// Load all forms from current_forms folder
pub fn load_forms(data_dir: &str) -> HashMap<String, FormData> {
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

    if let Ok(entries) = std::fs::read_dir(&current_forms_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(form_data) = serde_json::from_str::<FormData>(&content) {
                            forms.insert(form_data.code.clone(), form_data);
                        }
                    }
                }
            }
        }
    }

    forms
}

/// Save a single form to current_forms folder
pub fn save_form(data_dir: &str, form_data: &FormData) -> std::io::Result<()> {
    let current_forms_dir = format!("{}/current_forms", data_dir);
    std::fs::create_dir_all(&current_forms_dir)?;
    let form_path = format!("{}/{}.json", current_forms_dir, form_data.code);
    let content = serde_json::to_string_pretty(form_data)?;
    std::fs::write(&form_path, content)?;
    Ok(())
}

/// Archive old forms to old_forms folder (including CSV files)
pub fn archive_old_forms(
    data_dir: &str,
    account_name: &str,
    server_number: u32,
) -> std::io::Result<()> {
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
                                    "{}/{}_{}_{}.json",
                                    old_forms_dir, account_name, server_number, timestamp
                                );
                                std::fs::copy(entry.path(), &old_form_json_path)?;
                                std::fs::remove_file(entry.path())?;

                                let csv_file_name = format!("{}_submissions.csv", code);
                                let csv_path = format!("{}/{}", current_forms_dir, csv_file_name);
                                if Path::new(&csv_path).exists() {
                                    let old_csv_path = format!(
                                        "{}/{}_{}_{}_submissions.csv",
                                        old_forms_dir, account_name, server_number, timestamp
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
