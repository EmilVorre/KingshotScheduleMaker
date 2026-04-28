//! Tests for web module (state, persistence).

use std::collections::HashMap;

use prep_appointments::schedule::types::ScheduledAppointment;
use prep_appointments::schedule::DaySchedule;
use prep_appointments::web::{
    derive_scheduled_player_ids, generate_form_code, get_current_form, get_scheduled_player_ids,
    load_accounts, load_current_forms, load_schedule, load_statistics, save_accounts,
    save_current_forms, save_schedule, save_statistics, schedule_key, Account, AllianceStats,
    FormConfig, FormData, ScheduleData, StatsResponse,
};

fn make_appointment(player_id: &str, slot: u8) -> ScheduledAppointment {
    ScheduledAppointment {
        player_id: player_id.to_string(),
        name: "Test".to_string(),
        alliance: "Test".to_string(),
        slot,
        priority_score: 0,
    }
}

// ============ State tests ============

#[test]
fn test_derive_scheduled_player_ids_empty() {
    let data = ScheduleData {
        construction_schedule: None,
        research_schedule: None,
        troops_schedule: None,
        entries: None,
        scheduled_player_ids: None,
    };
    let ids = derive_scheduled_player_ids(&data);
    assert!(ids.is_empty());
}

#[test]
fn test_derive_scheduled_player_ids_from_appointments() {
    let mut cons = HashMap::new();
    cons.insert(1u8, make_appointment("p1", 1));
    cons.insert(2u8, make_appointment("p2", 2));
    let mut res = HashMap::new();
    res.insert(1u8, make_appointment("p3", 1));
    let data = ScheduleData {
        construction_schedule: Some(DaySchedule {
            appointments: cons,
            unassigned: vec![],
        }),
        research_schedule: Some(DaySchedule {
            appointments: res,
            unassigned: vec![],
        }),
        troops_schedule: None,
        entries: None,
        scheduled_player_ids: None,
    };
    let ids = derive_scheduled_player_ids(&data);
    assert_eq!(ids.len(), 3);
    assert!(ids.contains("p1"));
    assert!(ids.contains("p2"));
    assert!(ids.contains("p3"));
}

#[test]
fn test_get_scheduled_player_ids_uses_stored() {
    let data = ScheduleData {
        construction_schedule: None,
        research_schedule: None,
        troops_schedule: None,
        entries: None,
        scheduled_player_ids: Some(vec!["stored1".to_string(), "stored2".to_string()]),
    };
    let ids = get_scheduled_player_ids(&data);
    assert_eq!(ids.len(), 2);
    assert!(ids.contains("stored1"));
    assert!(ids.contains("stored2"));
}

// ============ Persistence tests ============

#[test]
fn test_schedule_key() {
    assert_eq!(schedule_key("alice", 1), "alice:1");
    assert_eq!(schedule_key("Bob", 42), "Bob:42");
}

#[test]
fn test_get_current_form_from_mapping() {
    let mut forms = HashMap::new();
    let form1 = FormData {
        code: "ABC123".to_string(),
        account_name: "alice".to_string(),
        server_number: 1,
        name: "Form 1".to_string(),
        created_at: "2025-01-01T00:00:00Z".to_string(),
        delete_date: None,
        config: FormConfig::default(),
    };
    forms.insert("ABC123".to_string(), form1.clone());
    let mut current_forms = HashMap::new();
    current_forms.insert("alice:1".to_string(), "ABC123".to_string());
    let result = get_current_form(&forms, &current_forms, "alice", 1);
    assert!(result.is_some());
    assert_eq!(result.unwrap().code, "ABC123");
}

#[test]
fn test_get_current_form_fallback_to_most_recent() {
    let mut forms = HashMap::new();
    let form1 = FormData {
        code: "OLD".to_string(),
        account_name: "alice".to_string(),
        server_number: 1,
        name: "Old".to_string(),
        created_at: "2025-01-01T00:00:00Z".to_string(),
        delete_date: None,
        config: FormConfig::default(),
    };
    let form2 = FormData {
        code: "NEW".to_string(),
        account_name: "alice".to_string(),
        server_number: 1,
        name: "New".to_string(),
        created_at: "2025-01-02T00:00:00Z".to_string(),
        delete_date: None,
        config: FormConfig::default(),
    };
    forms.insert("OLD".to_string(), form1);
    forms.insert("NEW".to_string(), form2);
    let current_forms = HashMap::new();
    let result = get_current_form(&forms, &current_forms, "alice", 1);
    assert!(result.is_some());
    assert_eq!(result.unwrap().code, "NEW");
}

#[test]
fn test_generate_form_code_format() {
    let code = generate_form_code();
    assert_eq!(code.len(), 12);
    assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
}

// ============ Persistence: accounts ============

#[tokio::test]
async fn test_load_save_accounts_roundtrip() {
    let dir = std::env::temp_dir().join("test_accounts_persist");
    std::fs::create_dir_all(&dir).ok();
    let dir_str = dir.to_str().unwrap();

    let mut accounts = HashMap::new();
    accounts.insert(
        "alice".to_string(),
        Account {
            account_name: "alice".to_string(),
            server_number: 1,
            password: "secret".to_string(),
            in_game_name: "AliceIG".to_string(),
            player_id: None,
            oauth_provider: None,
            oauth_id: None,
            admin: false,
            alliance_access: false,
            alliance_id: None,
            alliance_tag: None,
            alliance_name: None,
            friend_code: None,
        },
    );

    save_accounts(dir_str, &accounts).await.unwrap();
    let loaded = load_accounts(dir_str).await;
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.get("alice").unwrap().account_name, "alice");

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_load_accounts_missing_returns_empty() {
    let loaded = load_accounts("/nonexistent/path/12345").await;
    assert!(loaded.is_empty());
}

// ============ Persistence: current forms ============

#[tokio::test]
async fn test_load_save_current_forms_roundtrip() {
    let dir = std::env::temp_dir().join("test_current_forms");
    std::fs::create_dir_all(&dir).ok();
    let dir_str = dir.to_str().unwrap();

    let mut current_forms = HashMap::new();
    current_forms.insert("alice:1".to_string(), "ABC123".to_string());

    save_current_forms(dir_str, &current_forms).await.unwrap();
    let loaded = load_current_forms(dir_str).await;
    assert_eq!(loaded.get("alice:1"), Some(&"ABC123".to_string()));

    std::fs::remove_dir_all(&dir).ok();
}

// ============ Persistence: schedule ============

#[tokio::test]
async fn test_save_load_schedule_roundtrip() {
    let dir = std::env::temp_dir().join("test_schedule_persist");
    std::fs::create_dir_all(&dir).ok();
    let dir_str = dir.to_str().unwrap();

    let mut cons = HashMap::new();
    cons.insert(1u8, make_appointment("p1", 1));
    let schedule_data = ScheduleData {
        construction_schedule: Some(DaySchedule {
            appointments: cons,
            unassigned: vec![],
        }),
        research_schedule: None,
        troops_schedule: None,
        entries: None,
        scheduled_player_ids: None,
    };

    save_schedule(dir_str, "testacct", 1, &schedule_data)
        .await
        .unwrap();
    let loaded = load_schedule(dir_str, "testacct", 1).await;
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert!(loaded.construction_schedule.is_some());
    assert_eq!(
        loaded
            .construction_schedule
            .unwrap()
            .appointments
            .get(&1)
            .unwrap()
            .player_id,
        "p1"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_load_schedule_missing_returns_none() {
    let loaded = load_schedule("/nonexistent", "x", 99).await;
    assert!(loaded.is_none());
}

// ============ Persistence: statistics ============

#[tokio::test]
async fn test_save_load_statistics_roundtrip() {
    let dir = std::env::temp_dir().join("test_stats_persist");
    std::fs::create_dir_all(&dir).ok();
    let dir_str = dir.to_str().unwrap();

    let mut alliance_counts = HashMap::new();
    alliance_counts.insert(
        "COB".to_string(),
        AllianceStats {
            construction_requests: 5,
            research_requests: 3,
            troops_requests: 4,
        },
    );
    let stats = StatsResponse {
        alliance_counts,
        time_slot_popularity: None,
        construction_start_time: None,
        research_start_time: None,
        troops_start_time: None,
        construction_time_slot_popularity: None,
        research_time_slot_popularity: None,
        troops_time_slot_popularity: None,
    };

    save_statistics(dir_str, "testacct", 1, &stats)
        .await
        .unwrap();
    let loaded = load_statistics(dir_str, "testacct", 1).await;
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(
        loaded
            .alliance_counts
            .get("COB")
            .unwrap()
            .construction_requests,
        5
    );

    std::fs::remove_dir_all(&dir).ok();
}
