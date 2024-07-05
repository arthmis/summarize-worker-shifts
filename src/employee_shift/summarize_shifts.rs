use crate::employee_shift::employee;
use employee::{EmployeeShift, EmployeeShiftSummary, RawEmployeeShift};
use std::{collections::HashMap, io::BufReader, path::Path};

use anyhow::{Context, Error};
use chrono::{DateTime, Datelike, Days, NaiveDate, Timelike, Utc};
use chrono_tz::US::Central;

pub fn summarize_shifts_from_json_file(path: &Path) -> Result<Vec<EmployeeShiftSummary>, Error> {
    let shifts = read_shifts(path)?;

    let summaries = summarize_all_employee_hours(shifts);
    let mut summaries: Vec<EmployeeShiftSummary> = summaries.into_values().collect();
    calculate_overtime_hours(&mut summaries);

    Ok(summaries)
}

fn read_shifts(path: &Path) -> Result<Vec<EmployeeShift>, Error> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.to_string_lossy()))?;

    let reader = BufReader::new(file);
    let raw_shifts: Vec<RawEmployeeShift> = serde_json::from_reader(reader).with_context(|| {
        "serde library has a bug when reporting the correct line number where error occurred. Reported error line will be wrong but the actual error will probably be on a nearby line"
    })?;

    Ok(raw_shifts
        .into_iter()
        .map(|shift| validate_shift(&shift).unwrap())
        .collect())
}

// would add validation to check end time is after start time
fn validate_shift(shift: &RawEmployeeShift) -> Result<EmployeeShift, Error> {
    Ok(EmployeeShift {
        shift_id: shift.shift_id,
        employee_id: shift.employee_id,
        start_time: DateTime::parse_from_rfc3339(&shift.start_time)
            .with_context(|| format!("start time was not rfc3339 compliant: {:?}", shift))?
            .into(),
        end_time: DateTime::parse_from_rfc3339(&shift.end_time)
            .with_context(|| format!("end time was not rfc3339 compliant: {:?}", shift))?
            .into(),
    })
}

fn calculate_overtime_hours(summaries: &mut [EmployeeShiftSummary]) {
    for summary in summaries.iter_mut() {
        if summary.regular_hours > 40. {
            summary.overtime_hours = summary.regular_hours - 40.;
            summary.regular_hours = 40.;
        }
    }
}

fn summarize_all_employee_hours(
    shifts: Vec<EmployeeShift>,
) -> HashMap<(u64, NaiveDate), EmployeeShiftSummary> {
    let shifts = {
        let mut map = HashMap::new();
        for shift in shifts {
            map.insert((shift.employee_id, shift.shift_id, shift.start_time), shift);
        }
        map
    };

    let mut summaries: HashMap<(u64, NaiveDate), EmployeeShiftSummary> = HashMap::new();
    for shift in shifts.values() {
        // need the start of week for the shift end time because it's possible that a shift crosses
        // into the next week, which means the start of the week for the end of the shift is in
        // the next week and is necessary for hours worked calculations
        let (
            start_of_week_for_start_time,
            end_of_week_for_start_time,
            start_of_week_for_end_time,
            start_of_week_date_for_start_time,
            start_of_week_date_for_end_time,
        ) = get_start_of_week_for_shift(shift.start_time, shift.end_time);

        if does_shift_overlap_with_another_for_employee(shift, &shifts) {
            summaries
                .entry((shift.employee_id, start_of_week_date_for_start_time))
                .and_modify(|summary| summary.invalid_shifts.push(shift.shift_id))
                .or_insert(EmployeeShiftSummary {
                    employee_id: shift.employee_id,
                    start_of_week: start_of_week_date_for_start_time.to_string(),
                    regular_hours: 0.,
                    overtime_hours: 0.,
                    invalid_shifts: vec![shift.shift_id],
                });
            continue;
        }

        if start_of_week_for_start_time != start_of_week_for_end_time {
            let hours_first_week = end_of_week_for_start_time - shift.start_time;
            let hours_next_week = shift.end_time - start_of_week_for_end_time;

            // add hours for the week start time is part of
            summaries
                .entry((shift.employee_id, start_of_week_date_for_start_time))
                .and_modify(|summary| {
                    summary.regular_hours += hours_first_week.num_minutes() as f64 / 60.
                })
                .or_insert(EmployeeShiftSummary {
                    employee_id: shift.employee_id,
                    start_of_week: start_of_week_date_for_start_time.to_string(),
                    regular_hours: hours_first_week.num_minutes() as f64 / 60.,
                    overtime_hours: 0.,
                    invalid_shifts: vec![],
                });

            // add hours for the week end time is part of
            summaries
                .entry((shift.employee_id, start_of_week_date_for_end_time))
                .and_modify(|summary| {
                    summary.regular_hours += hours_next_week.num_minutes() as f64 / 60.
                })
                .or_insert(EmployeeShiftSummary {
                    employee_id: shift.employee_id,
                    start_of_week: start_of_week_date_for_end_time.to_string(),
                    regular_hours: hours_next_week.num_minutes() as f64 / 60.,
                    overtime_hours: 0.,
                    invalid_shifts: vec![],
                });
        } else {
            let hours = shift.end_time - shift.start_time;

            summaries
                .entry((shift.employee_id, start_of_week_date_for_start_time))
                .and_modify(|summary| summary.regular_hours += hours.num_minutes() as f64 / 60.)
                .or_insert(EmployeeShiftSummary {
                    employee_id: shift.employee_id,
                    start_of_week: start_of_week_date_for_start_time.to_string(),
                    regular_hours: hours.num_minutes() as f64 / 60.,
                    overtime_hours: 0.,
                    invalid_shifts: vec![],
                });
        }
    }
    summaries
}

fn does_shift_overlap_with_another_for_employee(
    current_shift: &EmployeeShift,
    shifts: &HashMap<(u64, u64, DateTime<Utc>), EmployeeShift>,
) -> bool {
    for (_, other_shift) in shifts
        .iter()
        .filter(|(key, _)| key.0 == current_shift.employee_id)
    {
        if current_shift.shift_id != other_shift.shift_id {
            // check if current shift overlaps with other shift
            if current_shift.start_time > other_shift.start_time
                && current_shift.start_time < other_shift.end_time
            {
                return true;
            }
            if current_shift.end_time > other_shift.start_time
                && current_shift.end_time < other_shift.end_time
            {
                return true;
            }

            // check if other shift overlaps with current shift
            // two different checks are necessary in case a shift completely
            // encompasses the other shift
            if other_shift.start_time > current_shift.start_time
                && other_shift.start_time < current_shift.end_time
            {
                return true;
            }
            if other_shift.end_time > current_shift.start_time
                && other_shift.end_time < current_shift.end_time
            {
                return true;
            }
        }
    }

    false
}

fn get_start_of_week_for_shift(
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> (
    DateTime<Utc>,
    DateTime<Utc>,
    DateTime<Utc>,
    NaiveDate,
    NaiveDate,
) {
    // convert to central time before calculating sunday midnight date
    let start_time_central = start_time.with_timezone(&Central);
    let start_work_week =
        start_time_central - Days::new(start_time_central.weekday().num_days_from_sunday() as u64);
    let start_of_week_for_start_time = start_work_week
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();

    let end_of_work_week_for_start_time = start_of_week_for_start_time + Days::new(7);

    let start_of_week_for_end_time = {
        // convert to central time before calculating sunday midnight date
        let end_time_central = end_time.with_timezone(&Central);
        let start_work_week =
            end_time_central - Days::new(end_time_central.weekday().num_days_from_sunday() as u64);
        start_work_week
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
    };

    let start_of_week_date_for_start_time = NaiveDate::from_ymd_opt(
        start_of_week_for_start_time.year(),
        start_of_week_for_start_time.month(),
        start_of_week_for_start_time.day(),
    )
    .unwrap();

    let start_of_week_date_for_end_time = NaiveDate::from_ymd_opt(
        start_of_week_for_end_time.year(),
        start_of_week_for_end_time.month(),
        start_of_week_for_end_time.day(),
    )
    .unwrap();

    (
        start_of_week_for_start_time.to_utc(),
        end_of_work_week_for_start_time.to_utc(),
        start_of_week_for_end_time.to_utc(),
        start_of_week_date_for_start_time,
        start_of_week_date_for_end_time,
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
    fn test_summarize_shifts_from_json_file() {
        let path = PathBuf::from_str("./test_datasets/test_dataset_multiple.json").unwrap();
        let summaries = summarize_shifts_from_json_file(&path).unwrap();
        assert_eq!(
            summaries
                .iter()
                .find(|item| item.employee_id == 41488322 && item.start_of_week == *"2021-08-29")
                .unwrap()
                .regular_hours,
            8.5
        );
        assert_eq!(
            summaries
                .iter()
                .find(|item| item.employee_id == 34009849 && item.start_of_week == *"2021-08-22")
                .unwrap()
                .regular_hours,
            12.5
        );
        assert_eq!(
            summaries
                .iter()
                .find(|item| item.employee_id == 38410756 && item.start_of_week == *"2021-08-22")
                .unwrap()
                .regular_hours,
            12.5
        );
    }

    #[test]
    fn test_calculate_overtime_hours() {
        let path = PathBuf::from_str("./test_datasets/test_dataset_overtime_hours.json").unwrap();
        let shifts = read_shifts(&path).unwrap();

        let summaries = summarize_all_employee_hours(shifts);
        let mut summaries: Vec<EmployeeShiftSummary> = summaries.into_values().collect();
        calculate_overtime_hours(&mut summaries);

        let summary_week_06_30_2024_41488322_employee = summaries
            .iter()
            .find(|summary| {
                summary.employee_id == 41488322
                    && summary.start_of_week
                        == NaiveDate::from_ymd_opt(2024, 6, 30).unwrap().to_string()
            })
            .unwrap();
        let summary_week_07_07_2024_4148_employee = summaries
            .iter()
            .find(|summary| {
                summary.employee_id == 4148
                    && summary.start_of_week
                        == NaiveDate::from_ymd_opt(2024, 7, 7).unwrap().to_string()
            })
            .unwrap();
        let summary_week_07_07_2024_4_employee = summaries
            .iter()
            .find(|summary| {
                summary.employee_id == 4
                    && summary.start_of_week
                        == NaiveDate::from_ymd_opt(2024, 7, 7).unwrap().to_string()
            })
            .unwrap();
        let summary_week_07_14_2024_4_employee = summaries
            .iter()
            .find(|summary| {
                summary.employee_id == 4
                    && summary.start_of_week
                        == NaiveDate::from_ymd_opt(2024, 7, 14).unwrap().to_string()
            })
            .unwrap();

        assert_eq!(summaries.len(), 4);
        assert_eq!(summary_week_06_30_2024_41488322_employee.regular_hours, 40.);
        assert_eq!(
            summary_week_06_30_2024_41488322_employee.overtime_hours,
            10.
        );
        assert_eq!(summary_week_07_07_2024_4148_employee.regular_hours, 28.5);
        assert_eq!(summary_week_07_07_2024_4148_employee.overtime_hours, 0.);
        assert_eq!(summary_week_07_07_2024_4_employee.regular_hours, 40.);
        assert_eq!(summary_week_07_07_2024_4_employee.overtime_hours, 3.5);
        assert_eq!(summary_week_07_14_2024_4_employee.regular_hours, 4.);
    }

    #[test]
    fn test_employee_with_overlapping_shifts() {
        let path =
            PathBuf::from_str("./test_datasets/test_dataset_overlapping_shift.json").unwrap();
        let shifts = read_shifts(&path).unwrap();

        let summaries = summarize_all_employee_hours(shifts);
        let summary_first_week = summaries
            .get(&(41488322, NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()))
            .unwrap();
        let summary_second_week = summaries
            .get(&(41488322, NaiveDate::from_ymd_opt(2024, 7, 7).unwrap()))
            .unwrap();

        assert_eq!(summaries.len(), 2);

        assert_eq!(summary_first_week.regular_hours, 12.5);
        assert_eq!(&summary_first_week.invalid_shifts, &[2663141019]);

        assert_eq!(summary_second_week.regular_hours, 0.);
        assert_eq!(&summary_second_week.invalid_shifts, &[2663141013]);
    }

    #[test]
    fn test_summarize_all_employees_shifts_crossing_sunday_midnight() {
        let path =
            PathBuf::from_str("./test_datasets/test_dataset_shift_crosses_sunday_midnight.json")
                .unwrap();
        let shifts = read_shifts(&path).unwrap();

        let summaries = summarize_all_employee_hours(shifts);
        assert_eq!(summaries.len(), 2);
        assert_eq!(
            summaries
                .get(&(41488322, NaiveDate::from_ymd_opt(2024, 6, 30).unwrap()))
                .unwrap()
                .regular_hours,
            17.
        );
        assert_eq!(
            summaries
                .get(&(41488322, NaiveDate::from_ymd_opt(2024, 7, 7).unwrap()))
                .unwrap()
                .regular_hours,
            16.
        );
    }

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
            _,
            _,
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
            _,
            _,
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
        let path = PathBuf::from_str("./test_datasets/test_dataset.json").unwrap();
        let shifts = read_shifts(&path).unwrap();

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
        let path = PathBuf::from_str("./test_datasets/test_dataset_multiple.json").unwrap();
        let shifts = read_shifts(&path).unwrap();

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

        let shift = validate_shift(&shifts[0]).unwrap();
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
}
