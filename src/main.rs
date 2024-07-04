use std::{
    collections::{BTreeMap, HashMap},
    io::BufReader,
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, Datelike, Days, NaiveDate, Timelike, Utc};
use chrono_tz::US::Central;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Clone)]
struct RawEmployeeShift {
    #[serde(alias = "ShiftID")]
    shift_id: u64,
    #[serde(alias = "EmployeeID")]
    employee_id: u64,
    #[serde(alias = "StartTime")]
    start_time: String,
    #[serde(alias = "EndTime")]
    end_time: String,
}

#[derive(Debug, Clone)]
struct EmployeeShift {
    shift_id: u64,
    employee_id: u64,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

#[derive(Serialize, Debug, Clone)]
struct EmployeeShiftSummary {
    employee_id: u64,
    start_of_week: String,
    regular_hours: f64,
    overtime_hours: f64,
    invalid_shifts: Vec<u64>,
}

fn main() {
    let path = PathBuf::from_str("./test_dataset_multiple.json").unwrap();
    // let path = PathBuf::from_str("./dataset_(1).json").unwrap();
    let shifts = read_shifts(&path);
    // shifts.first().unwrap().start_time.timezone();

    summarize_all_employee_hours(shifts);
}

fn read_shifts(path: &Path) -> Vec<EmployeeShift> {
    let file = std::fs::File::open(path).unwrap();
    let reader = BufReader::new(file);
    let raw_shifts: Vec<RawEmployeeShift> = serde_json::from_reader(reader).unwrap();

    raw_shifts
        .into_iter()
        .map(|shift| validate_shift(&shift))
        .collect()
}

fn validate_shift(shift: &RawEmployeeShift) -> EmployeeShift {
    dbg!(DateTime::parse_from_rfc3339(&shift.start_time).unwrap());
    dbg!(DateTime::parse_from_rfc3339(&shift.end_time).unwrap());

    EmployeeShift {
        shift_id: shift.shift_id,
        employee_id: shift.employee_id,
        start_time: DateTime::parse_from_rfc3339(&shift.start_time)
            .unwrap()
            .into(),
        end_time: DateTime::parse_from_rfc3339(&shift.end_time)
            .unwrap()
            .into(),
    }
}

// what about shift that starts saturday and ends sunday?
fn summarize_employee_hours(employee_id: u64) -> EmployeeShiftSummary {
    todo!()
}
// {
//     "ShiftID": 26629382113,
//     "EmployeeID": 34009849,
//     "StartTime": "2021-08-25T22:00:00.000000Z",
//     "EndTime": "2021-08-26T11:30:00.000000Z"
// },
// loop through all of the shifts
// figure out if shift is valid
// add shift_id it to invalid shift field array for the week of the shift
// get the start of the shift and end of shift dates
// if end of shift isn't the same week as start of shift then split hours between weeks
// when splitting hours, add the shift that moved to the next week to its appropriate week
// if a summary doesn't exist, create that summary in a hash table
// if total hours is greater than 40 then add that to overtime for week
fn summarize_all_employee_hours(shifts: Vec<EmployeeShift>) -> Vec<EmployeeShiftSummary> {
    let shifts = {
        let mut map = BTreeMap::new();
        for shift in shifts {
            map.insert((shift.employee_id, shift.start_time), shift);
        }
        map
    };

    let summaries: HashMap<(u64, NaiveDate), EmployeeShiftSummary> = HashMap::new();
    for shift in shifts.values() {
        let (start_of_week_for_start_time, end_of_week_for_start_time, start_of_week_for_end_time) =
            get_start_of_week_for_shift(shift.start_time, shift.end_time);
        // let end_of_week_for_start_time = get_start_of_week_for_shift(shift.start_time);
        // let start_of_week_for_end_time = end_of_week_for_start_time;

        if start_of_week_for_start_time != start_of_week_for_end_time {
            let hours_first_week = end_of_week_for_start_time - shift.start_time;
            let hours_second_week = shift.end_time - start_of_week_for_end_time;

            // insert these hours into the summaries map
        } else {
            let hours = shift.end_time - shift.start_time;

            // insert into summaries map
        }
        // start_time_week = shift.start_time.date_naive().week() ;
        // end_time_week =
    }
    // dbg!(shifts);
    todo!()
}

// remember to convert to CDT when calculating start date and end date of a week
fn get_start_of_week_for_shift(
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> (DateTime<Utc>, DateTime<Utc>, DateTime<Utc>) {
    let sunday = start_time - Days::new(start_time.weekday().num_days_from_sunday() as u64);
    let central_time = sunday.with_timezone(&Central);
    let start_work_week =
        central_time - Days::new(central_time.weekday().num_days_from_sunday() as u64);
    let start_work_week_for_start_time = start_work_week
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();
    let end_work_week_for_start_time = start_work_week_for_start_time + Days::new(7);
    let start_work_week_for_end_time = {
        let sunday = end_time - Days::new(start_time.weekday().num_days_from_sunday() as u64);
        let central_time = sunday.with_timezone(&Central);
        let start_work_week =
            central_time - Days::new(central_time.weekday().num_days_from_sunday() as u64);
        start_work_week
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
    };

    (
        start_work_week_for_start_time.to_utc(),
        end_work_week_for_start_time.to_utc(),
        start_work_week_for_end_time.to_utc(),
    )
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use chrono::SecondsFormat;
    use chrono::TimeZone;
    use chrono_tz::US::Central;

    use super::*;

    #[test]
    fn test_get_week_boundaries_for_shift_start_and_end_time() {
        let start_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2024-07-03T11:00:00.000000Z")
            .unwrap()
            .into();

        let end_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2024-07-03T15:00:00.000000Z")
            .unwrap()
            .into();

        let (
            expected_start_week_for_start_time,
            expected_end_date_for_start_time,
            expected_start_week_for_end_time,
        ) = get_start_of_week_for_shift(start_time, end_time);
        assert_eq!(
            expected_start_week_for_start_time,
            Central
                .with_ymd_and_hms(2024, 6, 30, 0, 0, 0)
                .unwrap()
                .to_utc()
        );

        assert_eq!(
            expected_end_date_for_start_time,
            Central
                .with_ymd_and_hms(2024, 7, 7, 0, 0, 0)
                .unwrap()
                .to_utc()
        );
        assert_eq!(
            expected_start_week_for_end_time,
            Central
                .with_ymd_and_hms(2024, 6, 30, 0, 0, 0)
                .unwrap()
                .to_utc()
        );
    }

    #[test]
    fn test_get_week_boundaries_for_shift_start_and_end_time_spanning_saturday_to_sunday() {
        let start_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2024-06-30T01:00:00.000000Z")
            .unwrap()
            .into();

        let end_time: DateTime<Utc> = DateTime::parse_from_rfc3339("2024-06-30T07:00:00.000000Z")
            .unwrap()
            .into();

        let (
            expected_start_week_for_start_time,
            expected_end_date_for_start_time,
            expected_start_week_for_end_time,
        ) = get_start_of_week_for_shift(start_time, end_time);
        assert_eq!(
            expected_start_week_for_start_time,
            Central
                .with_ymd_and_hms(2024, 6, 23, 0, 0, 0)
                .unwrap()
                .to_utc()
        );

        assert_eq!(
            expected_end_date_for_start_time,
            Central
                .with_ymd_and_hms(2024, 6, 30, 0, 0, 0)
                .unwrap()
                .to_utc()
        );
        assert_eq!(
            expected_start_week_for_end_time,
            Central
                .with_ymd_and_hms(2024, 6, 30, 0, 0, 0)
                .unwrap()
                .to_utc()
        );
    }

    #[test]
    fn test_read_employee_shift() {
        let path = PathBuf::from_str("./test_dataset.json").unwrap();
        let shifts = read_shifts(&path);

        assert_eq!(shifts[0].shift_id, 2663141019);
        assert_eq!(shifts[0].employee_id, 41488322);
        assert_eq!(
            shifts[0]
                .start_time
                .to_rfc3339_opts(SecondsFormat::Micros, true),
            "2021-08-30T12:30:00.000000Z"
        );
        assert_eq!(
            shifts[0]
                .end_time
                .to_rfc3339_opts(SecondsFormat::Micros, true),
            "2021-08-30T21:00:00.000000Z"
        );
        assert_eq!(shifts.len(), 1);
    }

    #[test]
    fn test_read_multiple_employee_shifts() {
        let path = PathBuf::from_str("./test_dataset_multiple.json").unwrap();
        let shifts = read_shifts(&path);

        assert_eq!(shifts.len(), 3);

        assert_eq!(shifts[0].shift_id, 2663141019);
        assert_eq!(shifts[0].employee_id, 41488322);
        assert_eq!(
            shifts[0]
                .start_time
                .to_rfc3339_opts(SecondsFormat::Micros, true),
            "2021-08-30T12:30:00.000000Z"
        );
        assert_eq!(
            shifts[0]
                .end_time
                .to_rfc3339_opts(SecondsFormat::Micros, true),
            "2021-08-30T21:00:00.000000Z"
        );

        assert_eq!(shifts[2].shift_id, 2662828955);
        assert_eq!(shifts[2].employee_id, 38410756);
        assert_eq!(
            shifts[2]
                .start_time
                .to_rfc3339_opts(SecondsFormat::Micros, true),
            "2021-08-27T13:00:00.000000Z"
        );
        assert_eq!(
            shifts[2]
                .end_time
                .to_rfc3339_opts(SecondsFormat::Micros, true),
            "2021-08-28T01:30:00.000000Z"
        );
    }

    #[test]
    fn test_validate_raw_employee_shift() {
        let shifts = [RawEmployeeShift {
            shift_id: 1,
            employee_id: 2,
            start_time: "2021-08-30T12:30:00.000000Z".to_string(),
            end_time: "2021-08-30T21:00:00.000000Z".to_string(),
        }];

        let shift = validate_shift(&shifts[0]);
        let expected_start_time: DateTime<Utc> =
            DateTime::parse_from_rfc3339("2021-08-30T12:30:00.000000Z")
                .unwrap()
                .into();
        let expected_end_time: DateTime<Utc> =
            DateTime::parse_from_rfc3339("2021-08-30T21:00:00.000000Z")
                .unwrap()
                .into();

        assert_eq!(shift.shift_id, 1);
        assert_eq!(shift.employee_id, 2);
        assert_eq!(shift.start_time, expected_start_time);
        assert_eq!(shift.end_time, expected_end_time);
    }

    // #[test]
    // fn test_summarize_employee_hours() {
    //     let path = PathBuf::from_str("./test_dataset.json").unwrap();
    //     let shifts = read_shifts(&path);

    //     let summary = summarize_employee_hours(shifts[0].employee_id);
    //     let shift = shifts[0];

    //     assert_eq!(summary.employee_id, shift.employee_id);
    //     assert_eq!(summary.start_of_week, shift.start_time.iso_week());
    //     assert_eq!(summary.regular_hours, 9);
    //     assert_eq!(summary.overtime_hours, 0);
    //     assert_eq!(summary.invalid_shifts, []);
    // }
}
