//! Unit tests for alliance invites (has_alliance_access, load/save_invites).

use std::collections::HashMap;
use std::sync::Mutex;

use prep_appointments::web::alliance_invites::{
    has_alliance_access, load_invites, save_invites, AllianceInvite,
};
use prep_appointments::web::{Account, AppState};

fn make_app_state(data_dir: String, accounts: HashMap<String, Account>) -> AppState {
    AppState {
        accounts: Mutex::new(accounts),
        schedules: Mutex::new(HashMap::new()),
        forms: Mutex::new(HashMap::new()),
        current_forms: Mutex::new(HashMap::new()),
        data_dir,
        pg: None,
        oauth_hmac_key: vec![0u8; 32],
    }
}

#[tokio::test]
async fn test_has_alliance_access_owner() {
    let dir = std::env::temp_dir().join("test_has_access_owner");
    std::fs::create_dir_all(&dir).ok();
    let mut accounts = HashMap::new();
    accounts.insert(
        "owner".to_string(),
        Account {
            account_name: "owner".to_string(),
            server_number: 140,
            password: String::new(),
            in_game_name: "Owner".to_string(),
            player_id: None,
            oauth_provider: None,
            oauth_id: None,
            admin: false,
            alliance_access: true,
            alliance_id: None,
            alliance_tag: None,
            alliance_name: None,
            friend_code: None,
        },
    );
    let state = make_app_state(dir.to_str().unwrap().to_string(), accounts);
    assert!(has_alliance_access(&state, "owner", "owner", 140, "slaughterhouse").await);
    assert!(has_alliance_access(&state, "Owner", "owner", 140, "slaughterhouse").await);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_has_alliance_access_via_invite() {
    let dir = std::env::temp_dir().join("test_has_access_invite");
    std::fs::create_dir_all(&dir).ok();
    let dir_str = dir.to_str().unwrap().to_string();
    let my_fc = "ABC123XYZ789".to_string();
    let mut accounts = HashMap::new();
    accounts.insert(
        "invitee".to_string(),
        Account {
            account_name: "invitee".to_string(),
            server_number: 1,
            password: String::new(),
            in_game_name: "Invitee".to_string(),
            player_id: None,
            oauth_provider: None,
            oauth_id: None,
            admin: false,
            alliance_access: false,
            alliance_id: None,
            alliance_tag: None,
            alliance_name: None,
            friend_code: Some(my_fc.clone()),
        },
    );
    let mut invites = HashMap::new();
    invites.insert(
        "inv_1".to_string(),
        AllianceInvite {
            id: "inv_1".to_string(),
            from_account: "owner".to_string(),
            from_server: 140,
            alliance_slug: "slaughterhouse".to_string(),
            alliance_name: "Slaughterhouse".to_string(),
            to_friend_code: my_fc,
            status: "accepted".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
        },
    );
    save_invites(&dir_str, &invites).await.unwrap();
    let state = make_app_state(dir_str, accounts);
    assert!(has_alliance_access(&state, "invitee", "owner", 140, "slaughterhouse").await);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_has_alliance_access_no_access() {
    let dir = std::env::temp_dir().join("test_has_access_no");
    std::fs::create_dir_all(&dir).ok();
    let mut accounts = HashMap::new();
    accounts.insert(
        "stranger".to_string(),
        Account {
            account_name: "stranger".to_string(),
            server_number: 1,
            password: String::new(),
            in_game_name: "Stranger".to_string(),
            player_id: None,
            oauth_provider: None,
            oauth_id: None,
            admin: false,
            alliance_access: false,
            alliance_id: None,
            alliance_tag: None,
            alliance_name: None,
            friend_code: Some("DIFFERENT12".to_string()),
        },
    );
    let state = make_app_state(dir.to_str().unwrap().to_string(), accounts);
    assert!(!has_alliance_access(&state, "stranger", "owner", 140, "slaughterhouse").await);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_load_save_invites_roundtrip() {
    let dir = std::env::temp_dir().join("test_invites_roundtrip");
    std::fs::create_dir_all(&dir).ok();
    let dir_str = dir.to_str().unwrap().to_string();
    let mut invites = HashMap::new();
    invites.insert(
        "inv_99".to_string(),
        AllianceInvite {
            id: "inv_99".to_string(),
            from_account: "alice".to_string(),
            from_server: 1,
            alliance_slug: "cob".to_string(),
            alliance_name: "COB".to_string(),
            to_friend_code: "FRIEND123456".to_string(),
            status: "pending".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
        },
    );
    save_invites(&dir_str, &invites).await.unwrap();
    let loaded = load_invites(&dir_str).await;
    assert_eq!(loaded.len(), 1);
    let inv = loaded.get("inv_99").unwrap();
    assert_eq!(inv.from_account, "alice");
    assert_eq!(inv.alliance_slug, "cob");
    assert_eq!(inv.status, "pending");
    std::fs::remove_dir_all(&dir).ok();
}
