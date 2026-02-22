//! Tests for parser module (CSV loading, player submission check).

mod common;

use prep_appointments::parser::{has_player_submitted, load_appointments};

#[test]
fn test_load_appointments_basic() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_appointments_basic.csv");
    let row = "01/01/2025 12.00.00,TestAlliance,,Player1,12345678,New submission,Yes,10,5,0,\"00:00, 00:15, 00:45, 01:15, 01:45\",Yes,5,3,\"00:00, 00:15, 00:45, 01:15, 01:45\",Yes,8,\"00:00, 00:15, 00:45, 01:15, 01:45\",,";
    common::create_test_csv(&path, &[row]).unwrap();
    let result = load_appointments(&path, None, None, None);
    std::fs::remove_file(&path).ok();
    assert!(
        result.is_ok(),
        "load_appointments failed: {:?}",
        result.err()
    );
    let entries = result.unwrap();
    assert!(!entries.is_empty(), "Expected at least one entry");
    assert_eq!(entries[0].player_id, "12345678");
    assert_eq!(entries[0].name, "Player1");
    assert_eq!(entries[0].alliance, "TestAlliance");
}

#[test]
fn test_has_player_submitted_found() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_has_submitted.csv");
    let row = "01/01/2025 12.00.00,Alliance,,Name,99999,New submission,Yes,0,0,0,\"00:00, 00:15, 00:45, 01:15, 01:45\",No,0,0,,No,0,,,";
    common::create_test_csv(&path, &[row]).unwrap();
    let entries = load_appointments(&path, None, None, None).unwrap();
    assert_eq!(entries.len(), 1, "load_appointments should find the row");
    assert_eq!(entries[0].player_id, "99999");
    assert!(has_player_submitted(&path, "99999"));
    assert!(has_player_submitted(&path, " 99999 "));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_has_player_submitted_not_found() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_not_submitted.csv");
    let row = "01/01/2025 12.00.00,Alliance,,Name,11111,New submission,Yes,0,0,0,\"00:00, 00:15\",No,0,0,,No,0,,,";
    common::create_test_csv(&path, &[row]).unwrap();
    assert!(!has_player_submitted(&path, "99999"));
    assert!(!has_player_submitted(&path, ""));
    std::fs::remove_file(&path).ok();
}
