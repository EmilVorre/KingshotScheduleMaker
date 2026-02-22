//! Tests for form export (export_submission_to_csv).

use prep_appointments::form::export_submission_to_csv;
use prep_appointments::form::FormSubmission;
use prep_appointments::parser::load_appointments;

#[test]
fn test_export_submission_to_csv_creates_file() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_export_submission.csv");
    let _ = std::fs::remove_file(&path);

    let submission = FormSubmission {
        timestamp: "01/01/2025 12.00.00".to_string(),
        alliance: "TestAlliance".to_string(),
        custom_alliance: None,
        character_name: "TestPlayer".to_string(),
        player_id: "12345678".to_string(),
        submission_type: "New submission".to_string(),
        wants_construction: true,
        construction_speedups: Some(10),
        construction_truegold: Some(5),
        construction_tempered_truegold: None,
        construction_time_slots: vec![1, 2, 3, 4, 5],
        wants_research: true,
        research_speedups: Some(5),
        research_truegold_dust: Some(3),
        research_time_slots: vec![1, 2, 3, 4, 5],
        wants_troops: false,
        troops_speedups: None,
        troops_time_slots: vec![],
        additional_notes: None,
        suggestions: None,
    };

    let result = export_submission_to_csv(
        &submission,
        &path,
        ("00:00", None),
        ("00:00", None),
        ("00:00", None),
    );
    assert!(result.is_ok(), "export failed: {:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("TestAlliance"));
    assert!(content.contains("TestPlayer"));
    assert!(content.contains("12345678"));

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_export_and_load_roundtrip() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_export_roundtrip.csv");
    let _ = std::fs::remove_file(&path);

    let submission = FormSubmission {
        timestamp: "01/01/2025 12.00.00".to_string(),
        alliance: "RoundtripAlliance".to_string(),
        custom_alliance: None,
        character_name: "RoundtripPlayer".to_string(),
        player_id: "99999999".to_string(),
        submission_type: "New submission".to_string(),
        wants_construction: true,
        construction_speedups: Some(8),
        construction_truegold: Some(2),
        construction_tempered_truegold: None,
        construction_time_slots: vec![1, 2, 3, 4, 5],
        wants_research: false,
        research_speedups: None,
        research_truegold_dust: None,
        research_time_slots: vec![],
        wants_troops: false,
        troops_speedups: None,
        troops_time_slots: vec![],
        additional_notes: None,
        suggestions: None,
    };

    export_submission_to_csv(
        &submission,
        &path,
        ("00:00", None),
        ("00:00", None),
        ("00:00", None),
    )
    .unwrap();

    let entries = load_appointments(&path, None, None, None).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].player_id, "99999999");
    assert_eq!(entries[0].name, "RoundtripPlayer");
    assert_eq!(entries[0].alliance, "RoundtripAlliance");

    std::fs::remove_file(&path).ok();
}
