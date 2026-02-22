//! Tests for display module.

mod common;

use prep_appointments::display::{format_player_name, print_day_schedule, write_schedule_to_file};

#[test]
fn test_format_player_name_with_alliance() {
    assert_eq!(format_player_name("COB", "Vor"), "[COB] Vor");
    assert_eq!(
        format_player_name("TestAlliance", "Player1"),
        "[TestAlliance] Player1"
    );
}

#[test]
fn test_format_player_name_without_alliance() {
    assert_eq!(format_player_name("", "SoloPlayer"), "SoloPlayer");
}

#[test]
fn test_write_schedule_to_file() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_schedule_output.txt");

    let schedule = common::schedule_with_appointments(vec![(1, "p1", 100), (2, "p2", 80)]);

    let result = write_schedule_to_file("Test Day", &schedule, path.to_str().unwrap());
    assert!(
        result.is_ok(),
        "write_schedule_to_file failed: {:?}",
        result.err()
    );

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("** Test Day **"));
    assert!(content.contains("00:00"));
    assert!(content.contains("[EMPTY]"));

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_print_day_schedule_no_panic() {
    let schedule = common::schedule_with_appointments(vec![(1, "p1", 100)]);
    let entries = vec![common::make_entry(
        "p1",
        true,
        false,
        false,
        vec![1],
        vec![],
        vec![],
        100,
        0,
        0,
    )];
    print_day_schedule("Test", &schedule, &entries, |e| e.construction_score);
}
