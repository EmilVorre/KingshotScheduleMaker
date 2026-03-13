//! Unit tests for alliance organisation (alliance_to_slug, etc).

use prep_appointments::web::alliances::alliance_to_slug;

#[test]
fn test_alliance_to_slug_simple() {
    assert_eq!(alliance_to_slug("COB"), "cob");
    assert_eq!(alliance_to_slug("Slaughterhouse"), "slaughterhouse");
}

#[test]
fn test_alliance_to_slug_with_spaces() {
    assert_eq!(alliance_to_slug("My Alliance"), "my_alliance");
    assert_eq!(alliance_to_slug("The  Best  Alliance"), "the_best_alliance");
}

#[test]
fn test_alliance_to_slug_special_chars() {
    assert_eq!(alliance_to_slug("A-B-C"), "a_b_c");
    assert_eq!(alliance_to_slug("Test.Alliance!"), "test_alliance");
}

#[test]
fn test_alliance_to_slug_truncates() {
    let long = "a".repeat(100);
    assert_eq!(alliance_to_slug(&long).len(), 64);
}
