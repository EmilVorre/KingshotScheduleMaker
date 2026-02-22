//! Tests for Kingshot API module (stove level labels).

use prep_appointments::kingshot_api::stove_lv_to_label;

#[test]
fn test_stove_lv_to_label_levels() {
    assert_eq!(stove_lv_to_label(1), "Level 1");
    assert_eq!(stove_lv_to_label(15), "Level 15");
    assert_eq!(stove_lv_to_label(30), "Level 30");
}

#[test]
fn test_stove_lv_to_label_30_minus() {
    assert_eq!(stove_lv_to_label(31), "30-1");
    assert_eq!(stove_lv_to_label(34), "30-4");
}

#[test]
fn test_stove_lv_to_label_tg() {
    assert_eq!(stove_lv_to_label(35), "TG 1");
    assert_eq!(stove_lv_to_label(36), "TG 1 - 1");
    assert_eq!(stove_lv_to_label(39), "TG 1 - 4");
    assert_eq!(stove_lv_to_label(40), "TG 2");
    assert_eq!(stove_lv_to_label(41), "TG 2 - 1");
}
