use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Represents a scheduled appointment for a specific day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledAppointment {
    pub player_id: String,
    pub name: String,
    pub alliance: String,
    pub slot: u8,
    pub priority_score: u32,
}

/// Schedule for a single day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySchedule {
    pub appointments: HashMap<u8, ScheduledAppointment>, // slot -> appointment
    pub unassigned: Vec<String>, // player IDs that couldn't be assigned
}

/// Represents a move in a chain of slot reassignments
#[derive(Debug, Clone)]
pub struct Move {
    pub player_id: String,
    pub from_slot: u8,
    pub to_slot: u8,
}

