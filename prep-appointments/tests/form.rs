//! Tests for form module (submission validation).

use prep_appointments::form::{validate_submission, FormSubmissionRequest};

fn valid_request() -> FormSubmissionRequest {
    FormSubmissionRequest {
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
        wants_research: false,
        research_speedups: None,
        research_truegold_dust: None,
        research_time_slots: vec![],
        wants_troops: false,
        troops_speedups: None,
        troops_time_slots: vec![],
        additional_notes: None,
        suggestions: None,
    }
}

#[test]
fn test_validate_submission_valid() {
    let req = valid_request();
    assert!(validate_submission(&req).is_ok());
}

#[test]
fn test_validate_submission_empty_character_name() {
    let mut req = valid_request();
    req.character_name = "   ".to_string();
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req)
        .unwrap_err()
        .contains("Character name"));
}

#[test]
fn test_validate_submission_empty_player_id() {
    let mut req = valid_request();
    req.player_id = "".to_string();
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req).unwrap_err().contains("Player ID"));
}

#[test]
fn test_validate_submission_invalid_player_id() {
    let mut req = valid_request();
    req.player_id = "abc123".to_string();
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req).unwrap_err().contains("digits"));
}

#[test]
fn test_validate_submission_invalid_submission_type() {
    let mut req = valid_request();
    req.submission_type = "Invalid".to_string();
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req)
        .unwrap_err()
        .contains("submission type"));
}

#[test]
fn test_validate_submission_empty_alliance() {
    let mut req = valid_request();
    req.alliance = "".to_string();
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req).unwrap_err().contains("Alliance"));
}

#[test]
fn test_validate_submission_non_of_above_without_custom() {
    let mut req = valid_request();
    req.alliance = "Non of the above".to_string();
    req.custom_alliance = None;
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req)
        .unwrap_err()
        .contains("Custom alliance"));
}

#[test]
fn test_validate_submission_construction_too_few_slots() {
    let mut req = valid_request();
    req.construction_time_slots = vec![1, 2, 3];
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req)
        .unwrap_err()
        .contains("5 time slots"));
}

#[test]
fn test_validate_submission_no_day_selected() {
    let mut req = valid_request();
    req.wants_construction = false;
    req.wants_research = false;
    req.wants_troops = false;
    assert!(validate_submission(&req).is_err());
    assert!(validate_submission(&req)
        .unwrap_err()
        .contains("At least one day type"));
}

#[test]
fn test_validate_submission_resubmission() {
    let mut req = valid_request();
    req.submission_type = "Re-Submission".to_string();
    assert!(validate_submission(&req).is_ok());
}
