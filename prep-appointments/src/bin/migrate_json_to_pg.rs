use postgres::{Client, NoTls};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use prep_appointments::web::{Account, FormData, ScheduleData, StatsResponse};

#[derive(Default)]
struct Counts {
    accounts: usize,
    current_forms_map: usize,
    forms: usize,
    schedules: usize,
    statistics: usize,
    feedback: usize,
    submissions: usize,
    domain_docs: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = env::var("MIGRATE_DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let database_url =
        env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required for migration script")?;
    let dry_run = env::var("DRY_RUN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut client = Client::connect(&database_url, NoTls)?;
    let mut counts = Counts::default();

    if dry_run {
        println!("Running in dry-run mode (no writes will be committed).");
    }

    let mut tx = client.transaction()?;
    migrate_accounts(&data_dir, &mut tx, &mut counts)?;
    migrate_current_forms_map(&data_dir, &mut tx, &mut counts)?;
    migrate_forms(&data_dir, &mut tx, &mut counts)?;
    migrate_schedules(&data_dir, &mut tx, &mut counts)?;
    migrate_statistics(&data_dir, &mut tx, &mut counts)?;
    migrate_feedback(&data_dir, &mut tx, &mut counts)?;
    migrate_submissions_csv(&data_dir, &mut tx, &mut counts)?;
    migrate_generic_domain_documents(&data_dir, &mut tx, &mut counts)?;

    if dry_run {
        tx.rollback()?;
        println!("Dry run finished. Transaction rolled back.");
    } else {
        tx.commit()?;
        println!("Migration committed.");
    }

    println!("Migration summary:");
    println!("  accounts: {}", counts.accounts);
    println!("  current_forms_map: {}", counts.current_forms_map);
    println!("  forms: {}", counts.forms);
    println!("  schedules: {}", counts.schedules);
    println!("  statistics: {}", counts.statistics);
    println!("  feedback: {}", counts.feedback);
    println!("  form_submissions: {}", counts.submissions);
    println!("  domain_documents: {}", counts.domain_docs);

    Ok(())
}

fn migrate_accounts(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(data_dir).join("accounts.json");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let accounts: HashMap<String, Account> = serde_json::from_str(&content)?;
    for (key, account) in accounts {
        let payload = serde_json::to_value(account)?;
        tx.execute(
            "INSERT INTO accounts (account_key, payload, updated_at) VALUES ($1, $2, NOW())
             ON CONFLICT (account_key) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
            &[&key, &payload],
        )?;
        counts.accounts += 1;
    }
    Ok(())
}

fn migrate_current_forms_map(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(data_dir).join("current_forms_map.json");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let mapping: HashMap<String, String> = serde_json::from_str(&content)?;
    for (key, value) in mapping {
        tx.execute(
            "INSERT INTO current_forms_map (schedule_key, form_code, updated_at) VALUES ($1, $2, NOW())
             ON CONFLICT (schedule_key) DO UPDATE SET form_code = EXCLUDED.form_code, updated_at = NOW()",
            &[&key, &value],
        )?;
        counts.current_forms_map += 1;
    }
    Ok(())
}

fn migrate_forms(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_forms_dir = Path::new(data_dir).join("current_forms");
    if current_forms_dir.exists() {
        for entry in fs::read_dir(&current_forms_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path)?;
            let form: FormData = serde_json::from_str(&content)?;
            let payload = serde_json::to_value(&form)?;
            tx.execute(
                "INSERT INTO forms (code, account_name, server_number, archived, archive_name, payload, updated_at)
                 VALUES ($1, $2, $3, FALSE, NULL, $4, NOW())
                 ON CONFLICT (code) DO UPDATE SET
                   account_name = EXCLUDED.account_name,
                   server_number = EXCLUDED.server_number,
                   archived = FALSE,
                   archive_name = NULL,
                   payload = EXCLUDED.payload,
                   updated_at = NOW()",
                &[&form.code, &form.account_name, &(form.server_number as i32), &payload],
            )?;
            counts.forms += 1;
        }
    }

    let old_forms_dir = Path::new(data_dir).join("old_forms");
    if old_forms_dir.exists() {
        for entry in fs::read_dir(&old_forms_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let archive_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let content = fs::read_to_string(&path)?;
            let form: FormData = serde_json::from_str(&content)?;
            let payload = serde_json::to_value(&form)?;
            tx.execute(
                "INSERT INTO forms (code, account_name, server_number, archived, archive_name, payload, updated_at)
                 VALUES ($1, $2, $3, TRUE, $4, $5, NOW())
                 ON CONFLICT (code) DO UPDATE SET
                   account_name = EXCLUDED.account_name,
                   server_number = EXCLUDED.server_number,
                   archived = TRUE,
                   archive_name = EXCLUDED.archive_name,
                   payload = EXCLUDED.payload,
                   updated_at = NOW()",
                &[
                    &form.code,
                    &form.account_name,
                    &(form.server_number as i32),
                    &archive_name,
                    &payload,
                ],
            )?;
            counts.forms += 1;
        }
    }
    Ok(())
}

fn migrate_schedules(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(data_dir).join("schedules");
    migrate_account_server_json_tree(dir, tx, "schedules", counts)
}

fn migrate_statistics(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = Path::new(data_dir).join("statistics");
    migrate_account_server_json_tree(dir, tx, "statistics", counts)
}

fn migrate_account_server_json_tree(
    root: PathBuf,
    tx: &mut postgres::Transaction<'_>,
    table: &str,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    if !root.exists() {
        return Ok(());
    }
    for account in fs::read_dir(root)? {
        let account_entry = account?;
        if !account_entry.file_type()?.is_dir() {
            continue;
        }
        let account_name = account_entry
            .file_name()
            .to_str()
            .unwrap_or_default()
            .to_string();
        for file in fs::read_dir(account_entry.path())? {
            let file_path = file?.path();
            if file_path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let server_number = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            if server_number == 0 {
                continue;
            }
            let content = fs::read_to_string(&file_path)?;
            let payload: Value = if table == "schedules" {
                let parsed: ScheduleData = serde_json::from_str(&content)?;
                serde_json::to_value(parsed)?
            } else {
                let parsed: StatsResponse = serde_json::from_str(&content)?;
                serde_json::to_value(parsed)?
            };
            let sql = format!(
                "INSERT INTO {} (account_name, server_number, payload, updated_at) VALUES ($1, $2, $3, NOW())
                 ON CONFLICT (account_name, server_number) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
                table
            );
            tx.execute(&sql, &[&account_name, &server_number, &payload])?;
            if table == "schedules" {
                counts.schedules += 1;
            } else {
                counts.statistics += 1;
            }
        }
    }
    Ok(())
}

fn migrate_feedback(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(data_dir).join("feedback.json");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)?;
    let entries: Vec<Value> = serde_json::from_str(&content)?;
    for entry in entries {
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let created_at = entry
            .get("created_at")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        tx.execute(
            "INSERT INTO feedback (id, created_at, payload) VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET created_at = EXCLUDED.created_at, payload = EXCLUDED.payload",
            &[&id, &created_at, &entry],
        )?;
        counts.feedback += 1;
    }
    Ok(())
}

fn migrate_submissions_csv(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut csv_paths = Vec::new();
    gather_csv_files(Path::new(data_dir), &mut csv_paths)?;
    for csv_path in csv_paths {
        let form_code = csv_path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_suffix("_submissions"))
            .unwrap_or_default()
            .to_string();
        if form_code.is_empty() {
            continue;
        }
        let mut reader = csv::Reader::from_path(&csv_path)?;
        let headers = reader
            .headers()?
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<_>>();
        for record in reader.records() {
            let record = record?;
            let mut row = serde_json::Map::new();
            for (idx, value) in record.iter().enumerate() {
                let key = headers
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| format!("col_{}", idx));
                row.insert(key, Value::String(value.to_string()));
            }
            let payload = Value::Object(row);
            let player_id = payload
                .get("player_id")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            tx.execute(
                "INSERT INTO form_submissions (form_code, player_id, row_data) VALUES ($1, $2, $3)",
                &[&form_code, &player_id, &payload],
            )?;
            counts.submissions += 1;
        }
    }
    Ok(())
}

fn migrate_generic_domain_documents(
    data_dir: &str,
    tx: &mut postgres::Transaction<'_>,
    counts: &mut Counts,
) -> Result<(), Box<dyn std::error::Error>> {
    let known_roots = [
        "accounts.json",
        "current_forms_map.json",
        "feedback.json",
        "current_forms",
        "old_forms",
        "schedules",
        "statistics",
    ];
    let root = Path::new(data_dir);
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if known_roots.iter().any(|k| *k == name) {
            continue;
        }
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            let payload: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
            tx.execute(
                "INSERT INTO domain_documents (domain, doc_key, payload, updated_at) VALUES ($1, $2, $3, NOW())
                 ON CONFLICT (domain, doc_key) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
                &[&"misc", &name, &payload],
            )?;
            counts.domain_docs += 1;
        } else if path.is_dir() {
            let domain = name;
            let mut json_files = Vec::new();
            gather_json_files(&path, &mut json_files)?;
            for file in json_files {
                let rel = file
                    .strip_prefix(root)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .to_string();
                let payload: Value = serde_json::from_str(&fs::read_to_string(&file)?)?;
                tx.execute(
                    "INSERT INTO domain_documents (domain, doc_key, payload, updated_at) VALUES ($1, $2, $3, NOW())
                     ON CONFLICT (domain, doc_key) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
                    &[&domain, &rel, &payload],
                )?;
                counts.domain_docs += 1;
            }
        }
    }
    Ok(())
}

fn gather_csv_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            gather_csv_files(&p, out)?;
        } else if p.extension().and_then(|e| e.to_str()) == Some("csv") {
            out.push(p);
        }
    }
    Ok(())
}

fn gather_json_files(
    path: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            gather_json_files(&p, out)?;
        } else if p.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(p);
        }
    }
    Ok(())
}
