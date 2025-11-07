use std::rc::Rc;
use slint::{VecModel};
use crate::MainWindow;
use crate::app::utils::parse_hour;

#[derive(Clone)]
pub struct NightCondition {
    day: String,
    hour: u32,
    condition: String,
    total_clouds: u32,
    is_evening: bool, // true for evening hours (after sunset), false for morning hours (before sunrise)
}

#[derive(Clone)]
pub struct ClearOutsideData {
    pub day0_conditions: Vec<String>,
    pub day0_clouds: Vec<i32>,
    pub day0_hours: Vec<i32>,
    pub day1_conditions: Vec<String>,
    pub day1_clouds: Vec<i32>,
    pub day1_hours: Vec<i32>,
    pub day2_conditions: Vec<String>,
    pub day2_clouds: Vec<i32>,
    pub day2_hours: Vec<i32>,
    pub day3_conditions: Vec<String>,
    pub day3_clouds: Vec<i32>,
    pub day3_hours: Vec<i32>,
    pub day4_conditions: Vec<String>,
    pub day4_clouds: Vec<i32>,
    pub day4_hours: Vec<i32>,
    pub day5_conditions: Vec<String>,
    pub day5_clouds: Vec<i32>,
    pub day5_hours: Vec<i32>,
    pub day6_conditions: Vec<String>,
    pub day6_clouds: Vec<i32>,
    pub day6_hours: Vec<i32>,
}

pub async fn fetch_clearoutside_data(lat: f64, lon: f64) -> Result<ClearOutsideData, Box<dyn std::error::Error>> {
    use clearoutside::ClearOutsideAPI;

    // Format coordinates for ClearOutside (lat.lon with 2 decimals)
    let lat_str = format!("{:.2}", lat);
    let lon_str = format!("{:.2}", lon);

    // Create API instance
    let api = ClearOutsideAPI::new(&lat_str, &lon_str, Some("midnight")).await?;

    // Fetch and parse data
    let forecast = api.pull()?;

    // Process data for night hours display
    let data = process_clearoutside_data(&forecast)?;

    Ok(data)
}

fn process_clearoutside_data(forecast: &clearoutside::ClearOutsideForecast) -> Result<ClearOutsideData, Box<dyn std::error::Error>> {
    let mut night_conditions = Vec::new();

    // Sort days by key (day-0, day-1, etc.)
    let mut sorted_days: Vec<_> = forecast.forecast.iter().collect();
    sorted_days.sort_by_key(|(k, _)| *k);

    for (i, (day_key, day_info)) in sorted_days.iter().enumerate() {
        // Parse sunset time for current day
        let sunset_hour = parse_hour(&day_info.sun.set)?;

        // Get hours after sunset from current day
        let evening_hours: Vec<_> = day_info.hours.iter()
            .filter(|(hour_str, _)| {
                if let Ok(hour) = hour_str.parse::<u32>() {
                    hour > sunset_hour
                } else {
                    false
                }
            })
            .collect();

        // Add evening hours from current day
        for (hour_str, hourly_data) in &evening_hours {
            night_conditions.push(NightCondition {
                day: format!("night-{}", i), // Use night-X instead of day-X
                hour: hour_str.parse().unwrap_or(0),
                condition: hourly_data.conditions.clone(),
                total_clouds: hourly_data.total_clouds.parse().unwrap_or(0),
                is_evening: true, // Evening hours (after sunset)
            });
        }

        // Get hours before sunrise from next day (if available)
        if i + 1 < sorted_days.len() {
            let next_day_info = &sorted_days[i + 1].1;
            let next_sunrise_hour = parse_hour(&next_day_info.sun.rise)?;

            let morning_hours: Vec<_> = next_day_info.hours.iter()
                .filter(|(hour_str, _)| {
                    if let Ok(hour) = hour_str.parse::<u32>() {
                        hour < next_sunrise_hour && hour <= 11 // Only morning hours 0-11
                    } else {
                        false
                    }
                })
                .collect();

            // Add morning hours from next day, but associate with current night
            for (hour_str, hourly_data) in &morning_hours {
                night_conditions.push(NightCondition {
                    day: format!("night-{}", i), // Associate with current night
                    hour: hour_str.parse().unwrap_or(0),
                    condition: hourly_data.conditions.clone(),
                    total_clouds: hourly_data.total_clouds.parse().unwrap_or(0),
                    is_evening: false, // Morning hours (before sunrise)
                });
            }
        }
    }

    // Sort all night conditions by night, then by evening/morning, then by hour
    night_conditions.sort_by(|a, b| {
        let night_cmp = a.day.cmp(&b.day);
        if night_cmp == std::cmp::Ordering::Equal {
            // Within the same night, evening hours come before morning hours
            match (a.is_evening, b.is_evening) {
                (true, false) => std::cmp::Ordering::Less,   // evening before morning
                (false, true) => std::cmp::Ordering::Greater, // morning after evening
                _ => a.hour.cmp(&b.hour), // same type (both evening or both morning), sort by hour
            }
        } else {
            night_cmp
        }
    });

    // Group conditions by day and prepare data
    use std::collections::HashMap;
    let mut conditions_by_day: HashMap<String, Vec<&NightCondition>> = HashMap::new();

    for condition in &night_conditions {
        conditions_by_day.entry(condition.day.clone()).or_insert(Vec::new()).push(condition);
    }

    // Sort days
    let mut sorted_days: Vec<_> = conditions_by_day.keys().collect();
    sorted_days.sort();

    let mut data = ClearOutsideData {
        day0_conditions: Vec::new(),
        day0_clouds: Vec::new(),
        day0_hours: Vec::new(),
        day1_conditions: Vec::new(),
        day1_clouds: Vec::new(),
        day1_hours: Vec::new(),
        day2_conditions: Vec::new(),
        day2_clouds: Vec::new(),
        day2_hours: Vec::new(),
        day3_conditions: Vec::new(),
        day3_clouds: Vec::new(),
        day3_hours: Vec::new(),
        day4_conditions: Vec::new(),
        day4_clouds: Vec::new(),
        day4_hours: Vec::new(),
        day5_conditions: Vec::new(),
        day5_clouds: Vec::new(),
        day5_hours: Vec::new(),
        day6_conditions: Vec::new(),
        day6_clouds: Vec::new(),
        day6_hours: Vec::new(),
    };

    // Set data for each day
    for (day_idx, day_key) in sorted_days.iter().enumerate() {
        if let Some(day_conditions) = conditions_by_day.get(*day_key) {
            // Conditions are already sorted by the global sorting logic
            let conditions_vec: Vec<String> = day_conditions.iter()
                .map(|c| c.condition.clone())
                .collect();
            let clouds_vec: Vec<i32> = day_conditions.iter()
                .map(|c| c.total_clouds as i32)
                .collect();
            let hours_vec: Vec<i32> = day_conditions.iter()
                .map(|c| c.hour as i32)
                .collect();

            match day_idx {
                0 => {
                    data.day0_conditions = conditions_vec;
                    data.day0_clouds = clouds_vec;
                    data.day0_hours = hours_vec;
                }
                1 => {
                    data.day1_conditions = conditions_vec;
                    data.day1_clouds = clouds_vec;
                    data.day1_hours = hours_vec;
                }
                2 => {
                    data.day2_conditions = conditions_vec;
                    data.day2_clouds = clouds_vec;
                    data.day2_hours = hours_vec;
                }
                3 => {
                    data.day3_conditions = conditions_vec;
                    data.day3_clouds = clouds_vec;
                    data.day3_hours = hours_vec;
                }
                4 => {
                    data.day4_conditions = conditions_vec;
                    data.day4_clouds = clouds_vec;
                    data.day4_hours = hours_vec;
                }
                5 => {
                    data.day5_conditions = conditions_vec;
                    data.day5_clouds = clouds_vec;
                    data.day5_hours = hours_vec;
                }
                6 => {
                    data.day6_conditions = conditions_vec;
                    data.day6_clouds = clouds_vec;
                    data.day6_hours = hours_vec;
                }
                _ => {}
            }
        }
    }

    Ok(data)
}

pub fn set_clearoutside_data(main_window: &MainWindow, data: ClearOutsideData) {
    main_window.set_day0_conditions(Rc::new(VecModel::from(data.day0_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day0_clouds(Rc::new(VecModel::from(data.day0_clouds)).into());
    main_window.set_day0_hours(Rc::new(VecModel::from(data.day0_hours)).into());

    main_window.set_day1_conditions(Rc::new(VecModel::from(data.day1_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day1_clouds(Rc::new(VecModel::from(data.day1_clouds)).into());
    main_window.set_day1_hours(Rc::new(VecModel::from(data.day1_hours)).into());

    main_window.set_day2_conditions(Rc::new(VecModel::from(data.day2_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day2_clouds(Rc::new(VecModel::from(data.day2_clouds)).into());
    main_window.set_day2_hours(Rc::new(VecModel::from(data.day2_hours)).into());

    main_window.set_day3_conditions(Rc::new(VecModel::from(data.day3_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day3_clouds(Rc::new(VecModel::from(data.day3_clouds)).into());
    main_window.set_day3_hours(Rc::new(VecModel::from(data.day3_hours)).into());

    main_window.set_day4_conditions(Rc::new(VecModel::from(data.day4_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day4_clouds(Rc::new(VecModel::from(data.day4_clouds)).into());
    main_window.set_day4_hours(Rc::new(VecModel::from(data.day4_hours)).into());

    main_window.set_day5_conditions(Rc::new(VecModel::from(data.day5_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day5_clouds(Rc::new(VecModel::from(data.day5_clouds)).into());
    main_window.set_day5_hours(Rc::new(VecModel::from(data.day5_hours)).into());

    main_window.set_day6_conditions(Rc::new(VecModel::from(data.day6_conditions.into_iter().map(|s| s.into()).collect::<Vec<slint::SharedString>>())).into());
    main_window.set_day6_clouds(Rc::new(VecModel::from(data.day6_clouds)).into());
    main_window.set_day6_hours(Rc::new(VecModel::from(data.day6_hours)).into());
}
