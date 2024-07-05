use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct RawEmployeeShift {
    #[serde(rename(deserialize = "ShiftID"))]
    pub shift_id: u64,
    #[serde(rename(deserialize = "EmployeeID"))]
    pub employee_id: u64,
    #[serde(rename(deserialize = "StartTime"))]
    pub start_time: String,
    #[serde(rename(deserialize = "EndTime"))]
    pub end_time: String,
}

#[derive(Debug)]
pub struct EmployeeShift {
    pub shift_id: u64,
    pub employee_id: u64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
pub struct EmployeeShiftSummary {
    #[serde(rename(serialize = "EmployeeID"))]
    pub employee_id: u64,
    #[serde(rename(serialize = "StartOfWeek"))]
    pub start_of_week: String,
    #[serde(rename(serialize = "RegularHours"))]
    pub regular_hours: f64,
    #[serde(rename(serialize = "OvertimeHours"))]
    pub overtime_hours: f64,
    #[serde(rename(serialize = "InvalidShifts"))]
    pub invalid_shifts: Vec<u64>,
}
