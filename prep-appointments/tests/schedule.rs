//! Tests for schedule module (slot_utils, construction, research, troops, generic, move_chain).

mod common;

use prep_appointments::schedule::generic::schedule_day_generic;
use prep_appointments::schedule::move_chain::{apply_move_chain, find_move_chain};
use prep_appointments::schedule::slot_utils::calculate_slot_rankings;
use prep_appointments::schedule::{
    calculate_time_slots, minutes_to_time_string, parse_time_to_minutes, schedule_construction_day,
    schedule_research_day, schedule_troops_day, slot_to_time,
};

#[test]
fn test_slot_to_time() {
    assert_eq!(slot_to_time(1), "00:00");
    assert_eq!(slot_to_time(2), "00:15");
    assert_eq!(slot_to_time(3), "00:45");
    assert_eq!(slot_to_time(4), "01:15");
    assert_eq!(slot_to_time(5), "01:45");
    assert_eq!(slot_to_time(49), "23:45");
}

#[test]
fn test_parse_time_to_minutes() {
    assert_eq!(parse_time_to_minutes("00:00"), Some(0));
    assert_eq!(parse_time_to_minutes("00:15"), Some(15));
    assert_eq!(parse_time_to_minutes("01:30"), Some(90));
    assert_eq!(parse_time_to_minutes("23:59"), Some(24 * 60 - 1));
    assert_eq!(parse_time_to_minutes("00:00"), Some(0));
    assert_eq!(parse_time_to_minutes("invalid"), None);
    assert_eq!(parse_time_to_minutes("25:00"), None); // invalid hours
    assert_eq!(parse_time_to_minutes("12:60"), None); // invalid minutes
}

#[test]
fn test_minutes_to_time_string() {
    assert_eq!(minutes_to_time_string(0), "00:00");
    assert_eq!(minutes_to_time_string(90), "01:30");
    assert_eq!(minutes_to_time_string(1439), "23:59");
}

#[test]
fn test_calculate_time_slots_default() {
    let slots = calculate_time_slots("00:00", None);
    assert!(!slots.is_empty());
    assert_eq!(slots[0], (1, "00:00".to_string()));
    assert_eq!(slots[1], (2, "00:15".to_string()));
    assert_eq!(slots[2], (3, "00:45".to_string()));
}

#[test]
fn test_calculate_time_slots_with_end() {
    let slots = calculate_time_slots("00:00", Some("01:00"));
    assert!(!slots.is_empty());
    assert_eq!(slots[0], (1, "00:00".to_string()));
    assert_eq!(slots[1], (2, "00:15".to_string()));
    assert_eq!(slots[2], (3, "00:45".to_string()));
}

#[test]
fn test_calculate_slot_rankings() {
    let list = vec![vec![1, 2, 3], vec![1, 2], vec![1]];
    let rankings = calculate_slot_rankings(&list);
    assert_eq!(rankings.get(&1), Some(&3));
    assert_eq!(rankings.get(&2), Some(&2));
    assert_eq!(rankings.get(&3), Some(&1));
}

// ============ Construction day ============

#[test]
fn test_schedule_construction_day_basic() {
    let entries = vec![
        common::make_entry(
            "p1",
            true,
            false,
            false,
            vec![1, 2, 3],
            vec![],
            vec![],
            100,
            0,
            0,
        ),
        common::make_entry(
            "p2",
            true,
            false,
            false,
            vec![1, 2, 4],
            vec![],
            vec![],
            80,
            0,
            0,
        ),
    ];
    let schedule = schedule_construction_day(&entries);
    assert_eq!(schedule.appointments.len(), 2);
    assert!(schedule.unassigned.is_empty());
    // p1 has higher score, gets preferred slot (1 has highest rank if both want it)
    assert!(schedule.appointments.values().any(|a| a.player_id == "p1"));
    assert!(schedule.appointments.values().any(|a| a.player_id == "p2"));
}

#[test]
fn test_schedule_construction_day_empty_entries() {
    let entries: Vec<prep_appointments::parser::AppointmentEntry> = vec![];
    let schedule = schedule_construction_day(&entries);
    assert!(schedule.appointments.is_empty());
    assert!(schedule.unassigned.is_empty());
}

#[test]
fn test_schedule_construction_day_no_construction_want() {
    let entries = vec![common::make_entry(
        "p1",
        false,
        true,
        false,
        vec![],
        vec![1, 2],
        vec![],
        0,
        50,
        0,
    )];
    let schedule = schedule_construction_day(&entries);
    assert!(schedule.appointments.is_empty());
    assert_eq!(schedule.unassigned.len(), 0); // filtered out, not unassigned
}

// ============ Research day ============

#[test]
fn test_schedule_research_day_basic() {
    let entries = vec![
        common::make_entry(
            "p1",
            true,
            true,
            false,
            vec![1, 2],
            vec![1, 2, 3],
            vec![],
            100,
            50,
            0,
        ),
        common::make_entry(
            "p2",
            false,
            true,
            false,
            vec![],
            vec![2, 3, 4],
            vec![],
            0,
            30,
            0,
        ),
    ];
    let construction = common::schedule_with_appointments(vec![(49, "p1", 100)]);
    let schedule = schedule_research_day(&entries, &construction);
    assert!(schedule.appointments.len() >= 1);
    // p1 should be in slot 1 (last construction -> first research)
    assert_eq!(
        schedule.appointments.get(&1).map(|a| a.player_id.as_str()),
        Some("p1")
    );
}

#[test]
fn test_schedule_research_day_empty_construction() {
    let entries = vec![common::make_entry(
        "p1",
        false,
        true,
        false,
        vec![],
        vec![1, 2, 3],
        vec![],
        0,
        50,
        0,
    )];
    let construction = common::empty_schedule();
    let schedule = schedule_research_day(&entries, &construction);
    assert!(schedule.appointments.len() >= 1);
}

// ============ Troops day ============

#[test]
fn test_schedule_troops_day_basic() {
    let entries = vec![
        common::make_entry(
            "p1",
            false,
            false,
            true,
            vec![],
            vec![],
            vec![1, 2, 3],
            0,
            0,
            10,
        ),
        common::make_entry(
            "p2",
            false,
            false,
            true,
            vec![],
            vec![],
            vec![1, 2, 4],
            0,
            0,
            8,
        ),
    ];
    let schedule = schedule_troops_day(&entries);
    assert_eq!(schedule.appointments.len(), 2);
    assert!(schedule.unassigned.is_empty());
}

// ============ Generic ============

#[test]
fn test_schedule_day_generic_basic() {
    let entries = vec![
        common::make_entry(
            "p1",
            false,
            true,
            false,
            vec![],
            vec![1, 2, 3],
            vec![],
            0,
            100,
            0,
        ),
        common::make_entry(
            "p2",
            false,
            true,
            false,
            vec![],
            vec![2, 3, 4],
            vec![],
            0,
            80,
            0,
        ),
    ];
    let schedule = schedule_day_generic(
        &entries,
        |e| e.wants_research,
        |e| &e.research_available_slots,
        |e| e.research_score,
    );
    assert!(schedule.appointments.len() >= 1);
}

// ============ Move chain ============

#[test]
fn test_find_move_chain_direct_free_slot() {
    use prep_appointments::parser::AppointmentEntry;
    use std::collections::HashSet;

    let mut schedule = std::collections::HashMap::new();
    schedule.insert(1u8, common::make_appointment("p1", 1, 100));
    let used_slots: HashSet<u8> = [1].into_iter().collect();
    let entry = AppointmentEntry {
        alliance: "T".to_string(),
        name: "P1".to_string(),
        player_id: "p1".to_string(),
        wants_construction: true,
        wants_research: false,
        wants_troops: false,
        construction_speedups: 0,
        research_speedups: 0,
        troops_speedups: 0,
        construction_truegold: 0,
        construction_tempered_truegold: 0,
        construction_score: 100,
        research_truegold_dust: 0,
        research_score: 0,
        construction_available_slots: vec![1, 2, 3],
        research_available_slots: vec![],
        troops_available_slots: vec![],
    };
    let mut entry_map = std::collections::HashMap::new();
    entry_map.insert("p1".to_string(), &entry);
    let mut visited = HashSet::new();
    let locked: HashSet<u8> = HashSet::new();

    let chain = find_move_chain(
        "p1",
        1,
        &[1, 2, 3],
        &schedule,
        &used_slots,
        &entry_map,
        |e| &e.construction_available_slots,
        1,
        5,
        &mut visited,
        &locked,
    );
    assert!(chain.is_some());
    let chain = chain.unwrap();
    assert_eq!(chain.len(), 1);
    assert_eq!(chain[0].from_slot, 1);
    assert_eq!(chain[0].to_slot, 2); // or 3 - any free slot
}

#[test]
fn test_apply_move_chain() {
    use prep_appointments::schedule::types::Move;
    use std::collections::{HashMap, HashSet};

    let mut schedule = HashMap::new();
    schedule.insert(1u8, common::make_appointment("p1", 1, 100));
    let mut used_slots: HashSet<u8> = [1].into_iter().collect();

    let moves = vec![Move {
        player_id: "p1".to_string(),
        from_slot: 1,
        to_slot: 2,
    }];
    apply_move_chain(&moves, &mut schedule, &mut used_slots);

    assert!(schedule.get(&1).is_none());
    assert!(schedule.get(&2).is_some());
    assert_eq!(schedule.get(&2).unwrap().player_id, "p1");
    assert!(!used_slots.contains(&1));
    assert!(used_slots.contains(&2));
}
